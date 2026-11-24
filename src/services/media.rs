use crate::config::{find_in_path, MediaConfig};
use crate::formats::{FormatKind, FormatSpec};
use crate::jobs::MediaProgress;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::VecDeque;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct PreparedAudio {
    pub path: PathBuf,
    pub format: String,
    pub size_bytes: u64,
    pub bitrate_kbps: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub duration_seconds: f64,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    format: Option<FfprobeFormat>,
    streams: Option<Vec<FfprobeStream>>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
}

#[derive(Clone)]
pub struct MediaEncoder {
    config: MediaConfig,
}

impl MediaEncoder {
    pub fn new(config: MediaConfig) -> Self {
        Self { config }
    }

    pub fn is_ffmpeg_available(&self) -> bool {
        find_in_path(&self.config.ffmpeg_path)
    }

    pub fn is_ffprobe_available(&self) -> bool {
        find_in_path(&self.config.ffprobe_path)
    }

    pub fn max_output_bytes(&self) -> u64 {
        self.config.max_output_size_mb * 1024 * 1024
    }

    pub fn target_output_bytes(&self) -> u64 {
        self.config.target_output_size_mb * 1024 * 1024
    }

    pub fn calculate_bitrate(&self, duration_secs: f64, target_bytes: u64) -> Result<u32> {
        if !duration_secs.is_finite() || duration_secs <= 0.0 {
            return Err(anyhow!("Media duration must be greater than zero"));
        }
        let safe_bytes = (target_bytes as f64) * 0.94;
        let kbps = ((safe_bytes * 8.0) / duration_secs / 1000.0).floor() as u32;
        Ok(kbps.clamp(
            self.config.min_audio_bitrate_kbps,
            self.config.max_audio_bitrate_kbps,
        ))
    }

    pub fn estimate_size_bytes(&self, duration_secs: f64, bitrate_kbps: u32) -> u64 {
        ((duration_secs * (bitrate_kbps as f64) * 1000.0) / 8.0).ceil() as u64
    }

    pub fn ensure_fits_minimum_bitrate(&self, duration_secs: f64) -> Result<()> {
        let estimated = self.estimate_size_bytes(duration_secs, self.config.min_audio_bitrate_kbps);
        if estimated > self.max_output_bytes() {
            let est_mb = estimated as f64 / (1024.0 * 1024.0);
            return Err(anyhow!(
                "Media cannot fit under {}MB even at {}kbps minimum bitrate (duration: {}s, estimated: {:.1}MB)",
                self.config.max_output_size_mb,
                self.config.min_audio_bitrate_kbps,
                duration_secs.round() as u64,
                est_mb
            ));
        }
        Ok(())
    }

