use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Created,
    Queued,
    Scheduling,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Created,
    Queued,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    pub cpu_cores: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub gpu_count: Option<u32>,
    pub python_version: Option<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    pub arch: Option<String>,
    pub os: Option<String>,
    pub runtime_type: Option<String>,
}

/// Result of auto-detecting and installing Python dependencies for a job.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DependencyReport {
    /// Top-level import names found in the job source.
    #[serde(default)]
    pub detected: Vec<String>,
    /// Pip package names that were newly installed.
    #[serde(default)]
    pub installed: Vec<String>,
    /// Import / package names that were already available.
    #[serde(default)]
    pub already_present: Vec<String>,
    /// Import names skipped because they are part of the Python stdlib.
    #[serde(default)]
    pub skipped_stdlib: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContext {
    pub working_dir: Option<String>,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    #[serde(default)]
    pub cli_args: Vec<String>,
    #[serde(default)]
    pub input_files: Vec<String>,
    pub output_dir: Option<String>,
    pub package_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EntryPoint {
    PythonFunction { body: String },
    PythonScript { script: String },
    PythonModule { module: String },
    Example {
        #[serde(rename = "exampleId")]
        example_id: String,
        #[serde(default)]
        args: Option<serde_json::Value>,
    },
    /// Native MPI launch via `mpirun` / `mpiexec`.
    MpiExecutable {
        executable: String,
        #[serde(default)]
        ranks: Option<u32>,
        #[serde(default)]
        hostfile: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSpec {
    pub name: String,
    pub description: Option<String>,
    pub entry_point: EntryPoint,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub resources: ResourceRequirements,
    #[serde(default)]
    pub priority: i32,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub retry_policy: Option<RetryPolicy>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub execution_context: ExecutionContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub entry_point: EntryPoint,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub resources: ResourceRequirements,
    #[serde(default)]
    pub priority: i32,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub retry_policy: Option<RetryPolicy>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    pub status: JobStatus,
    pub scheduler_id: String,
    pub owner: String,
    pub submitted_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_secs: u64,
    #[serde(default)]
    pub execution_context: ExecutionContext,
    /// Auto-detected / installed Python dependencies for this job (if resolved).
    #[serde(default)]
    pub dependencies: Option<DependencyReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub job_id: String,
    pub name: String,
    pub entry_point: EntryPoint,
    #[serde(default)]
    pub resources: ResourceRequirements,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub name: String,
    pub path: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub percent: f64,
    pub active_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    #[serde(default)]
    pub running_nodes: Vec<String>,
    pub eta_secs: Option<u64>,
    #[serde(default)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JobMetrics {
    pub execution_time_ms: u64,
    pub workers_used: usize,
    pub cpu_utilization: Option<f64>,
    pub speedup: Option<f64>,
    #[serde(default)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub job_id: String,
    pub status: JobStatus,
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub metrics: JobMetrics,
    #[serde(default)]
    pub scheduler_metadata: serde_json::Value,
    #[serde(default)]
    pub workers: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    pub result_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub errors: Vec<String>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerCapabilities {
    pub supports_python: bool,
    pub supports_actors: bool,
    pub supports_dags: bool,
    pub supports_gpu: bool,
    pub supports_fault_tolerance: bool,
    pub supports_autoscaling: bool,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerInfo {
    pub plugin_id: String,
    pub display_name: String,
    pub health: String,
    pub address: Option<String>,
    pub dashboard_url: Option<String>,
    pub worker_count: usize,
    pub total_cores: usize,
    pub client_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    pub id: String,
    pub name: String,
    pub status: JobStatus,
    pub scheduler_id: String,
    pub submitted_at: DateTime<Utc>,
    pub duration_secs: u64,
    pub progress_percent: f64,
    pub result_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAck {
    pub job_id: String,
    pub status: JobStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDetail {
    #[serde(flatten)]
    pub job: Job,
    pub progress: JobProgress,
    pub result: Option<JobResult>,
    #[serde(default)]
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerListEntry {
    pub plugin_id: String,
    pub display_name: String,
    pub capabilities: SchedulerCapabilities,
    pub available: bool,
}

impl JobSpec {
    pub fn example(name: &str, example_id: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            entry_point: EntryPoint::Example {
                example_id: example_id.to_string(),
                args: None,
            },
            args: serde_json::Value::Null,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            priority: 0,
            timeout_secs: None,
            retry_policy: None,
            tags: vec!["example".to_string()],
            metadata: HashMap::new(),
            execution_context: ExecutionContext::default(),
        }
    }
}

impl From<&JobSpec> for EntryPoint {
    fn from(spec: &JobSpec) -> Self {
        spec.entry_point.clone()
    }
}
