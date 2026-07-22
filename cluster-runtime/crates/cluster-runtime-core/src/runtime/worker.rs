use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerState {
    Idle,
    Busy,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: String,
    pub node_id: String,
    pub state: WorkerState,
    pub assigned_task: Option<String>,
    pub current_cpu_usage: f32,
    pub current_memory_usage: f32,
}
