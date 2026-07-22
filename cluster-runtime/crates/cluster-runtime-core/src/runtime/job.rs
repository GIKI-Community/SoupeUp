use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub plugin: String,
    pub priority: i32,
    pub owner: String,
    pub creation_time: String,
    pub task_count: usize,
    pub status: JobStatus,
}
