use crate::prompts::sanitize_filename;
use crate::routes::{json_error, AppState};
use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PdfRequest {
    pub markdown: String,
    pub filename: Option<String>,
}

pub async fn generate(
    State(state): State<AppState>,
    Json(payload): Json<PdfRequest>,
) -> Response {
    if payload.markdown.trim().is_empty() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Markdown content is required",
            "errors.noContent",
        );
    }

    let raw_name = payload.filename.as_deref().unwrap_or("document");
    let stem = raw_name.strip_suffix(".pdf").unwrap_or(raw_name);
    let safe_stem = sanitize_filename(stem);
    let pdf_filename = format!("{}.pdf", safe_stem);

    match state.pdf.generate_pdf(&payload.markdown).await {
        Ok(pdf_bytes) => {
            let ascii_fallback: String = pdf_filename
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
                .collect();
            let encoded = percent_encode_utf8(&pdf_filename);
            let disposition = format!(
                "attachment; filename=\"{}\"; filename*=UTF-8''{}",
                ascii_fallback, encoded
            );

            let mut resp = (StatusCode::OK, pdf_bytes).into_response();
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/pdf"),
            );
            if let Ok(val) = HeaderValue::from_str(&disposition) {
                resp.headers_mut().insert(header::CONTENT_DISPOSITION, val);
            }
            resp
        }
        Err(err) => {
            tracing::error!("PDF generation error: {}", err);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                err.to_string(),
                "errors.pdfFailed",
            )
        }
    }
}

fn percent_encode_utf8(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for &b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}
