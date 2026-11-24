use crate::formats;
use crate::routes::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "app": state.config.app_name,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "capabilities": {
            "ai": state.openai.is_configured(),
            "ffmpeg": state.media.is_ffmpeg_available(),
            "ffprobe": state.media.is_ffprobe_available(),
            "libreoffice": state.office.is_libreoffice_available(),
            "office": state.office.is_available(),
            "pdf": state.pdf.is_available()
        }
    }))
}

pub async fn public_config(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "appName": state.config.app_name,
        "convertModels": state.config.openai.convert_models,
        "defaultConvertModel": state.config.openai.default_convert_model,
        "summarizeModels": state.config.openai.summarize_models,
        "defaultSummarizeModel": state.config.openai.default_summarize_model,
        "maxUploadSizeMb": state.config.max_upload_size_mb,
        "mediaMaxOutputSizeMb": state.config.media.max_output_size_mb,
        "supportedExtensions": formats::all_extensions(),
        "formats": formats::public_formats(),
        "capabilities": {
            "ai": state.openai.is_configured(),
            "ffmpeg": state.media.is_ffmpeg_available(),
            "office": state.office.is_available(),
            "pdf": state.pdf.is_available()
        }
    }))
}
