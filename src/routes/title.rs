use crate::routes::{json_error, AppState};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct TitleRequest {
    pub markdown: String,
    pub model: Option<String>,
}

pub async fn generate(
    State(state): State<AppState>,
    Json(payload): Json<TitleRequest>,
) -> Response {
    if payload.markdown.trim().is_empty() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Markdown content is required",
            "errors.noContent",
        );
    }

    match state
        .openai
        .generate_filename(&payload.markdown, payload.model.as_deref())
        .await
    {
        Ok(title) => Json(json!({
            "success": true,
            "title": title,
            "fallback": false
        }))
        .into_response(),
        Err(err) => {
            tracing::warn!("Filename generation failed, using fallback: {}", err);
            let short_id = &Uuid::new_v4().to_string()[..8];
            Json(json!({
                "success": true,
                "title": format!("document-{}", short_id),
                "fallback": true
            }))
            .into_response()
        }
    }
}
