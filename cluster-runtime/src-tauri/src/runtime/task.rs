use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Created,
    Queued,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub job_id: String,
    pub plugin: String,
    pub worker: Option<String>,
    pub cpu_requirement: f32,
    pub memory_requirement: u64,
    pub gpu_requirement: u64,
    pub dependencies: Vec<String>,
    pub retry_count: u32,
    pub timeout: u64,
    pub env_vars: HashMap<String, String>,
    pub status: TaskStatus,
}
