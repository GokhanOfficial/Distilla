use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfInputMode {
    ImageUrl,
    File,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub base_url: String,
    pub convert_models: Vec<ModelOption>,
    pub default_convert_model: Option<String>,
    pub summarize_models: Vec<ModelOption>,
    pub default_summarize_model: Option<String>,
    pub filename_model: Option<String>,
    pub pdf_input_mode: PdfInputMode,
}

#[derive(Debug, Clone)]
pub struct MediaConfig {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub max_output_size_mb: u64,
    pub target_output_size_mb: u64,
    pub min_audio_bitrate_kbps: u32,
    pub max_audio_bitrate_kbps: u32,
    pub output_sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct OfficeConfig {
    pub libreoffice_path: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_token_json: Option<String>,
}

impl OfficeConfig {
    pub fn has_google_drive(&self) -> bool {
        self.google_client_id
            .as_deref()
            .is_some_and(|s| !s.is_empty())
            && self
                .google_client_secret
                .as_deref()
                .is_some_and(|s| !s.is_empty())
            && self
                .google_token_json
                .as_deref()
                .is_some_and(|s| !s.is_empty())
    }
}

#[derive(Debug, Clone)]
pub struct PdfConfig {
    pub chromium_path: Option<String>,
    pub max_concurrency: usize,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub app_name: String,
    pub app_url: Option<String>,
    pub cors_origin: String,
    pub max_upload_size_mb: u64,
    pub openai: OpenAiConfig,
    pub media: MediaConfig,
    pub office: OfficeConfig,
    pub pdf: PdfConfig,
    pub temp_root: PathBuf,
}

impl AppConfig {
    pub fn load() -> Self {
        load_dotenv_lenient(Path::new(".env"));

        let port = env_u16("PORT", 3000);
        let app_name = env_string("APP_NAME").unwrap_or_else(|| "Distilla".to_string());
        let app_url = env_string("APP_URL");
        let cors_origin = env_string("CORS_ORIGIN").unwrap_or_else(|| "*".to_string());

        let max_upload_size_mb = env_u64_opt("MAX_UPLOAD_SIZE_MB")
            .or_else(|| env_u64_opt("MAX_FILE_SIZE"))
            .unwrap_or(1024)
            .max(1);

        let api_key = env_string("OPENAI_API_KEY").unwrap_or_default();
        let base_url = env_string("OPENAI_BASE_URL")
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
            .trim_end_matches('/')
            .to_string();

        let convert_models = parse_models(env_string("CONVERT_MODELS").as_deref());
        let default_convert_model = env_string("CONVERT_DEFAULT_MODEL")
            .or_else(|| convert_models.first().map(|m| m.id.clone()));

        let summarize_models = parse_models(env_string("SUMMARIZE_MODELS").as_deref());
        let default_summarize_model = env_string("SUMMARIZE_DEFAULT_MODEL")
            .or_else(|| summarize_models.first().map(|m| m.id.clone()));

        let filename_model = env_string("FILENAME_MODEL")
            .or_else(|| env_string("OPENAI_FILENAME_MODEL"))
            .or_else(|| env_string("AI_FILENAME_MODEL"));

        let pdf_input_mode = match env_string("PDF_INPUT_MODE")
            .unwrap_or_else(|| "image_url".to_string())
            .to_lowercase()
            .as_str()
        {
            "file" => PdfInputMode::File,
            _ => PdfInputMode::ImageUrl,
        };

        let media = MediaConfig {
            ffmpeg_path: env_string("FFMPEG_PATH").unwrap_or_else(|| "ffmpeg".to_string()),
            ffprobe_path: env_string("FFPROBE_PATH").unwrap_or_else(|| "ffprobe".to_string()),
            max_output_size_mb: env_u64("MEDIA_MAX_OUTPUT_SIZE_MB", 100).max(1),
            target_output_size_mb: env_u64("MEDIA_TARGET_OUTPUT_SIZE_MB", 95).max(1),
            min_audio_bitrate_kbps: env_u32("MEDIA_MIN_AUDIO_BITRATE_KBPS", 32).max(16),
            max_audio_bitrate_kbps: env_u32("MEDIA_MAX_AUDIO_BITRATE_KBPS", 320).max(32),
            output_sample_rate: env_u32("MEDIA_OUTPUT_SAMPLE_RATE", 44100),
        };

        let office = OfficeConfig {
            libreoffice_path: env_string("LIBREOFFICE_PATH")
                .unwrap_or_else(|| "soffice".to_string()),
            google_client_id: env_string("GOOGLE_CLIENT_ID"),
            google_client_secret: env_string("GOOGLE_CLIENT_SECRET"),
            google_token_json: env_string("GOOGLE_TOKEN"),
        };

        let chromium_path = env_string("CHROMIUM_PATH")
            .or_else(|| env_string("PUPPETEER_EXECUTABLE_PATH"))
            .or_else(detect_chromium_binary);

        let pdf = PdfConfig {
            chromium_path,
            max_concurrency: (env_u64("PDF_MAX_CONCURRENCY", 2) as usize).max(1),
        };

        let temp_root = std::env::temp_dir().join("distilla");

        Self {
            port,
            app_name,
            app_url,
            cors_origin,
            max_upload_size_mb,
            openai: OpenAiConfig {
                api_key,
                base_url,
                convert_models,
                default_convert_model,
                summarize_models,
                default_summarize_model,
                filename_model,
                pdf_input_mode,
            },
            media,
            office,
            pdf,
            temp_root,
        }
    }

