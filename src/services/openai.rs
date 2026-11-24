use crate::config::{OpenAiConfig, PdfInputMode};
use crate::prompts::{
    clean_markdown_fence, sanitize_filename, DOCUMENT_PROMPT, FILENAME_PROMPT, MEDIA_PROMPT,
    SUMMARIZE_PROMPT,
};
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::future::Future;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum PreparedInput {
    Text { content: String, filename: String },
    Image { bytes: Vec<u8>, mime: String },
    Pdf { bytes: Vec<u8>, filename: String },
    Audio { bytes: Vec<u8>, format: String },
}

#[derive(Clone)]
pub struct OpenAiService {
    client: Client,
    config: OpenAiConfig,
}

impl OpenAiService {
    pub fn new(config: OpenAiConfig) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self { client, config }
    }

    pub fn is_configured(&self) -> bool {
        !self.config.api_key.trim().is_empty()
    }

    pub async fn stream_convert<F, Fut>(
        &self,
        input: PreparedInput,
        model: &str,
        cancel_token: CancellationToken,
        on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = ()>,
    {
        if !self.is_configured() {
            return Err(anyhow!("OPENAI_API_KEY is not configured"));
        }

        let (content_parts, is_media) = match input {
            PreparedInput::Text { content, filename } => (
                vec![json!({
                    "type": "text",
                    "text": format!(
                        "{}\n\n---\nDosya: {}\n---\n{}",
                        DOCUMENT_PROMPT, filename, content
                    )
                })],
                false,
            ),
            PreparedInput::Image { bytes, mime } => {
                let encoded = BASE64.encode(&bytes);
                (
                    vec![
                        json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", mime, encoded)
                            }
                        }),
                        json!({
                            "type": "text",
                            "text": DOCUMENT_PROMPT
                        }),
                    ],
                    false,
                )
            }
            PreparedInput::Pdf { bytes, filename } => {
                let encoded = BASE64.encode(&bytes);
                let data_uri = format!("data:application/pdf;base64,{}", encoded);
                let pdf_part = match self.config.pdf_input_mode {
                    PdfInputMode::File => json!({
                        "type": "file",
                        "file": {
                            "filename": filename,
                            "file_data": data_uri
                        }
                    }),
                    PdfInputMode::ImageUrl => json!({
                        "type": "image_url",
                        "image_url": {
                            "url": data_uri
                        }
                    }),
                };
                (
                    vec![
                        pdf_part,
                        json!({
                            "type": "text",
                            "text": DOCUMENT_PROMPT
                        }),
                    ],
                    false,
                )
            }
            PreparedInput::Audio { bytes, format } => {
                let encoded = BASE64.encode(&bytes);
                (
                    vec![
                        json!({
                            "type": "input_audio",
                            "input_audio": {
                                "data": encoded,
                                "format": format
                            }
                        }),
                        json!({
                            "type": "text",
                            "text": MEDIA_PROMPT
                        }),
                    ],
                    true,
                )
            }
        };

        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": content_parts
                }
            ],
            "stream": true,
            "temperature": if is_media { 0.15 } else { 0.1 },
            "top_p": 0.95
        });

        let raw = self
            .stream_chat_completion(payload, cancel_token, on_chunk)
            .await?;
        Ok(clean_markdown_fence(&raw))
    }

    pub async fn stream_summarize<F, Fut>(
        &self,
        markdown: &str,
        model: &str,
        cancel_token: CancellationToken,
        on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = ()>,
    {
        if !self.is_configured() {
            return Err(anyhow!("OPENAI_API_KEY is not configured"));
        }

        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": format!("{}\n\n{}", SUMMARIZE_PROMPT, markdown)
                }
            ],
            "stream": true,
            "temperature": 0.3,
            "top_p": 0.95
        });

        let raw = self
            .stream_chat_completion(payload, cancel_token, on_chunk)
            .await?;
        Ok(clean_markdown_fence(&raw))
    }

    pub async fn generate_filename(
        &self,
        markdown: &str,
        model_override: Option<&str>,
    ) -> Result<String> {
        if !self.is_configured() {
            return Err(anyhow!("OPENAI_API_KEY is not configured"));
        }

        let model = self
            .config
            .filename_model
            .as_deref()
            .or(model_override)
            .unwrap_or(&self.config.default_model);

        let sample = sample_markdown(markdown, 4000);
        let prompt = format!("{}\n{}\n\nDosya adı:", FILENAME_PROMPT, sample);

        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 100,
            "top_p": 0.8
        });

        let url = format!("{}/chat/completions", self.config.base_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .timeout(Duration::from_secs(30))
            .json(&payload)
            .send()
            .await
            .context("Failed to reach AI API for filename generation")?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .context("Invalid JSON response from AI API")?;

        if !status.is_success() {
            let msg = extract_api_error(&body).unwrap_or_else(|| format!("HTTP {}", status));
            return Err(anyhow!("AI API error: {}", msg));
        }

        let text = body
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Empty filename response from AI"))?;

        Ok(sanitize_filename(text))
    }

    async fn stream_chat_completion<F, Fut>(
        &self,
        payload: Value,
        cancel_token: CancellationToken,
        mut on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = ()>,
    {
        let url = format!("{}/chat/completions", self.config.base_url);

        let send_fut = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .header("Accept", "text/event-stream")
            .json(&payload)
            .send();

        let response = tokio::select! {
            _ = cancel_token.cancelled() => return Err(anyhow!("Cancelled")),
            res = send_fut => res.context("Failed to connect to AI API")?,
        };

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            let detail = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| extract_api_error(&v))
                .unwrap_or_else(|| {
                    if text.is_empty() {
                        format!("HTTP {}", status)
                    } else {
                        text.chars().take(300).collect()
                    }
                });
            return Err(anyhow!("AI API error ({}): {}", status.as_u16(), detail));
        }

        let mut stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::with_capacity(4096);
        let mut full_text = String::new();

        loop {
            let next_chunk = tokio::select! {
                _ = cancel_token.cancelled() => return Err(anyhow!("Cancelled")),
                item = stream.next() => item,
            };

            let Some(chunk_result) = next_chunk else {
                break;
            };
            let bytes = chunk_result.context("AI stream read error")?;
            buffer.extend_from_slice(&bytes);

            while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=newline_pos).collect();
                let line = String::from_utf8_lossy(&line_bytes);
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with(':') {
                    continue;
                }
                if let Some(data) = trimmed.strip_prefix("data:") {
                    let data = data.trim();
                    if data == "[DONE]" {
                        return Ok(full_text);
                    }
                    if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                        if let Some(err) = extract_api_error(&parsed) {
                            return Err(anyhow!("AI stream error: {}", err));
                        }
                        if let Some(delta) = parsed
                            .pointer("/choices/0/delta/content")
                            .and_then(|v| v.as_str())
                        {
                            if !delta.is_empty() {
                                full_text.push_str(delta);
                                on_chunk(delta.to_string()).await;
                            }
                        }
                    }
                }
            }
        }

        // Handle non-SSE JSON response if proxy ignored stream=true
        if full_text.is_empty() && !buffer.is_empty() {
            if let Ok(parsed) = serde_json::from_slice::<Value>(&buffer) {
                if let Some(err) = extract_api_error(&parsed) {
                    return Err(anyhow!("AI API error: {}", err));
                }
                if let Some(content) = parsed
                    .pointer("/choices/0/message/content")
                    .and_then(|v| v.as_str())
                {
                    full_text.push_str(content);
                    on_chunk(content.to_string()).await;
                }
            }
        }

        Ok(full_text)
    }
}

fn extract_api_error(val: &Value) -> Option<String> {
    val.pointer("/error/message")
        .and_then(|v| v.as_str())
        .or_else(|| val.pointer("/error").and_then(|v| v.as_str()))
        .or_else(|| val.pointer("/detail").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

fn sample_markdown(markdown: &str, edge_chars: usize) -> String {
    let total = markdown.chars().count();
    if total <= edge_chars * 2 {
        return markdown.to_string();
    }
    let first: String = markdown.chars().take(edge_chars).collect();
    let last: String = markdown.chars().skip(total - edge_chars).collect();
    format!("{}\n\n[...]\n\n{}", first, last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_unicode_safely() {
        let s = "ğ".repeat(100);
        let sampled = sample_markdown(&s, 20);
        assert!(sampled.contains("[...]"));
        assert!(sampled.chars().count() < 60);
    }
}
