use crate::formats::{detect_format, FormatKind, FormatSpec};
use crate::jobs::JobKind;
use crate::routes::{json_error, stream_job_events, AppState};
use crate::services::openai::PreparedInput;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_dir_all(&self.path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    "Failed to remove temp directory {}: {}",
                    self.path.display(),
                    err
                );
            }
        }
    }
}

/// POST /api/convert/jobs - Single-step streaming multipart upload to isolated temp dir + async job start
pub async fn start_job(State(state): State<AppState>, mut multipart: Multipart) -> Response {
    let job_dir = state.config.temp_root.join(format!("job-{}", Uuid::new_v4()));
    if let Err(err) = tokio::fs::create_dir_all(&job_dir).await {
        tracing::error!("Failed to create job temp dir: {}", err);
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to prepare upload storage",
            "errors.serverError",
        );
    }

    let mut guard = Some(TempDirGuard::new(job_dir.clone()));
    let mut selected_model: Option<String> = state.config.openai.default_convert_model.clone();
    let mut saved_file: Option<(PathBuf, String, &'static FormatSpec)> = None;
    let max_bytes = state.config.max_upload_bytes();

    while let Ok(Some(mut field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or_default().to_string();

        if field_name == "model" {
            if let Ok(val) = field.text().await {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    selected_model = Some(trimmed.to_string());
                }
            }
            continue;
        }

        if field_name == "file" && saved_file.is_none() {
            let orig_name = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "upload".to_string());
            let content_type = field.content_type().unwrap_or_default().to_string();

            let Some(spec) = detect_format(&orig_name, &content_type) else {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    format!("Unsupported file format: {} ({})", orig_name, content_type),
                    "errors.unsupportedFormat",
                );
            };

            let target_path = job_dir.join(format!("input{}", spec.primary_ext));
            let mut file = match tokio::fs::File::create(&target_path).await {
                Ok(f) => f,
                Err(err) => {
                    tracing::error!("Failed to create temp upload file: {}", err);
                    return json_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to store uploaded file",
                        "errors.uploadFailed",
                    );
                }
            };

            let mut total_written: u64 = 0;
            loop {
                match field.chunk().await {
                    Ok(Some(chunk)) => {
                        total_written += chunk.len() as u64;
                        if total_written > max_bytes {
                            drop(file);
                            return json_error(
                                StatusCode::PAYLOAD_TOO_LARGE,
                                format!(
                                    "File exceeds maximum upload size of {} MB",
                                    state.config.max_upload_size_mb
                                ),
                                "errors.fileTooLarge",
                            );
                        }
                        if let Err(err) = file.write_all(&chunk).await {
                            tracing::error!("Failed writing upload chunk: {}", err);
                            return json_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed while saving uploaded file",
                                "errors.uploadFailed",
                            );
                        }
                    }
                    Ok(None) => break,
                    Err(err) => {
                        tracing::warn!("Multipart upload interrupted: {}", err);
                        return json_error(
                            StatusCode::BAD_REQUEST,
                            format!("Upload interrupted: {}", err),
                            "errors.uploadFailed",
                        );
                    }
                }
            }

            if let Err(err) = file.flush().await {
                tracing::error!("Failed flushing uploaded file: {}", err);
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to finalize uploaded file",
                    "errors.uploadFailed",
                );
            }

            if total_written == 0 {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "Uploaded file is empty",
                    "errors.emptyFile",
                );
            }

            saved_file = Some((target_path, orig_name, spec));
        }
    }

    let Some((input_path, filename, spec)) = saved_file else {
        return json_error(
            StatusCode::BAD_REQUEST,
            "No file uploaded",
            "errors.noFile",
        );
    };

    let Some(selected_model) = selected_model.filter(|m| !m.trim().is_empty()) else {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "No converter AI model configured (set CONVERT_MODELS in .env)",
            "errors.noModelConfigured",
        );
    };

    let (job_id, cancel_token) = state
        .jobs
        .create_job(JobKind::Convert, Some(filename.clone()))
        .await;

    tracing::info!(
        job_id = %job_id,
        file = %filename,
        format = spec.name,
        model = %selected_model,
        "Started single-file conversion job"
    );

    // Move ownership of the temp dir cleanup guard into the background task
    let task_guard = guard.take().expect("guard present");
    let task_state = state.clone();
    let task_job_id = job_id.clone();

    tokio::spawn(async move {
        let _cleanup_guard = task_guard;
        run_conversion_job(
            task_state,
            task_job_id,
            input_path,
            filename,
            spec,
            selected_model,
            job_dir,
            cancel_token,
        )
        .await;
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

#[allow(clippy::too_many_arguments)]
async fn run_conversion_job(
    state: AppState,
    job_id: String,
    input_path: PathBuf,
    filename: String,
    spec: &'static FormatSpec,
    model: String,
    job_dir: PathBuf,
    cancel_token: CancellationToken,
) {
    state
        .jobs
        .set_phase(&job_id, "preparing", "progress.preparing", 10, None)
        .await;

    let prepared = match prepare_input(
        &state,
        &job_id,
        &input_path,
        &filename,
        spec,
        &job_dir,
        cancel_token.clone(),
    )
    .await
    {
        Ok(inp) => inp,
        Err(err) => {
            if cancel_token.is_cancelled() {
                state.jobs.cancel(&job_id).await;
            } else {
                tracing::error!(job_id = %job_id, "Preparation failed: {}", err);
                state.jobs.fail(&job_id, err.to_string()).await;
            }
            return;
        }
    };

    if cancel_token.is_cancelled() {
        state.jobs.cancel(&job_id).await;
        return;
    }

    state
        .jobs
        .set_phase(&job_id, "ai-conversion", "progress.aiConverting", 65, None)
        .await;

    let jobs_for_chunk = state.jobs.clone();
    let job_id_for_chunk = job_id.clone();

    match state
        .openai
        .stream_convert(prepared, &model, cancel_token.clone(), move |delta| {
            let jobs = jobs_for_chunk.clone();
            let jid = job_id_for_chunk.clone();
            async move {
                jobs.append_chunk(&jid, &delta).await;
            }
        })
        .await
    {
        Ok(final_md) => {
            if cancel_token.is_cancelled() {
                state.jobs.cancel(&job_id).await;
            } else {
                tracing::info!(job_id = %job_id, chars = final_md.len(), "Conversion completed");
                state.jobs.complete(&job_id, final_md, None).await;
            }
        }
        Err(err) => {
            if cancel_token.is_cancelled() {
                state.jobs.cancel(&job_id).await;
            } else {
                tracing::error!(job_id = %job_id, "AI conversion failed: {}", err);
                state.jobs.fail(&job_id, err.to_string()).await;
            }
        }
    }
}

async fn prepare_input(
    state: &AppState,
    job_id: &str,
    input_path: &PathBuf,
    filename: &str,
    spec: &'static FormatSpec,
    job_dir: &PathBuf,
    cancel_token: CancellationToken,
) -> anyhow::Result<PreparedInput> {
    match spec.kind {
        FormatKind::Text => {
            let bytes = tokio::fs::read(input_path).await?;
            let content = String::from_utf8_lossy(&bytes).into_owned();
            Ok(PreparedInput::Text {
                content,
                filename: filename.to_string(),
            })
        }
        FormatKind::Image => {
            let bytes = tokio::fs::read(input_path).await?;
            Ok(PreparedInput::Image {
                bytes,
                mime: spec.canonical_mime.to_string(),
            })
        }
        FormatKind::Pdf => {
            let bytes = tokio::fs::read(input_path).await?;
            Ok(PreparedInput::Pdf {
                bytes,
                filename: filename.to_string(),
            })
        }
        FormatKind::Office { google_mime } => {
            state
                .jobs
                .set_phase(job_id, "office-conversion", "progress.officeConverting", 25, None)
                .await;
            let pdf_bytes = state
                .office
                .convert_to_pdf(
                    input_path,
                    spec.canonical_mime,
                    google_mime,
                    job_dir,
                    cancel_token,
                )
                .await?;
            Ok(PreparedInput::Pdf {
                bytes: pdf_bytes,
                filename: format!("{}.pdf", filename),
            })
        }
        FormatKind::DirectAudio { .. } | FormatKind::EncodeAudio | FormatKind::EncodeVideo => {
            let jobs_ref = state.jobs.clone();
            let jid = job_id.to_string();
            let prepared_audio = state
                .media
                .prepare_audio(input_path, spec, job_dir, cancel_token, move |mp| {
                    let jobs = jobs_ref.clone();
                    let jid = jid.clone();
                    async move {
                        let overall = 15 + ((mp.percent as u16 * 45) / 100) as u8;
                        jobs.set_phase(
                            &jid,
                            "media-encoding",
                            "progress.mediaEncoding",
                            overall.min(60),
                            Some(mp),
                        )
                        .await;
                    }
                })
                .await?;

            tracing::info!(
                job_id = %job_id,
                format = %prepared_audio.format,
                size_bytes = prepared_audio.size_bytes,
                bitrate_kbps = ?prepared_audio.bitrate_kbps,
                "Audio prepared for AI"
            );
            let bytes = tokio::fs::read(&prepared_audio.path).await?;
            Ok(PreparedInput::Audio {
                bytes,
                format: prepared_audio.format,
            })
        }
    }
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
            tracing::info!(job_id = %id, "Conversion job cancelled");
            Json(json!({ "success": true, "job": snap })).into_response()
        }
        None => json_error(StatusCode::NOT_FOUND, "Job not found", "errors.jobNotFound"),
    }
}
