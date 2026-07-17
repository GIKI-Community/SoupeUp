use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{Job, JobStatus};

struct JobRecord {
    job: Job,
}

/// In-memory job history for the Jobs UI (example runs, submissions, etc.).
pub struct JobHistory {
    records: Arc<RwLock<Vec<JobRecord>>>,
}

impl JobHistory {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn begin(&self, runtime: &str, owner: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let job = Job {
            id: id.clone(),
            status: JobStatus::Running,
            owner: owner.to_string(),
            submitted_at: Utc::now(),
            runtime: runtime.to_string(),
            duration_secs: 0,
        };
        self.records.write().await.push(JobRecord { job });
        id
    }

    pub async fn finish(&self, id: &str, success: bool, duration_ms: u64) {
        let mut records = self.records.write().await;
        if let Some(record) = records.iter_mut().find(|r| r.job.id == id) {
            record.job.status = if success {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            };
            record.job.duration_secs = (duration_ms / 1000).max(1);
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
                    job.duration_secs = (now - job.submitted_at).num_seconds().max(0) as u64;
                }
                job
            })
            .collect()
    }
}

impl Default for JobHistory {
    fn default() -> Self {
        Self::new()
    }
}
