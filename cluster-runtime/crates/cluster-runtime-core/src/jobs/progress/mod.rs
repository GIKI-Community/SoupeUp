use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::jobs::models::{JobProgress, JobStatus};

pub struct ProgressTracker {
    records: Arc<RwLock<HashMap<String, JobProgress>>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set(&self, job_id: &str, progress: JobProgress) {
        self.records
            .write()
            .await
            .insert(job_id.to_string(), progress);
    }

    pub async fn get(&self, job_id: &str) -> JobProgress {
        self.records
            .read()
            .await
            .get(job_id)
            .cloned()
            .unwrap_or_else(|| JobProgress {
                percent: 0.0,
                active_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                running_nodes: vec![],
                eta_secs: None,
                extra: serde_json::Value::Null,
            })
    }

    pub async fn update_for_status(&self, job_id: &str, status: &JobStatus) {
        let mut progress = self.get(job_id).await;
        match status {
            JobStatus::Created | JobStatus::Queued => progress.percent = 0.0,
            JobStatus::Scheduling => progress.percent = 5.0,
            JobStatus::Running => {
                if progress.percent < 10.0 {
                    progress.percent = 10.0;
                }
            }
            JobStatus::Completed => progress.percent = 100.0,
            JobStatus::Failed | JobStatus::Cancelled => {}
        }
        self.set(job_id, progress).await;
    }

    pub async fn complete(&self, job_id: &str, success: bool) {
        let mut progress = self.get(job_id).await;
        progress.percent = if success { 100.0 } else { progress.percent };
        if !success {
            progress.failed_tasks = progress.failed_tasks.max(1);
        } else {
            progress.completed_tasks = progress.completed_tasks.max(1);
        }
        self.set(job_id, progress).await;
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
