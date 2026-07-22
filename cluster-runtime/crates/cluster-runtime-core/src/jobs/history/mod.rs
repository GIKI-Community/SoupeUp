use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::jobs::models::{Job, JobResult, JobSpec, JobStatus};

#[derive(Debug, Clone)]
struct HistoryRecord {
    job: Job,
    logs: Vec<String>,
    original_spec: Option<JobSpec>,
}

pub struct JobHistoryStore {
    records: Arc<RwLock<Vec<HistoryRecord>>>,
    persist_path: PathBuf,
}

impl JobHistoryStore {
    pub fn new(persist_path: PathBuf) -> Self {
        Self {
            records: Arc::new(RwLock::new(Vec::new())),
            persist_path,
        }
    }

    pub async fn load(&self) {
        if let Ok(data) = tokio::fs::read_to_string(&self.persist_path).await {
            let mut loaded = Vec::new();
            for line in data.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(record) = serde_json::from_str::<PersistedRecord>(line) {
                    loaded.push(HistoryRecord {
                        job: record.job,
                        logs: record.logs,
                        original_spec: record.original_spec,
                    });
                }
            }
            *self.records.write().await = loaded;
        }
    }

    async fn append_persist(&self, record: &PersistedRecord) {
        if let Some(parent) = self.persist_path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        if let Ok(line) = serde_json::to_string(record) {
            use tokio::io::AsyncWriteExt;
            if let Ok(mut file) = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.persist_path)
                .await
            {
                let _ = file.write_all(line.as_bytes()).await;
                let _ = file.write_all(b"\n").await;
            }
        }
    }

    pub async fn create(&self, job: Job, spec: JobSpec) -> String {
        let id = job.id.clone();
        let record = HistoryRecord {
            job,
            logs: vec![format!("[{}] Job created", Utc::now().to_rfc3339())],
            original_spec: Some(spec),
        };
        let persisted = PersistedRecord {
            job: record.job.clone(),
            logs: record.logs.clone(),
            original_spec: record.original_spec.clone(),
        };
        self.records.write().await.push(record);
        self.append_persist(&persisted).await;
        id
    }

    pub async fn update_status(&self, id: &str, status: JobStatus) {
        let mut records = self.records.write().await;
        if let Some(record) = records.iter_mut().find(|r| r.job.id == id) {
            record.job.status = status;
            record.logs.push(format!(
                "[{}] Status → {:?}",
                Utc::now().to_rfc3339(),
                record.job.status
            ));
        }
    }

    pub async fn mark_started(&self, id: &str) {
        let mut records = self.records.write().await;
        if let Some(record) = records.iter_mut().find(|r| r.job.id == id) {
            record.job.started_at = Some(Utc::now());
            record.job.status = JobStatus::Running;
            record.logs.push(format!("[{}] Job started", Utc::now().to_rfc3339()));
        }
    }

    pub async fn finish(&self, id: &str, result: &JobResult) {
        let mut records = self.records.write().await;
        if let Some(record) = records.iter_mut().find(|r| r.job.id == id) {
            record.job.status = result.status.clone();
            record.job.finished_at = Some(Utc::now());
            record.job.duration_secs = (result.metrics.execution_time_ms / 1000).max(1);
            record.logs.push(format!(
                "[{}] Job finished: {:?}",
                Utc::now().to_rfc3339(),
                result.status
            ));
            if let Some(err) = result.errors.first() {
                record.logs.push(format!("[{}] Error: {}", Utc::now().to_rfc3339(), err));
            }
        }
    }

    pub async fn append_log(&self, id: &str, message: &str) {
        let mut records = self.records.write().await;
        if let Some(record) = records.iter_mut().find(|r| r.job.id == id) {
            record.logs.push(format!("[{}] {}", Utc::now().to_rfc3339(), message));
        }
    }

    pub async fn list(&self) -> Vec<Job> {
        let now = Utc::now();
        self.records
            .read()
            .await
            .iter()
            .map(|record| {
                let mut job = record.job.clone();
                if job.status == JobStatus::Running {
                    if let Some(started) = job.started_at {
                        job.duration_secs = (now - started).num_seconds().max(0) as u64;
                    } else {
                        job.duration_secs = (now - job.submitted_at).num_seconds().max(0) as u64;
                    }
                }
                job
            })
            .collect()
    }

    pub async fn get(&self, id: &str) -> Option<(Job, Vec<String>, Option<JobSpec>)> {
        self.records.read().await.iter().find(|r| r.job.id == id).map(|r| {
            (r.job.clone(), r.logs.clone(), r.original_spec.clone())
        })
    }

    pub async fn get_spec(&self, id: &str) -> Option<JobSpec> {
        self.records
            .read()
            .await
            .iter()
            .find(|r| r.job.id == id)
            .and_then(|r| r.original_spec.clone())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedRecord {
    job: Job,
    logs: Vec<String>,
    original_spec: Option<JobSpec>,
}

// Legacy alias for backward compatibility during migration
pub type JobHistory = JobHistoryStore;
