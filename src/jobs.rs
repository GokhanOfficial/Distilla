use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Convert,
    Summarize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProgress {
    pub percent: u8,
    pub bitrate_kbps: u32,
    pub attempt: u8,
    pub duration_seconds: f64,
    pub elapsed_seconds: u64,
    pub eta_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStats {
    pub chars: usize,
    pub words: usize,
    pub elapsed_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub job_id: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub phase: String,
    pub message_key: String,
    pub filename: Option<String>,
    pub generated_title: Option<String>,
    pub markdown: String,
    pub progress: u8,
    pub media_progress: Option<MediaProgress>,
    pub ai_stats: Option<AiStats>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobEvent {
    State {
        status: JobStatus,
        phase: String,
        #[serde(rename = "messageKey")]
        message_key: String,
        progress: u8,
        #[serde(rename = "mediaProgress")]
        media_progress: Option<MediaProgress>,
        #[serde(rename = "aiStats")]
        ai_stats: Option<AiStats>,
    },
    Chunk {
        delta: String,
        #[serde(rename = "aiStats")]
        ai_stats: AiStats,
    },
    Done {
        status: JobStatus,
        markdown: String,
        #[serde(rename = "generatedTitle")]
        generated_title: Option<String>,
        error: Option<String>,
    },
}

struct JobEntry {
    snapshot: JobSnapshot,
    tx: broadcast::Sender<JobEvent>,
    cancel_token: CancellationToken,
    ai_started_at: Option<Instant>,
    completed_at: Option<Instant>,
}

#[derive(Clone)]
pub struct JobManager {
    jobs: Arc<RwLock<HashMap<String, JobEntry>>>,
}

impl JobManager {
    pub fn new() -> Self {
        let manager = Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        };
        manager.spawn_cleanup_loop();
        manager
    }

    pub async fn create_job(
        &self,
        kind: JobKind,
        filename: Option<String>,
    ) -> (String, CancellationToken) {
        let id = Uuid::new_v4().to_string();
        let (tx, _) = broadcast::channel(512);
        let cancel_token = CancellationToken::new();

        let snapshot = JobSnapshot {
            job_id: id.clone(),
            kind,
            status: JobStatus::Pending,
            phase: "queued".to_string(),
            message_key: "progress.queued".to_string(),
            filename,
            generated_title: None,
            markdown: String::new(),
            progress: 0,
            media_progress: None,
            ai_stats: None,
            error: None,
        };

        let entry = JobEntry {
            snapshot,
            tx,
            cancel_token: cancel_token.clone(),
            ai_started_at: None,
            completed_at: None,
        };

        self.jobs.write().await.insert(id.clone(), entry);
        (id, cancel_token)
    }

    pub async fn get_snapshot(&self, id: &str) -> Option<JobSnapshot> {
        self.jobs.read().await.get(id).map(|e| e.snapshot.clone())
    }

    pub async fn subscribe(
        &self,
        id: &str,
    ) -> Option<(JobSnapshot, broadcast::Receiver<JobEvent>)> {
        let jobs = self.jobs.read().await;
        jobs.get(id)
            .map(|e| (e.snapshot.clone(), e.tx.subscribe()))
    }

    pub async fn set_phase(
        &self,
        id: &str,
        phase: &str,
        message_key: &str,
        progress: u8,
        media_progress: Option<MediaProgress>,
    ) {
        let mut jobs = self.jobs.write().await;
        let Some(entry) = jobs.get_mut(id) else {
            return;
        };
        if entry.snapshot.status.is_terminal() {
            return;
        }
        entry.snapshot.status = JobStatus::Processing;
        entry.snapshot.phase = phase.to_string();
        entry.snapshot.message_key = message_key.to_string();
        entry.snapshot.progress = progress;
        entry.snapshot.media_progress = media_progress.clone();

        if phase == "ai-conversion" && entry.ai_started_at.is_none() {
            entry.ai_started_at = Some(Instant::now());
            entry.snapshot.ai_stats = Some(AiStats {
                chars: 0,
                words: 0,
                elapsed_seconds: 0,
            });
        }

        let _ = entry.tx.send(JobEvent::State {
            status: entry.snapshot.status,
            phase: entry.snapshot.phase.clone(),
            message_key: entry.snapshot.message_key.clone(),
            progress: entry.snapshot.progress,
            media_progress,
            ai_stats: entry.snapshot.ai_stats.clone(),
        });
    }

    pub async fn append_chunk(&self, id: &str, delta: &str) {
        let mut jobs = self.jobs.write().await;
        let Some(entry) = jobs.get_mut(id) else {
            return;
        };
        if entry.snapshot.status.is_terminal() {
            return;
        }
        entry.snapshot.markdown.push_str(delta);
        let elapsed_seconds = entry
            .ai_started_at
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);
        let chars = entry.snapshot.markdown.chars().count();
        let words = entry.snapshot.markdown.split_whitespace().count();
        let stats = AiStats {
            chars,
            words,
            elapsed_seconds,
        };
        entry.snapshot.ai_stats = Some(stats.clone());

        let _ = entry.tx.send(JobEvent::Chunk {
            delta: delta.to_string(),
            ai_stats: stats,
        });
    }

    pub async fn complete(
        &self,
        id: &str,
        final_markdown: String,
        generated_title: Option<String>,
    ) {
        let mut jobs = self.jobs.write().await;
        let Some(entry) = jobs.get_mut(id) else {
            return;
        };
        if entry.snapshot.status.is_terminal() {
            return;
        }
        entry.snapshot.status = JobStatus::Completed;
        entry.snapshot.phase = "completed".to_string();
        entry.snapshot.message_key = "progress.completed".to_string();
        entry.snapshot.progress = 100;
        entry.snapshot.media_progress = None;
        entry.snapshot.markdown = final_markdown.clone();
        entry.snapshot.generated_title = generated_title.clone();
        entry.completed_at = Some(Instant::now());

        let _ = entry.tx.send(JobEvent::Done {
            status: JobStatus::Completed,
            markdown: final_markdown,
            generated_title,
            error: None,
        });
    }

    pub async fn fail(&self, id: &str, error_msg: String) {
        let mut jobs = self.jobs.write().await;
        let Some(entry) = jobs.get_mut(id) else {
            return;
        };
        if entry.snapshot.status.is_terminal() {
            return;
        }
        entry.snapshot.status = JobStatus::Failed;
        entry.snapshot.phase = "failed".to_string();
        entry.snapshot.message_key = "progress.failed".to_string();
        entry.snapshot.media_progress = None;
        entry.snapshot.error = Some(error_msg.clone());
        entry.completed_at = Some(Instant::now());

        let _ = entry.tx.send(JobEvent::Done {
            status: JobStatus::Failed,
            markdown: entry.snapshot.markdown.clone(),
            generated_title: None,
            error: Some(error_msg),
        });
    }

    pub async fn cancel(&self, id: &str) -> Option<JobSnapshot> {
        let mut jobs = self.jobs.write().await;
        let entry = jobs.get_mut(id)?;
        if !entry.snapshot.status.is_terminal() {
            entry.cancel_token.cancel();
            entry.snapshot.status = JobStatus::Cancelled;
            entry.snapshot.phase = "cancelled".to_string();
            entry.snapshot.message_key = "progress.cancelled".to_string();
            entry.snapshot.media_progress = None;
            entry.snapshot.error = Some("Cancelled".to_string());
            entry.completed_at = Some(Instant::now());

            let _ = entry.tx.send(JobEvent::Done {
                status: JobStatus::Cancelled,
                markdown: entry.snapshot.markdown.clone(),
                generated_title: None,
                error: Some("Cancelled".to_string()),
            });
        }
        Some(entry.snapshot.clone())
    }

    pub async fn cancel_all(&self) {
        let mut jobs = self.jobs.write().await;
        for entry in jobs.values_mut() {
            if !entry.snapshot.status.is_terminal() {
                entry.cancel_token.cancel();
            }
        }
    }

    fn spawn_cleanup_loop(&self) {
        let jobs = self.jobs.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            let ttl = Duration::from_secs(30 * 60);
            loop {
                interval.tick().await;
                let mut map = jobs.write().await;
                map.retain(|_, entry| match entry.completed_at {
                    Some(done_at) => done_at.elapsed() < ttl,
                    None => true,
                });
            }
        });
    }
}
