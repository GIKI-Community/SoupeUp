use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::events::{ClusterEvent, EventBus};
use crate::jobs::history::JobHistoryStore;
use crate::jobs::models::{
    DependencyReport, Job, JobDetail, JobMetrics, JobProgress, JobResult, JobSpec, JobStatus,
    JobSummary, SubmitAck,
};
use crate::jobs::progress::ProgressTracker;
use crate::jobs::queue::{transition, JobQueue};
use crate::jobs::resources::validate_job;
use crate::jobs::results::ResultStore;
use crate::scheduler::selection::SchedulerRegistry;

pub struct JobManager {
    registry: Arc<SchedulerRegistry>,
    history: Arc<JobHistoryStore>,
    results: Arc<ResultStore>,
    progress: Arc<ProgressTracker>,
    queue: tokio::sync::Mutex<JobQueue>,
    event_bus: Arc<EventBus>,
}

impl JobManager {
    pub fn new(
        registry: Arc<SchedulerRegistry>,
        history: Arc<JobHistoryStore>,
        results: Arc<ResultStore>,
        progress: Arc<ProgressTracker>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            registry,
            history,
            results,
            progress,
            queue: tokio::sync::Mutex::new(JobQueue::new()),
            event_bus,
        }
    }

    pub async fn submit(&self, spec: JobSpec, owner: &str) -> Result<SubmitAck, String> {
        let scheduler = self
            .registry
            .active()
            .await
            .map_err(|e| e.to_string())?;
        let scheduler_id = scheduler.plugin_id().to_string();
        let capabilities = scheduler.capabilities().await;

        let warnings = validate_job(&spec, &capabilities);
        for warning in &warnings {
            log::warn!("Job validation: {}", warning);
        }

        let job_id = format!("job-{}", Uuid::new_v4());

        // Detect imports and install missing packages before dispatch (fail-fast).
        let dep_report = match scheduler.ensure_job_dependencies(&spec).await {
            Ok(report) => report,
            Err(e) => {
                let err_msg = e.to_string();
                let job = Job {
                    id: job_id.clone(),
                    name: spec.name.clone(),
                    description: spec.description.clone(),
                    entry_point: spec.entry_point.clone(),
                    args: spec.args.clone(),
                    env: spec.env.clone(),
                    resources: spec.resources.clone(),
                    priority: spec.priority,
                    timeout_secs: spec.timeout_secs,
                    retry_policy: spec.retry_policy.clone(),
                    tags: spec.tags.clone(),
                    metadata: spec.metadata.clone(),
                    status: JobStatus::Failed,
                    scheduler_id: scheduler_id.clone(),
                    owner: owner.to_string(),
                    submitted_at: Utc::now(),
                    started_at: None,
                    finished_at: Some(Utc::now()),
                    duration_secs: 0,
                    execution_context: spec.execution_context.clone(),
                    dependencies: None,
                };
                self.history.create(job, spec.clone()).await;
                let failed = JobResult {
                    job_id: job_id.clone(),
                    status: JobStatus::Failed,
                    output: None,
                    errors: vec![err_msg.clone()],
                    metrics: JobMetrics::default(),
                    scheduler_metadata: serde_json::Value::Null,
                    workers: vec![],
                    artifacts: vec![],
                    result_summary: Some(format!("Dependency resolution failed: {err_msg}")),
                };
                self.results.put(failed.clone()).await;
                self.history.finish(&job_id, &failed).await;
                self.history
                    .append_log(
                        &job_id,
                        &format!("Dependency install failed before dispatch: {err_msg}"),
                    )
                    .await;
                self.event_bus.publish(ClusterEvent::JobFinished {
                    job_id: job_id.clone(),
                    success: false,
                });
                return Err(err_msg);
            }
        };

        let mut job = Job {
            id: job_id.clone(),
            name: spec.name.clone(),
            description: spec.description.clone(),
            entry_point: spec.entry_point.clone(),
            args: spec.args.clone(),
            env: spec.env.clone(),
            resources: spec.resources.clone(),
            priority: spec.priority,
            timeout_secs: spec.timeout_secs,
            retry_policy: spec.retry_policy.clone(),
            tags: spec.tags.clone(),
            metadata: spec.metadata.clone(),
            status: JobStatus::Created,
            scheduler_id: scheduler_id.clone(),
            owner: owner.to_string(),
            submitted_at: Utc::now(),
            started_at: None,
            finished_at: None,
            duration_secs: 0,
            execution_context: spec.execution_context.clone(),
            dependencies: Some(dep_report.clone()),
        };

        self.history.create(job.clone(), spec.clone()).await;
        self.append_dependency_log(&job_id, &dep_report).await;
        transition(&mut job, JobStatus::Queued);
        self.history.update_status(&job_id, JobStatus::Queued).await;
        self.queue.lock().await.enqueue(&job_id);

        self.event_bus
            .publish(ClusterEvent::JobStarted { job_id: job_id.clone() });

        transition(&mut job, JobStatus::Scheduling);
        self.history
            .update_status(&job_id, JobStatus::Scheduling)
            .await;
        self.history
            .append_log(&job_id, &format!("Dispatching to scheduler {}", scheduler_id))
            .await;

        self.history.mark_started(&job_id).await;
        self.progress
            .update_for_status(&job_id, &JobStatus::Running)
            .await;

        let ack = scheduler
            .submit(&job_id, &spec)
            .await
            .map_err(|e| e.to_string())?;

        let result = scheduler.result(&job_id).await.map_err(|e| e.to_string())?;
        self.results.put(result.clone()).await;
        self.history.finish(&job_id, &result).await;
        self.progress.complete(&job_id, result.status == JobStatus::Completed).await;

        self.queue.lock().await.remove(&job_id);

        self.event_bus.publish(ClusterEvent::JobFinished {
            job_id: job_id.clone(),
            success: result.status == JobStatus::Completed,
        });

        Ok(ack)
    }

    async fn append_dependency_log(&self, job_id: &str, report: &DependencyReport) {
        if report.detected.is_empty()
            && report.installed.is_empty()
            && report.already_present.is_empty()
            && report.skipped_stdlib.is_empty()
        {
            return;
        }
        let mut parts = Vec::new();
        if !report.detected.is_empty() {
            parts.push(format!("detected=[{}]", report.detected.join(", ")));
        }
        if !report.installed.is_empty() {
            parts.push(format!("installed=[{}]", report.installed.join(", ")));
        }
        if !report.already_present.is_empty() {
            parts.push(format!(
                "already_present=[{}]",
                report.already_present.join(", ")
            ));
        }
        if !report.skipped_stdlib.is_empty() {
            parts.push(format!(
                "skipped_stdlib=[{}]",
                report.skipped_stdlib.join(", ")
            ));
        }
        self.history
            .append_log(
                job_id,
                &format!("Dependencies resolved: {}", parts.join("; ")),
            )
            .await;
    }

    pub async fn cancel(&self, job_id: &str) -> Result<(), String> {
        let job = self
            .history
            .get(job_id)
            .await
            .ok_or_else(|| format!("Job '{}' not found", job_id))?;
        let scheduler = self.registry.get(&job.0.scheduler_id).await.map_err(|e| e.to_string())?;
        scheduler.cancel(job_id).await.map_err(|e| e.to_string())?;
        self.history
            .update_status(job_id, JobStatus::Cancelled)
            .await;
        self.queue.lock().await.remove(job_id);
        Ok(())
    }

    pub async fn status(&self, job_id: &str) -> Result<JobStatus, String> {
        let (job, _, _) = self
            .history
            .get(job_id)
            .await
            .ok_or_else(|| format!("Job '{}' not found", job_id))?;
        Ok(job.status)
    }

    pub async fn progress(&self, job_id: &str) -> Result<JobProgress, String> {
        let stored = self.progress.get(job_id).await;
        if stored.percent > 0.0 {
            return Ok(stored);
        }
        let (job, _, _) = self
            .history
            .get(job_id)
            .await
            .ok_or_else(|| format!("Job '{}' not found", job_id))?;
        let scheduler = self.registry.get(&job.scheduler_id).await.map_err(|e| e.to_string())?;
        scheduler.progress(job_id).await.map_err(|e| e.to_string())
    }

    pub async fn result(&self, job_id: &str) -> Result<JobResult, String> {
        if let Some(result) = self.results.get(job_id).await {
            return Ok(result);
        }
        let (job, _, _) = self
            .history
            .get(job_id)
            .await
            .ok_or_else(|| format!("Job '{}' not found", job_id))?;
        let scheduler = self.registry.get(&job.scheduler_id).await.map_err(|e| e.to_string())?;
        scheduler.result(job_id).await.map_err(|e| e.to_string())
    }

    pub async fn list(&self) -> Vec<Job> {
        self.history.list().await
    }

    pub async fn get(&self, job_id: &str) -> Result<JobDetail, String> {
        let (job, logs, _) = self
            .history
            .get(job_id)
            .await
            .ok_or_else(|| format!("Job '{}' not found", job_id))?;
        let progress = self.progress.get(job_id).await;
        let result = self.results.get(job_id).await;
        Ok(JobDetail {
            job,
            progress,
            result,
            logs,
        })
    }

    pub async fn retry(&self, job_id: &str) -> Result<SubmitAck, String> {
        let spec = self
            .history
            .get_spec(job_id)
            .await
            .ok_or_else(|| format!("Job '{}' not found or has no spec", job_id))?;
        let (job, _, _) = self.history.get(job_id).await.unwrap();
        self.submit(spec, &job.owner).await
    }

    pub async fn summaries(&self) -> Vec<JobSummary> {
        let jobs = self.history.list().await;
        let mut summaries = Vec::new();
        for job in jobs {
            let job_id = job.id.clone();
            let progress = self.progress.get(&job_id).await;
            summaries.push(JobSummary {
                id: job.id,
                name: job.name,
                status: job.status,
                scheduler_id: job.scheduler_id,
                submitted_at: job.submitted_at,
                duration_secs: job.duration_secs,
                progress_percent: progress.percent,
                result_summary: self
                    .results
                    .get(&job_id)
                    .await
                    .and_then(|r| r.result_summary),
            });
        }
        summaries
    }

    pub async fn load_persisted(&self) {
        self.results.load().await;
    }
}