    pub fn max_upload_bytes(&self) -> u64 {
        self.max_upload_size_mb * 1024 * 1024
    }
}

/// Lenient `.env` loader that preserves unquoted JSON values (like `GOOGLE_TOKEN={...}`)
/// and strips trailing `# comments` from plain values without overwriting existing env vars.
fn load_dotenv_lenient(path: &Path) {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        if key.is_empty() || std::env::var_os(key).is_some() {
            continue;
        }
        let cleaned = clean_env_value(v.trim());
        std::env::set_var(key, cleaned);
    }
}

fn clean_env_value(raw: &str) -> String {
    if (raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2)
        || (raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2)
    {
        return raw[1..raw.len() - 1].to_string();
    }
    if raw.starts_with('{') && raw.ends_with('}') {
        return raw.to_string();
    }
    // Strip inline comment starting with whitespace + '#'
    if let Some(idx) = raw.find(" #") {
        return raw[..idx].trim().to_string();
    }
    raw.to_string()
}

fn format_model_label(id: &str) -> String {
    let label = id
        .split(['-', '_', '/', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_uppercase().to_string();
                    word.push_str(&chars.as_str().to_lowercase());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    if label.is_empty() {
        id.to_string()
    } else {
        label
    }
}

fn parse_models(raw: Option<&str>) -> Vec<ModelOption> {
    let Some(s) = raw else {
        return Vec::new();
    };

    s.split(',')
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(ModelOption {
                id: trimmed.to_string(),
                label: format_model_label(trimmed),
            })
        })
        .collect()
}

fn detect_chromium_binary() -> Option<String> {
    let candidates = [
        "chromium",
        "chromium-browser",
        "google-chrome-stable",
        "google-chrome",
    ];
    for bin in candidates {
        if find_in_path(bin) {
            return Some(bin.to_string());
        }
    }
    None
}

pub fn find_in_path(binary: &str) -> bool {
    let p = Path::new(binary);
    if p.components().count() > 1 {
        return p.is_file();
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary);
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

fn env_string(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn env_u16(key: &str, fallback: u16) -> u16 {
    env_string(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(fallback)
}

fn env_u32(key: &str, fallback: u32) -> u32 {
    env_string(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(fallback)
}

fn env_u64(key: &str, fallback: u64) -> u64 {
    env_string(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(fallback)
}

fn env_u64_opt(key: &str) -> Option<u64> {
    env_string(key).and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_inline_comments_and_json() {
        assert_eq!(
            clean_env_value("https://api.example.com/v1  # Optional: custom"),
            "https://api.example.com/v1"
        );
        assert_eq!(
            clean_env_value(r#"{"access_token":"abc","refresh_token":"xyz"}"#),
            r#"{"access_token":"abc","refresh_token":"xyz"}"#
        );
        assert_eq!(clean_env_value("\"hello world\""), "hello world");
    }

    #[test]
    fn parses_model_list_and_generates_labels() {
        assert_eq!(format_model_label("gemini-2.5-flash"), "Gemini 2.5 Flash");
        assert_eq!(format_model_label("gemini-4-hydrogen"), "Gemini 4 Hydrogen");
        assert_eq!(format_model_label("gpt-4o"), "Gpt 4o");
        assert_eq!(format_model_label("gpt-6.2-sol"), "Gpt 6.2 Sol");

        let models = parse_models(Some("gemini-2.5-flash, gemini-4-hydrogen, gpt-4o, gpt-6.2-sol"));
        assert_eq!(models.len(), 4);
        assert_eq!(models[0].id, "gemini-2.5-flash");
        assert_eq!(models[0].label, "Gemini 2.5 Flash");
        assert_eq!(models[1].label, "Gemini 4 Hydrogen");
        assert_eq!(models[2].label, "Gpt 4o");
        assert!(parse_models(None).is_empty());
        assert!(parse_models(Some("  ,  ")).is_empty());
    }
}
