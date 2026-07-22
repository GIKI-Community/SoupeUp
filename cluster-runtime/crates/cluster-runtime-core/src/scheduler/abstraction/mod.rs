use async_trait::async_trait;
use thiserror::Error;

use crate::jobs::models::{
    DependencyReport, JobProgress, JobResult, JobSpec, JobStatus, JobSummary, SchedulerCapabilities,
    SchedulerInfo, SubmitAck,
};

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Scheduler not ready: {0}")]
    NotReady(String),
    #[error("Job not found: {0}")]
    JobNotFound(String),
    #[error("Job error: {0}")]
    JobError(String),
    #[error("Cluster error: {0}")]
    ClusterError(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}

pub type SchedulerResult<T> = Result<T, SchedulerError>;

#[async_trait]
pub trait SchedulerService: Send + Sync {
    fn plugin_id(&self) -> &str;
    fn display_name(&self) -> &str;

    async fn capabilities(&self) -> SchedulerCapabilities;
    async fn cluster_info(&self) -> SchedulerResult<SchedulerInfo>;

    /// Detect imports in the job source and install missing packages into the
    /// active Python venv (shared by local workers). Default is a no-op.
    async fn ensure_job_dependencies(&self, _spec: &JobSpec) -> SchedulerResult<DependencyReport> {
        Ok(DependencyReport::default())
    }

    async fn submit(&self, job_id: &str, spec: &JobSpec) -> SchedulerResult<SubmitAck>;
    async fn cancel(&self, job_id: &str) -> SchedulerResult<()>;
    async fn status(&self, job_id: &str) -> SchedulerResult<JobStatus>;
    async fn progress(&self, job_id: &str) -> SchedulerResult<JobProgress>;
    async fn result(&self, job_id: &str) -> SchedulerResult<JobResult>;
    async fn list_jobs(&self) -> SchedulerResult<Vec<JobSummary>>;
}
