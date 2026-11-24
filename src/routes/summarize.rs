use crate::jobs::JobKind;
use crate::routes::{json_error, stream_job_events, AppState};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct SummarizeRequest {
    pub markdown: String,
    pub model: Option<String>,
}

pub async fn start_job(
    State(state): State<AppState>,
    Json(payload): Json<SummarizeRequest>,
) -> Response {
    let text = payload.markdown.trim().to_string();
    if text.is_empty() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Text cannot be empty",
            "errors.emptyText",
        );
    }

    let model = payload
        .model
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| state.config.openai.default_model.clone());

    let (job_id, cancel_token) = state.jobs.create_job(JobKind::Summarize, None).await;
    tracing::info!(job_id = %job_id, model = %model, "Started summarization job");

    let task_state = state.clone();
    let task_job_id = job_id.clone();

    tokio::spawn(async move {
        task_state
            .jobs
            .set_phase(
                &task_job_id,
                "ai-conversion",
                "progress.summarizing",
                15,
                None,
            )
            .await;

        let jobs_ref = task_state.jobs.clone();
        let jid_ref = task_job_id.clone();

        match task_state
            .openai
            .stream_summarize(&text, &model, cancel_token.clone(), move |delta| {
                let jobs = jobs_ref.clone();
                let jid = jid_ref.clone();
                async move {
                    jobs.append_chunk(&jid, &delta).await;
                }
            })
            .await
        {
            Ok(summary) => {
                if cancel_token.is_cancelled() {
                    task_state.jobs.cancel(&task_job_id).await;
                    return;
                }

                let title = match task_state
                    .openai
                    .generate_filename(&summary, Some(&model))
                    .await
                {
                    Ok(t) => Some(t),
                    Err(err) => {
                        tracing::warn!("Summary filename generation failed: {}", err);
                        Some("summary".to_string())
                    }
                };

                task_state
                    .jobs
                    .complete(&task_job_id, summary, title)
                    .await;
            }
            Err(err) => {
                if cancel_token.is_cancelled() {
                    task_state.jobs.cancel(&task_job_id).await;
                } else {
                    tracing::error!(job_id = %task_job_id, "Summarization failed: {}", err);
                    task_state.jobs.fail(&task_job_id, err.to_string()).await;
                }
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "success": true,
            "jobId": job_id,
            "status": "pending"
        })),
    )
        .into_response()
}

pub async fn get_job(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.jobs.get_snapshot(&id).await {
        Some(snap) => Json(snap).into_response(),
        None => json_error(StatusCode::NOT_FOUND, "Job not found", "errors.jobNotFound"),
    }
}

pub async fn job_events(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    stream_job_events(state, id).await
}

pub async fn cancel_job(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.jobs.cancel(&id).await {
        Some(snap) => {
            tracing::info!(job_id = %id, "Summarization job cancelled");
            Json(json!({ "success": true, "job": snap })).into_response()
        }
        None => json_error(StatusCode::NOT_FOUND, "Job not found", "errors.jobNotFound"),
    }
}