    pub async fn probe(&self, input_path: &Path) -> Result<MediaInfo> {
        let output = Command::new(&self.config.ffprobe_path)
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(input_path)
            .output()
            .await
            .context("Failed to run ffprobe")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("ffprobe failed: {}", stderr.trim()));
        }

        let parsed: FfprobeOutput =
            serde_json::from_slice(&output.stdout).context("Invalid ffprobe JSON output")?;

        let has_audio = parsed
            .streams
            .unwrap_or_default()
            .iter()
            .any(|s| s.codec_type.as_deref() == Some("audio"));

        if !has_audio {
            return Err(anyhow!("No audio stream found in media file"));
        }

        let duration = parsed
            .format
            .and_then(|f| f.duration)
            .and_then(|d| d.parse::<f64>().ok())
            .filter(|d| d.is_finite() && *d > 0.0)
            .ok_or_else(|| anyhow!("Could not determine media duration"))?;

        Ok(MediaInfo {
            duration_seconds: duration,
        })
    }

    pub async fn prepare_audio<F, Fut>(
        &self,
        input_path: &Path,
        spec: &FormatSpec,
        temp_dir: &Path,
        cancel_token: CancellationToken,
        mut on_progress: F,
    ) -> Result<PreparedAudio>
    where
        F: FnMut(MediaProgress) -> Fut,
        Fut: Future<Output = ()>,
    {
        let meta = tokio::fs::metadata(input_path)
            .await
            .context("Failed to stat input media")?;
        let input_size = meta.len();

        // Direct native audio under output size limit does NOT require FFmpeg at all.
        if let FormatKind::DirectAudio { format } = spec.kind {
            if input_size <= self.max_output_bytes() {
                return Ok(PreparedAudio {
                    path: input_path.to_path_buf(),
                    format: format.to_string(),
                    size_bytes: input_size,
                    bitrate_kbps: None,
                });
            }
        }

        if !self.is_ffmpeg_available() {
            return Err(anyhow!(
                "FFmpeg is not installed or FFMPEG_PATH is invalid"
            ));
        }
        if !self.is_ffprobe_available() {
            return Err(anyhow!(
                "FFprobe is not installed or FFPROBE_PATH is invalid"
            ));
        }

        let info = self.probe(input_path).await?;
        self.ensure_fits_minimum_bitrate(info.duration_seconds)?;

        let mut bitrate = self.calculate_bitrate(info.duration_seconds, self.target_output_bytes())?;
        let mut last_output: Option<PathBuf> = None;

        for attempt in 1u8..=3u8 {
            if cancel_token.is_cancelled() {
                return Err(anyhow!("Cancelled"));
            }

            let out_path = temp_dir.join(format!("encoded-attempt-{}.mp3", attempt));
            self.run_ffmpeg(
                input_path,
                &out_path,
                info.duration_seconds,
                bitrate,
                attempt,
                cancel_token.clone(),
                &mut on_progress,
            )
            .await?;

            if let Some(prev) = last_output.take() {
                let _ = tokio::fs::remove_file(prev).await;
            }
            last_output = Some(out_path.clone());

            let out_size = tokio::fs::metadata(&out_path)
                .await
                .context("Failed to stat encoded MP3")?
                .len();

            if out_size <= self.max_output_bytes() {
                return Ok(PreparedAudio {
                    path: out_path,
                    format: "mp3".to_string(),
                    size_bytes: out_size,
                    bitrate_kbps: Some(bitrate),
                });
            }

            if bitrate <= self.config.min_audio_bitrate_kbps {
                let out_mb = out_size as f64 / (1024.0 * 1024.0);
                return Err(anyhow!(
                    "Encoded MP3 still exceeds {}MB at {}kbps ({:.1}MB)",
                    self.config.max_output_size_mb,
                    self.config.min_audio_bitrate_kbps,
                    out_mb
                ));
            }

            // Compute next lower bitrate and allow the next iteration to actually try it (including min bitrate)
            let ratio = (self.target_output_bytes() as f64) / (out_size as f64);
            let next_kbps = ((bitrate as f64) * ratio * 0.96).floor() as u32;
            bitrate = next_kbps.max(self.config.min_audio_bitrate_kbps);
        }

        Err(anyhow!(
            "Failed to compress audio under {}MB within 3 attempts",
            self.config.max_output_size_mb
        ))
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_ffmpeg<F, Fut>(
        &self,
        input_path: &Path,
        output_path: &Path,
        duration_secs: f64,
        bitrate_kbps: u32,
        attempt: u8,
        cancel_token: CancellationToken,
        on_progress: &mut F,
    ) -> Result<()>
    where
        F: FnMut(MediaProgress) -> Fut,
        Fut: Future<Output = ()>,
    {
        let channels = if bitrate_kbps <= 48 { "1" } else { "2" };
        let mut cmd = Command::new(&self.config.ffmpeg_path);
        cmd.args([
            "-hide_banner",
            "-y",
            "-i",
        ])
        .arg(input_path)
        .args([
            "-vn",
            "-map",
            "0:a:0",
            "-codec:a",
            "libmp3lame",
            "-b:a",
            &format!("{}k", bitrate_kbps),
            "-ar",
            &self.config.output_sample_rate.to_string(),
            "-ac",
            channels,
            "-f",
            "mp3",
            "-progress",
            "pipe:2",
            "-nostats",
        ])
        .arg(output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

        // Start FFmpeg in its own process group so cancel kills cleanly
        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        let mut child = cmd.spawn().context("Failed to spawn ffmpeg")?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("Failed to capture ffmpeg stderr"))?;
        let mut reader = BufReader::new(stderr).lines();
        let mut tail_lines: VecDeque<String> = VecDeque::with_capacity(20);
        let started = Instant::now();
        let mut last_reported_percent: Option<u8> = None;

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    kill_child_group(&mut child).await;
                    return Err(anyhow!("Cancelled"));
                }
                line_res = reader.next_line() => {
                    match line_res? {
                        Some(line) => {
                            if let Some(raw_us) = line
                                .strip_prefix("out_time_us=")
                                .or_else(|| line.strip_prefix("out_time_ms="))
                            {
                                if let Ok(us) = raw_us.trim().parse::<u64>() {
                                    let current_secs = (us as f64) / 1_000_000.0;
                                    let pct = ((current_secs / duration_secs) * 100.0)
                                        .round()
                                        .clamp(0.0, 100.0) as u8;
                                    if last_reported_percent != Some(pct) {
                                        last_reported_percent = Some(pct);
                                        let elapsed = started.elapsed().as_secs();
                                        let eta = if pct > 0 && pct < 100 {
                                            let el_f = started.elapsed().as_secs_f64().max(0.5);
                                            Some(((el_f / (pct as f64)) * ((100 - pct) as f64)).ceil() as u64)
                                        } else {
                                            None
                                        };
                                        on_progress(MediaProgress {
                                            percent: pct,
                                            bitrate_kbps,
                                            attempt,
                                            duration_seconds: duration_secs,
                                            elapsed_seconds: elapsed,
                                            eta_seconds: eta,
                                        }).await;
                                    }
                                }
                            } else if !line.contains('=') && !line.trim().is_empty() {
                                if tail_lines.len() >= 20 {
                                    tail_lines.pop_front();
                                }
                                tail_lines.push_back(line);
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        let status = tokio::select! {
            _ = cancel_token.cancelled() => {
                kill_child_group(&mut child).await;
                return Err(anyhow!("Cancelled"));
            }
            st = child.wait() => st.context("Failed waiting for ffmpeg")?,
        };

        if !status.success() {
            let err_summary = tail_lines.into_iter().collect::<Vec<_>>().join(" | ");
            return Err(anyhow!(
                "FFmpeg exited with {}: {}",
                status,
                if err_summary.is_empty() {
                    "unknown error".to_string()
                } else {
                    err_summary
                }
            ));
        }

        on_progress(MediaProgress {
            percent: 100,
            bitrate_kbps,
            attempt,
            duration_seconds: duration_secs,
            elapsed_seconds: started.elapsed().as_secs(),
            eta_seconds: Some(0),
        })
        .await;

        Ok(())
    }
}

pub async fn kill_child_group(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        unsafe {
            libc::kill(-(pid as libc::pid_t), libc::SIGTERM);
        }
        tokio::select! {
            _ = child.wait() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
                unsafe {
                    libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
                }
            }
        }
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_encoder() -> MediaEncoder {
        MediaEncoder::new(MediaConfig {
            ffmpeg_path: "ffmpeg".into(),
            ffprobe_path: "ffprobe".into(),
            max_output_size_mb: 100,
            target_output_size_mb: 95,
            min_audio_bitrate_kbps: 32,
            max_audio_bitrate_kbps: 320,
            output_sample_rate: 44100,
        })
    }

    #[test]
    fn calculates_clamped_bitrate() {
        let enc = test_encoder();
        // Short 60s clip -> clamped to 320 kbps
        assert_eq!(enc.calculate_bitrate(60.0, enc.target_output_bytes()).unwrap(), 320);
        // 2 hours (7200s) -> ~104 kbps
        let kbps = enc.calculate_bitrate(7200.0, enc.target_output_bytes()).unwrap();
        assert!((95..=115).contains(&kbps));
        // Zero duration fails
        assert!(enc.calculate_bitrate(0.0, enc.target_output_bytes()).is_err());
    }

    #[test]
    fn enforces_minimum_bitrate_limit() {
        let enc = test_encoder();
        // 1 hour fits easily at 32kbps
        assert!(enc.ensure_fits_minimum_bitrate(3600.0).is_ok());
        // 10 hours (36000s) at 32kbps is ~144MB > 100MB -> fails
        assert!(enc.ensure_fits_minimum_bitrate(36000.0).is_err());
    }
}
