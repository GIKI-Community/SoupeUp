use std::sync::Arc;

use crate::jobs::history::JobHistoryStore;
use crate::jobs::manager::JobManager;
use crate::jobs::models::{
    Job, JobDetail, JobProgress, JobResult, JobSpec, JobStatus, SchedulerListEntry, SubmitAck,
};
use crate::scheduler::SchedulerRegistry;

pub struct JobApi {
    manager: Arc<JobManager>,
    registry: Arc<SchedulerRegistry>,
    #[allow(dead_code)]
    history: Arc<JobHistoryStore>,
}

impl JobApi {
    pub fn new(
        manager: Arc<JobManager>,
        registry: Arc<SchedulerRegistry>,
        history: Arc<JobHistoryStore>,
    ) -> Self {
        Self {
            manager,
            registry,
            history,
        }
    }

    pub async fn submit(&self, spec: JobSpec, owner: &str) -> Result<SubmitAck, String> {
        self.manager.submit(spec, owner).await
    }

    pub async fn cancel(&self, job_id: &str) -> Result<(), String> {
        self.manager.cancel(job_id).await
    }

    pub async fn status(&self, job_id: &str) -> Result<JobStatus, String> {
        self.manager.status(job_id).await
    }

    pub async fn progress(&self, job_id: &str) -> Result<JobProgress, String> {
        self.manager.progress(job_id).await
    }

    pub async fn result(&self, job_id: &str) -> Result<JobResult, String> {
        self.manager.result(job_id).await
    }

    pub async fn list(&self) -> Vec<Job> {
        self.manager.list().await
    }

    pub async fn get(&self, job_id: &str) -> Result<JobDetail, String> {
        self.manager.get(job_id).await
    }

    pub async fn retry(&self, job_id: &str) -> Result<SubmitAck, String> {
        self.manager.retry(job_id).await
    }

    pub async fn scheduler_list(&self) -> Vec<SchedulerListEntry> {
        self.registry.list().await
    }

    pub async fn scheduler_get_active(&self) -> String {
        self.registry.active_id().await
    }

    pub async fn scheduler_set_active(&self, plugin_id: &str) -> Result<(), String> {
        self.registry.set_active(plugin_id).await.map_err(|e| e.to_string())
    }
}
