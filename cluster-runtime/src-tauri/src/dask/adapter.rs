use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use crate::dask::types::{ExampleJobResult, JobResult as DaskJobResult};
use crate::dask::DaskService;
use crate::jobs::models::{
    DependencyReport, EntryPoint, JobMetrics, JobProgress, JobResult, JobSpec, JobStatus, JobSummary,
    SchedulerCapabilities, SchedulerInfo, SubmitAck,
};
use crate::scheduler::abstraction::{SchedulerError, SchedulerResult, SchedulerService};
use crate::scheduler::selection::DASK_PLUGIN_ID;

#[derive(Debug, Clone)]
struct TrackedJob {
    status: JobStatus,
    result: Option<JobResult>,
    name: String,
    submitted_at: chrono::DateTime<Utc>,
}

pub struct DaskSchedulerAdapter {
    service: Arc<DaskService>,
    jobs: Arc<RwLock<HashMap<String, TrackedJob>>>,
}

impl DaskSchedulerAdapter {
    pub fn new(service: Arc<DaskService>) -> Self {
        Self {
            service,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn map_dask_result(job_id: &str, raw: DaskJobResult) -> JobResult {
        JobResult {
            job_id: job_id.to_string(),
            status: if raw.success {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            },
            output: raw.result,
            errors: raw.error.map(|e| vec![e]).unwrap_or_default(),
            metrics: JobMetrics {
                execution_time_ms: raw.execution_time_ms,
                workers_used: raw.workers_used,
                cpu_utilization: raw.cpu_utilization,
                speedup: raw.speedup,
                extra: serde_json::Value::Null,
            },
            scheduler_metadata: serde_json::json!({ "schedulerJobId": raw.job_id }),
            workers: vec![],
            artifacts: vec![],
            result_summary: None,
        }
    }

    fn map_example_result(job_id: &str, raw: ExampleJobResult) -> JobResult {
        JobResult {
            job_id: job_id.to_string(),
            status: if raw.success {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            },
            output: raw.details,
            errors: raw.error.map(|e| vec![e]).unwrap_or_default(),
            metrics: JobMetrics {
                execution_time_ms: raw.execution_time_ms,
                workers_used: raw.workers_used,
                cpu_utilization: raw.cpu_utilization,
                speedup: raw.speedup,
                extra: serde_json::json!({ "exampleId": raw.example_id }),
            },
            scheduler_metadata: serde_json::Value::Null,
            workers: vec![],
            artifacts: vec![],
            result_summary: Some(raw.result_summary),
        }
    }

    async fn execute_spec(&self, job_id: &str, spec: &JobSpec) -> SchedulerResult<JobResult> {
        {
            let mut jobs = self.jobs.write().await;
            if let Some(tracked) = jobs.get_mut(job_id) {
                tracked.status = JobStatus::Running;
            }
        }

        let result = match &spec.entry_point {
            EntryPoint::PythonFunction { body } => self
                .service
                .submit_python_function(body.clone(), spec.args.clone())
                .await
                .map(|r| Self::map_dask_result(job_id, r))
                .map_err(|e| SchedulerError::JobError(e.to_string())),
            EntryPoint::PythonScript { script } => self
                .service
                .submit_script(script.clone())
                .await
                .map(|r| Self::map_dask_result(job_id, r))
                .map_err(|e| SchedulerError::JobError(e.to_string())),
            EntryPoint::PythonModule { module } => self
                .service
                .submit_module(module.clone())
                .await
                .map(|r| Self::map_dask_result(job_id, r))
                .map_err(|e| SchedulerError::JobError(e.to_string())),
            EntryPoint::Example { example_id, .. } => self
                .service
                .run_example(example_id.clone())
                .await
                .map(|r| Self::map_example_result(job_id, r))
                .map_err(|e| SchedulerError::JobError(e.to_string())),
            EntryPoint::MpiExecutable { .. } => Err(SchedulerError::Unsupported(
                "MpiExecutable jobs require the MPI scheduler".into(),
            )),
        }?;

        {
            let mut jobs = self.jobs.write().await;
            if let Some(tracked) = jobs.get_mut(job_id) {
                tracked.status = result.status.clone();
                tracked.result = Some(result.clone());
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl SchedulerService for DaskSchedulerAdapter {
    fn plugin_id(&self) -> &str {
        DASK_PLUGIN_ID
    }

    fn display_name(&self) -> &str {
        "Dask"
    }

    async fn capabilities(&self) -> SchedulerCapabilities {
        SchedulerCapabilities {
            supports_python: true,
            supports_actors: false,
            supports_dags: false,
            supports_gpu: false,
            supports_fault_tolerance: true,
            supports_autoscaling: false,
            supports_streaming: false,
        }
    }

    async fn cluster_info(&self) -> SchedulerResult<SchedulerInfo> {
        let snap = self
            .service
            .cluster_snapshot()
            .await
            .map_err(|e| SchedulerError::ClusterError(e.to_string()))?;
        Ok(SchedulerInfo {
            plugin_id: DASK_PLUGIN_ID.to_string(),
            display_name: "Dask".to_string(),
            health: format!("{:?}", snap.health).to_lowercase(),
            address: snap.scheduler.address.clone(),
            dashboard_url: snap.scheduler.dashboard_url.clone(),
            worker_count: snap.workers.len(),
            total_cores: snap.total_cores,
            client_connected: snap.client_connected,
        })
    }

    async fn ensure_job_dependencies(&self, spec: &JobSpec) -> SchedulerResult<DependencyReport> {
        self.service
            .ensure_job_dependencies(spec)
            .await
            .map_err(|e| SchedulerError::JobError(e.to_string()))
    }

    async fn submit(&self, job_id: &str, spec: &JobSpec) -> SchedulerResult<SubmitAck> {
        self.jobs.write().await.insert(
            job_id.to_string(),
            TrackedJob {
                status: JobStatus::Scheduling,
                result: None,
                name: spec.name.clone(),
                submitted_at: Utc::now(),
            },
        );

        let jobs = self.jobs.clone();
        let job_id_owned = job_id.to_string();
        let spec_clone = spec.clone();

        // Execute synchronously in current task since Dask jobs block today
        let this = DaskSchedulerAdapter {
            service: self.service.clone(),
            jobs: jobs.clone(),
        };
        match this.execute_spec(&job_id_owned, &spec_clone).await {
            Ok(result) => {
                if let Some(tracked) = jobs.write().await.get_mut(&job_id_owned) {
                    tracked.status = result.status.clone();
                    tracked.result = Some(result);
                }
            }
            Err(e) => {
                let failed = JobResult {
                    job_id: job_id_owned.clone(),
                    status: JobStatus::Failed,
                    output: None,
                    errors: vec![e.to_string()],
                    metrics: JobMetrics::default(),
                    scheduler_metadata: serde_json::Value::Null,
                    workers: vec![],
                    artifacts: vec![],
                    result_summary: None,
                };
                if let Some(tracked) = jobs.write().await.get_mut(&job_id_owned) {
                    tracked.status = JobStatus::Failed;
                    tracked.result = Some(failed);
                }
            }
        }

        let status = jobs
            .read()
            .await
            .get(job_id)
            .map(|j| j.status.clone())
            .unwrap_or(JobStatus::Failed);

        Ok(SubmitAck {
            job_id: job_id.to_string(),
            status,
        })
    }

    async fn cancel(&self, job_id: &str) -> SchedulerResult<()> {
        self.service
            .cancel_job(job_id.to_string())
            .await
            .map_err(|e| SchedulerError::JobError(e.to_string()))?;
        if let Some(tracked) = self.jobs.write().await.get_mut(job_id) {
            tracked.status = JobStatus::Cancelled;
        }
        Ok(())
    }

    async fn status(&self, job_id: &str) -> SchedulerResult<JobStatus> {
        self.jobs
            .read()
            .await
            .get(job_id)
            .map(|j| j.status.clone())
            .ok_or_else(|| SchedulerError::JobNotFound(job_id.to_string()))
    }

    async fn progress(&self, job_id: &str) -> SchedulerResult<JobProgress> {
        let status = self.status(job_id).await?;
        let percent = match status {
            JobStatus::Created | JobStatus::Queued => 0.0,
            JobStatus::Scheduling => 5.0,
            JobStatus::Running => 50.0,
            JobStatus::Completed => 100.0,
            JobStatus::Failed | JobStatus::Cancelled => 0.0,
        };
        Ok(JobProgress {
            percent,
            active_tasks: if status == JobStatus::Running { 1 } else { 0 },
            completed_tasks: if status == JobStatus::Completed { 1 } else { 0 },
            failed_tasks: if status == JobStatus::Failed { 1 } else { 0 },
            running_nodes: vec![],
            eta_secs: None,
            extra: serde_json::Value::Null,
        })
    }

    async fn result(&self, job_id: &str) -> SchedulerResult<JobResult> {
        self.jobs
            .read()
            .await
            .get(job_id)
            .and_then(|j| j.result.clone())
            .ok_or_else(|| SchedulerError::JobNotFound(job_id.to_string()))
    }

    async fn list_jobs(&self) -> SchedulerResult<Vec<JobSummary>> {
        let jobs = self.jobs.read().await;
        Ok(jobs
            .iter()
            .map(|(id, tracked)| JobSummary {
                id: id.clone(),
                name: tracked.name.clone(),
                status: tracked.status.clone(),
                scheduler_id: DASK_PLUGIN_ID.to_string(),
                submitted_at: tracked.submitted_at,
                duration_secs: tracked
                    .result
                    .as_ref()
                    .map(|r| r.metrics.execution_time_ms / 1000)
                    .unwrap_or(0),
                progress_percent: if tracked.status == JobStatus::Completed {
                    100.0
                } else {
                    0.0
                },
                result_summary: tracked.result.as_ref().and_then(|r| r.result_summary.clone()),
            })
            .collect())
    }
}
