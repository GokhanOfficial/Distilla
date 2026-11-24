pub mod config;
pub mod convert;
pub mod pdf;
pub mod summarize;
pub mod title;

use crate::config::AppConfig;
use crate::jobs::{JobEvent, JobManager};
use crate::services::media::MediaEncoder;
use crate::services::office::OfficeConverter;
use crate::services::openai::OpenAiService;
use crate::services::pdf::PdfService;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::stream::Stream;
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub jobs: JobManager,
    pub openai: OpenAiService,
    pub media: MediaEncoder,
    pub office: OfficeConverter,
    pub pdf: PdfService,
}

pub fn json_error(status: StatusCode, error: impl Into<String>, error_key: &str) -> Response {
    (
        status,
        Json(json!({
            "error": error.into(),
            "errorKey": error_key
        })),
    )
        .into_response()
}

/// Shared SSE stream builder for both conversion and summarization jobs.
pub async fn stream_job_events(state: AppState, job_id: String) -> Response {
    let Some((initial_snapshot, rx)) = state.jobs.subscribe(&job_id).await else {
        return json_error(StatusCode::NOT_FOUND, "Job not found", "errors.jobNotFound");
    };

    let jobs_mgr = state.jobs.clone();
    let stream = async_stream(job_id, initial_snapshot, rx, jobs_mgr);

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("ping"),
        )
        .into_response()
}

fn async_stream(
    job_id: String,
    initial_snapshot: crate::jobs::JobSnapshot,
    mut rx: broadcast::Receiver<JobEvent>,
    jobs_mgr: JobManager,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let (tx, rx_out) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);

    tokio::spawn(async move {
        let already_done = initial_snapshot.status.is_terminal();
        if let Ok(payload) = serde_json::to_string(&initial_snapshot) {
            if tx
                .send(Ok(Event::default().event("snapshot").data(payload)))
                .await
                .is_err()
            {
                return;
            }
        }
        if already_done {
            return;
        }

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let is_done = matches!(event, JobEvent::Done { .. });
                    let event_name = match event {
                        JobEvent::State { .. } => "state",
                        JobEvent::Chunk { .. } => "chunk",
                        JobEvent::Done { .. } => "done",
                    };
                    if let Ok(payload) = serde_json::to_string(&event) {
                        if tx
                            .send(Ok(Event::default().event(event_name).data(payload)))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    if is_done {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    if let Some(snap) = jobs_mgr.get_snapshot(&job_id).await {
                        let done = snap.status.is_terminal();
                        if let Ok(payload) = serde_json::to_string(&snap) {
                            if tx
                                .send(Ok(Event::default().event("snapshot").data(payload)))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        if done {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    tokio_stream::wrappers::ReceiverStream::new(rx_out)
}
