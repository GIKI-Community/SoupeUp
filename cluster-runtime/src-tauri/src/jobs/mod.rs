use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod history;

pub use history::JobHistory;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub status: JobStatus,
    pub owner: String,
    pub submitted_at: DateTime<Utc>,
    pub runtime: String,
    pub duration_secs: u64,
}

pub fn mock_jobs() -> Vec<Job> {
    Vec::new()
}
