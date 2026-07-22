use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClusterHealth {
    Healthy,
    Degraded,
    Unhealthy,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadInfo {
    pub status: ComponentStatus,
    pub address: Option<String>,
    pub dashboard_url: Option<String>,
    pub process_id: Option<String>,
    pub host: String,
    pub port: u16,
    pub dashboard_port: u16,
    pub started_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    #[serde(default)]
    pub logs: String,
}

impl Default for HeadInfo {
    fn default() -> Self {
        Self {
            status: ComponentStatus::Stopped,
            address: None,
            dashboard_url: None,
            process_id: None,
            host: "0.0.0.0".to_string(),
            port: 6379,
            dashboard_port: 8265,
            started_at: None,
            error: None,
            logs: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerInfo {
    pub status: ComponentStatus,
    pub name: String,
    pub head_address: String,
    pub process_id: Option<String>,
    pub num_cpus: usize,
    pub object_store_memory: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    #[serde(default)]
    pub logs: String,
}

impl Default for WorkerInfo {
    fn default() -> Self {
        Self {
            status: ComponentStatus::Stopped,
            name: "ray-worker-1".to_string(),
            head_address: "127.0.0.1:6379".to_string(),
            process_id: None,
            num_cpus: 0,
            object_store_memory: None,
            started_at: None,
            error: None,
            logs: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedWorker {
    pub id: String,
    pub name: String,
    pub address: String,
    pub nthreads: usize,
    pub memory_limit: u64,
    pub memory_used: u64,
    pub cpu: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClusterSnapshot {
    pub health: ClusterHealth,
    pub head: HeadInfo,
    pub local_worker: WorkerInfo,
    pub workers: Vec<ConnectedWorker>,
    pub total_cores: usize,
    pub total_memory: u64,
    pub active_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub bandwidth_bytes_per_sec: f64,
    pub client_connected: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub job_id: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub workers_used: usize,
    pub cpu_utilization: Option<f64>,
    pub speedup: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleJobResult {
    pub example_id: String,
    pub title: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub workers_used: usize,
    pub cpu_utilization: Option<f64>,
    pub speedup: Option<f64>,
    pub result_summary: String,
    pub details: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Error)]
pub enum RayError {
    #[error("Python Runtime unavailable: {0}")]
    PythonUnavailable(String),

    #[error("Package installation failed: {0}")]
    PackageError(String),

    #[error("Head error: {0}")]
    HeadError(String),

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("Client error: {0}")]
    ClientError(String),

    #[error("Job error: {0}")]
    JobError(String),

    #[error("Not ready: {0}")]
    NotReady(String),

    #[error("JSON error: {0}")]
    JsonError(String),
}

pub type RayResult<T> = Result<T, RayError>;
