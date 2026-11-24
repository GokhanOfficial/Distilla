use crate::config::{find_in_path, OfficeConfig};
use crate::services::media::kill_child_group;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct OfficeConverter {
    config: OfficeConfig,
    client: Client,
}

#[derive(Deserialize)]
struct GoogleToken {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expiry_date: Option<i64>,
}

#[derive(Deserialize)]
struct TokenRefreshResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct DriveUploadResponse {
    id: String,
}

impl OfficeConverter {
    pub fn new(config: OfficeConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build office http client");
        Self { config, client }
    }

    pub fn is_libreoffice_available(&self) -> bool {
        find_in_path(&self.config.libreoffice_path)
    }

    pub fn is_available(&self) -> bool {
        self.is_libreoffice_available() || self.config.has_google_drive()
    }

    /// Converts an Office document (DOCX/DOC/PPTX/PPT/XLSX/XLS) to PDF bytes.
    /// Prefers local headless LibreOffice; falls back to Google Drive API if configured.
    pub async fn convert_to_pdf(
        &self,
        input_path: &Path,
        source_mime: &str,
        google_mime: &str,
        temp_dir: &Path,
        cancel_token: CancellationToken,
    ) -> Result<Vec<u8>> {
        if self.is_libreoffice_available() {
            return self
                .convert_with_libreoffice(input_path, temp_dir, cancel_token)
                .await;
        }

        if self.config.has_google_drive() {
            return self
                .convert_with_google_drive(input_path, source_mime, google_mime, cancel_token)
                .await;
        }

        Err(anyhow!(
            "Office conversion requires LibreOffice (soffice) installed on the server, or Google Drive OAuth credentials"
        ))
    }

    async fn convert_with_libreoffice(
        &self,
        input_path: &Path,
        temp_dir: &Path,
        cancel_token: CancellationToken,
    ) -> Result<Vec<u8>> {
        let profile_dir = temp_dir.join("lo-profile");
        tokio::fs::create_dir_all(&profile_dir).await?;
        let out_dir = temp_dir.join("lo-out");
        tokio::fs::create_dir_all(&out_dir).await?;

        let mut cmd = Command::new(&self.config.libreoffice_path);
        cmd.arg(format!("-env:UserInstallation=file://{}", profile_dir.display()))
            .args(["--headless", "--norestore", "--nolockcheck", "--convert-to", "pdf", "--outdir"])
            .arg(&out_dir)
            .arg(input_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        let mut child = cmd.spawn().context("Failed to spawn LibreOffice")?;
        let mut stderr_pipe = child.stderr.take();

        let status = tokio::select! {
            _ = cancel_token.cancelled() => {
                kill_child_group(&mut child).await;
                return Err(anyhow!("Cancelled"));
            }
            res = child.wait() => res.context("LibreOffice execution failed")?,
        };

        if !status.success() {
            let mut stderr_buf = Vec::new();
            if let Some(ref mut pipe) = stderr_pipe {
                use tokio::io::AsyncReadExt;
                let _ = pipe.read_to_end(&mut stderr_buf).await;
            }
            let stderr = String::from_utf8_lossy(&stderr_buf);
            return Err(anyhow!("LibreOffice conversion failed: {}", stderr.trim()));
        }

        let pdf_path = find_pdf_in_dir(&out_dir).await?;
        let bytes = tokio::fs::read(&pdf_path)
            .await
            .context("Failed to read converted PDF")?;
        Ok(bytes)
    }

    async fn convert_with_google_drive(
        &self,
        input_path: &Path,
        source_mime: &str,
        google_mime: &str,
        cancel_token: CancellationToken,
    ) -> Result<Vec<u8>> {
        let access_token = self.resolve_google_access_token().await?;
        let file_bytes = tokio::fs::read(input_path).await?;

        let metadata = json!({
            "name": format!("distilla_temp_{}", chrono::Utc::now().timestamp_millis()),
            "mimeType": google_mime
        });

        let form = reqwest::multipart::Form::new()
            .part(
                "metadata",
                reqwest::multipart::Part::text(metadata.to_string())
                    .mime_str("application/json; charset=UTF-8")?,
            )
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_bytes).mime_str(source_mime)?,
            );

        let upload_fut = self
            .client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id")
            .bearer_auth(&access_token)
            .multipart(form)
            .send();

        let upload_res = tokio::select! {
            _ = cancel_token.cancelled() => return Err(anyhow!("Cancelled")),
            r = upload_fut => r.context("Google Drive upload request failed")?,
        };

        if !upload_res.status().is_success() {
            let text = upload_res.text().await.unwrap_or_default();
            return Err(anyhow!("Google Drive upload failed: {}", text));
        }

        let uploaded: DriveUploadResponse = upload_res.json().await?;
        let file_id = uploaded.id;

        let export_result = async {
            let export_url = format!(
                "https://www.googleapis.com/drive/v3/files/{}/export?mimeType=application/pdf",
                file_id
            );
            let export_res = tokio::select! {
                _ = cancel_token.cancelled() => return Err(anyhow!("Cancelled")),
                r = self.client.get(&export_url).bearer_auth(&access_token).send() => {
                    r.context("Google Drive export failed")?
                }
            };

            if !export_res.status().is_success() {
                let text = export_res.text().await.unwrap_or_default();
                return Err(anyhow!("Google Drive PDF export failed: {}", text));
            }
            let bytes = export_res.bytes().await?;
            Ok(bytes.to_vec())
        }
        .await;

        // Always delete temporary Drive file
        let delete_url = format!("https://www.googleapis.com/drive/v3/files/{}", file_id);
        let _ = self
            .client
            .delete(&delete_url)
            .bearer_auth(&access_token)
            .send()
            .await;

        export_result
    }

    async fn resolve_google_access_token(&self) -> Result<String> {
        let token_str = self
            .config
            .google_token_json
            .as_deref()
            .ok_or_else(|| anyhow!("GOOGLE_TOKEN not configured"))?;
        let token: GoogleToken =
            serde_json::from_str(token_str).context("Invalid GOOGLE_TOKEN JSON")?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        if let (Some(access), Some(expiry)) = (token.access_token.as_ref(), token.expiry_date) {
            if expiry > now_ms + 60_000 {
                return Ok(access.clone());
            }
        }

        let refresh_token = token
            .refresh_token
            .ok_or_else(|| anyhow!("Google OAuth refresh_token missing"))?;
        let client_id = self
            .config
            .google_client_id
            .as_deref()
            .ok_or_else(|| anyhow!("GOOGLE_CLIENT_ID missing"))?;
        let client_secret = self
            .config
            .google_client_secret
            .as_deref()
            .ok_or_else(|| anyhow!("GOOGLE_CLIENT_SECRET missing"))?;

        let res = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("refresh_token", refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .context("Failed to refresh Google OAuth token")?;

        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Google OAuth token refresh failed: {}", text));
        }

        let refreshed: TokenRefreshResponse = res.json().await?;
        Ok(refreshed.access_token)
    }
}

async fn find_pdf_in_dir(dir: &Path) -> Result<PathBuf> {
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        {
            return Ok(path);
        }
    }
    Err(anyhow!("LibreOffice did not produce a PDF file"))
}
