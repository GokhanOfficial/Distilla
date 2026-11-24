mod config;
mod formats;
mod jobs;
mod prompts;
mod routes;
mod services;

use crate::config::AppConfig;
use crate::jobs::JobManager;
use crate::routes::AppState;
use crate::services::media::MediaEncoder;
use crate::services::office::OfficeConverter;
use crate::services::openai::OpenAiService;
use crate::services::pdf::PdfService;
use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use rust_embed::Embed;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Embed)]
#[folder = "web/dist/"]
struct FrontendAssets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("distilla=info,tower_http=info")),
        )
        .init();

    let config = Arc::new(AppConfig::load());

    // Clean stale temp directories from previous runs
    if config.temp_root.exists() {
        if let Err(err) = std::fs::remove_dir_all(&config.temp_root) {
            tracing::warn!("Could not clean stale temp dir: {}", err);
        }
    }
    std::fs::create_dir_all(&config.temp_root)?;

    let jobs = JobManager::new();
    let openai = OpenAiService::new(config.openai.clone());
    let media = MediaEncoder::new(config.media.clone());
    let office = OfficeConverter::new(config.office.clone());
    let pdf = PdfService::new(
        config.pdf.clone(),
        config.app_name.clone(),
        config.app_url.clone(),
        config.temp_root.clone(),
    );

    let state = AppState {
        config: config.clone(),
        jobs: jobs.clone(),
        openai,
        media,
        office,
        pdf,
    };

    let cors = build_cors(&config.cors_origin);

    // Upload route disables default 2MB limit; convert::start_job streams and enforces config.max_upload_bytes()
    let upload_route = Router::new()
        .route("/api/convert/jobs", post(routes::convert::start_job))
        .layer(DefaultBodyLimit::disable());

    let api_routes = Router::new()
        .route("/api/health", get(routes::config::health))
        .route("/api/config", get(routes::config::public_config))
        .route("/api/convert/jobs/{id}", get(routes::convert::get_job))
        .route(
            "/api/convert/jobs/{id}/events",
            get(routes::convert::job_events),
        )
        .route(
            "/api/convert/jobs/{id}/cancel",
            post(routes::convert::cancel_job),
        )
        .route("/api/summarize/jobs", post(routes::summarize::start_job))
        .route("/api/summarize/jobs/{id}", get(routes::summarize::get_job))
        .route(
            "/api/summarize/jobs/{id}/events",
            get(routes::summarize::job_events),
        )
        .route(
            "/api/summarize/jobs/{id}/cancel",
            post(routes::summarize::cancel_job),
        )
        .route("/api/pdf", post(routes::pdf::generate))
        .route("/api/generate-title", post(routes::title::generate))
        .layer(DefaultBodyLimit::max(25 * 1024 * 1024));

    let app = Router::new()
        .merge(upload_route)
        .merge(api_routes)
        .fallback(serve_frontend)
        .with_state(state.clone())
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(
        "Distilla server listening on http://localhost:{} (max upload: {} MB)",
        config.port,
        config.max_upload_size_mb
    );
    if !state.openai.is_configured() {
        tracing::warn!("OPENAI_API_KEY is not set!");
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let temp_root = config.temp_root.clone();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(jobs, temp_root))
        .await?;

    Ok(())
}

fn build_cors(origin: &str) -> CorsLayer {
    let base = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION]);

    if origin == "*" || origin.is_empty() {
        base.allow_origin(Any)
    } else if let Ok(val) = HeaderValue::from_str(origin) {
        base.allow_origin(val)
    } else {
        base.allow_origin(Any)
    }
}

async fn serve_frontend(uri: Uri) -> Response {
    let raw_path = uri.path().trim_start_matches('/');

    // Never fall back to index.html for unknown /api/* paths
    if raw_path.starts_with("api/") || raw_path == "api" {
        return routes::json_error(StatusCode::NOT_FOUND, "Endpoint not found", "errors.notFound");
    }

    let path = if raw_path.is_empty() {
        "index.html"
    } else {
        raw_path
    };

    if let Some(asset) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let cache = if path.starts_with("assets/") {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        };
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime.as_ref().to_string()),
                (header::CACHE_CONTROL, cache.to_string()),
            ],
            asset.data.into_owned(),
        )
            .into_response();
    }

    // SPA fallback to index.html
    if let Some(index) = FrontendAssets::get("index.html") {
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8".to_string()),
                (header::CACHE_CONTROL, "no-cache".to_string()),
            ],
            index.data.into_owned(),
        )
            .into_response();
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Frontend build not embedded. Run `cd web && npm run build` first.",
    )
        .into_response()
}

async fn shutdown_signal(jobs: JobManager, temp_root: std::path::PathBuf) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received — cancelling active jobs and cleaning temp files");
    jobs.cancel_all().await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let _ = std::fs::remove_dir_all(&temp_root);
}
