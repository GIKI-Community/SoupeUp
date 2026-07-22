use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use crate::jobs::models::{
    DependencyReport, EntryPoint, JobMetrics, JobProgress, JobResult, JobSpec, JobStatus, JobSummary,
    SchedulerCapabilities, SchedulerInfo, SubmitAck,
};
use crate::mpi::jobs::is_python_mpi_entry;
use crate::mpi::types::MpiLaunchResult;
use crate::mpi::MpiService;
use crate::scheduler::abstraction::{SchedulerError, SchedulerResult, SchedulerService};
use crate::scheduler::selection::MPI_PLUGIN_ID;

#[derive(Debug, Clone)]
struct TrackedJob {
    status: JobStatus,
    result: Option<JobResult>,
    name: String,
    submitted_at: chrono::DateTime<Utc>,
}

pub struct MpiSchedulerAdapter {
    service: Arc<MpiService>,
    jobs: Arc<RwLock<HashMap<String, TrackedJob>>>,
}

impl MpiSchedulerAdapter {
    pub fn new(service: Arc<MpiService>) -> Self {
        Self {
            service,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn map_result(job_id: &str, raw: MpiLaunchResult) -> JobResult {
        JobResult {
            job_id: job_id.to_string(),
            status: if raw.success {
                JobStatus::Completed
            } else {
                JobStatus::Failed
            },
            output: Some(serde_json::json!({
                "stdout": raw.stdout,
                "stderr": raw.stderr,
                "exitCode": raw.exit_code,
                "ranks": raw.ranks,
            })),
            errors: if raw.success {
                vec![]
            } else if raw.stderr.is_empty() {
                vec![format!(
                    "MPI job exited with code {:?}",
                    raw.exit_code
                )]
            } else {
                vec![raw.stderr.clone()]
            },
            metrics: JobMetrics {
                execution_time_ms: raw.execution_time_ms,
                workers_used: raw.ranks as usize,
                cpu_utilization: Some(0.0),
                speedup: Some(0.0),
                extra: serde_json::json!({ "ranks": raw.ranks }),
            },
            scheduler_metadata: serde_json::json!({ "plugin": MPI_PLUGIN_ID }),
            workers: vec![],
            artifacts: vec![],
            result_summary: Some(if raw.success {
                format!("MPI completed ({} ranks)", raw.ranks)
            } else {
                format!("MPI failed (exit {:?})", raw.exit_code)
            }),
        }
    }

    async fn execute_spec(&self, job_id: &str, spec: &JobSpec) -> SchedulerResult<JobResult> {
        {
            let mut jobs = self.jobs.write().await;
            if let Some(tracked) = jobs.get_mut(job_id) {
                tracked.status = JobStatus::Running;
            }
        }

        let raw = self
            .service
            .run_job(job_id, spec)
            .await
            .map_err(|e| SchedulerError::JobError(e.to_string()))?;
        let result = Self::map_result(job_id, raw);

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
impl SchedulerService for MpiSchedulerAdapter {
    fn plugin_id(&self) -> &str {
        MPI_PLUGIN_ID
    }

    fn display_name(&self) -> &str {
        "MPI"
    }

    async fn capabilities(&self) -> SchedulerCapabilities {
        let python_ready = self.service.toolchain().await.is_some()
            && self.service.is_ready().await;
        // Advertise Python when we might run mpi4py (actual check at submit).
        let _ = python_ready;
        SchedulerCapabilities {
            supports_python: true,
            supports_actors: false,
            supports_dags: false,
            supports_gpu: false,
            supports_fault_tolerance: false,
            supports_autoscaling: false,
            supports_streaming: false,
        }
    }

    async fn cluster_info(&self) -> SchedulerResult<SchedulerInfo> {
        let tc = self.service.toolchain().await;
        let ready = tc.is_some();
        Ok(SchedulerInfo {
            plugin_id: MPI_PLUGIN_ID.to_string(),
            display_name: "MPI".to_string(),
            health: if ready {
                "healthy".into()
            } else {
                "unavailable".into()
            },
            address: tc.map(|t| t.launcher),
            dashboard_url: None,
            worker_count: 0,
            total_cores: 0,
            client_connected: ready,
        })
    }

    async fn ensure_job_dependencies(&self, spec: &JobSpec) -> SchedulerResult<DependencyReport> {
        if is_python_mpi_entry(&spec.entry_point) {
            self.service
                .ensure_mpi4py()
                .await
                .map_err(|e| SchedulerError::JobError(e.to_string()))?;
            return Ok(DependencyReport {
                detected: vec!["mpi4py".into()],
                installed: vec!["mpi4py".into()],
                already_present: vec![],
                skipped_stdlib: vec![],
                ..Default::default()
            });
        }
        Ok(DependencyReport::default())
    }

    async fn submit(&self, job_id: &str, spec: &JobSpec) -> SchedulerResult<SubmitAck> {
        match &spec.entry_point {
            EntryPoint::MpiExecutable { .. }
            | EntryPoint::PythonScript { .. }
            | EntryPoint::PythonFunction { .. } => {}
            other => {
                return Err(SchedulerError::Unsupported(format!(
                    "MPI scheduler does not support this entry point: {:?}",
                    std::mem::discriminant(other)
                )));
            }
        }

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
        let this = MpiSchedulerAdapter {
            service: self.service.clone(),
            jobs: jobs.clone(),
        };

        match this.execute_spec(&job_id_owned, spec).await {
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
            .cancel_job(job_id)
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
                scheduler_id: MPI_PLUGIN_ID.to_string(),
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
                result_summary: tracked
                    .result
                    .as_ref()
                    .and_then(|r| r.result_summary.clone()),
            })
            .collect())
    }
}
