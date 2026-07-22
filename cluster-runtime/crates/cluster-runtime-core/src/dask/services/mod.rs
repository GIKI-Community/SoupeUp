use std::sync::Arc;
use tokio::sync::RwLock;

use crate::dask::client::ClientManager;
use crate::dask::dashboard::{dashboard_view, DashboardView};
use crate::dask::jobs::JobService;
use crate::dask::monitoring::{DaskMetrics, MonitoringService};
use crate::dask::scheduler::SchedulerManager;
use crate::dask::settings::DaskSettings;
use crate::dask::types::{
    ClusterSnapshot, ComponentStatus, DaskError, DaskResult, ExampleJobResult, JobResult,
    SchedulerInfo, WorkerInfo,
};
use crate::dask::worker::WorkerManager;
use crate::python_runtime::PythonExecutionService;

const REQUIRED_PACKAGES: &[&str] = &[
    "dask",
    "distributed",
    "cloudpickle",
    "msgpack",
    "psutil",
    "numpy",
];

/// Public service surface for the Dask Scheduler Plugin.
pub struct DaskService {
    python: Arc<PythonExecutionService>,
    settings: Arc<RwLock<DaskSettings>>,
    scheduler: Arc<SchedulerManager>,
    worker: Arc<WorkerManager>,
    client: Arc<ClientManager>,
    jobs: Arc<JobService>,
    monitoring: Arc<MonitoringService>,
    packages_ready: Arc<RwLock<bool>>,
}

impl DaskService {
    pub fn new(python: Arc<PythonExecutionService>) -> Self {
        let settings = Arc::new(RwLock::new(DaskSettings::default()));
        let scheduler = Arc::new(SchedulerManager::new(python.clone(), settings.clone()));
        let worker = Arc::new(WorkerManager::new(python.clone(), settings.clone()));
        let client = Arc::new(ClientManager::new(python.clone(), settings.clone()));
        let jobs = Arc::new(JobService::new(client.clone()));
        let monitoring = Arc::new(MonitoringService::new(
            scheduler.clone(),
            worker.clone(),
            client.clone(),
        ));

        Self {
            python,
            settings,
            scheduler,
            worker,
            client,
            jobs,
            monitoring,
            packages_ready: Arc::new(RwLock::new(false)),
        }
    }

    /// Ensure required packages are installed via the Python Runtime package manager.
    pub async fn ensure_packages(&self) -> DaskResult<Vec<String>> {
        self.ensure_packages_list(REQUIRED_PACKAGES).await
    }

    /// Install any missing packages from the given list (e.g. example-specific deps).
    pub async fn ensure_packages_list(&self, packages: &[&str]) -> DaskResult<Vec<String>> {
        let installed = self
            .python
            .list_packages()
            .await
            .map_err(|e| DaskError::PackageError(e.to_string()))?;

        let installed_names: std::collections::HashSet<String> = installed
            .iter()
            .map(|p| p.name.to_ascii_lowercase())
            .collect();

        let mut newly = Vec::new();
        for pkg in packages {
            if !installed_names.contains(&pkg.to_ascii_lowercase()) {
                log::info!("Dask plugin: installing missing package '{}'", pkg);
                self.python
                    .install_package(pkg, None)
                    .await
                    .map_err(|e| DaskError::PackageError(format!("{}: {}", pkg, e)))?;
                newly.push((*pkg).to_string());
            }
        }

        *self.packages_ready.write().await = true;
        Ok(newly)
    }

    /// Detect imports in the job source and install missing packages for workers.
    pub async fn ensure_job_dependencies(
        &self,
        spec: &crate::jobs::JobSpec,
    ) -> DaskResult<crate::jobs::DependencyReport> {
        crate::jobs::dependencies::resolve_and_install(&self.python, spec)
            .await
            .map_err(DaskError::PackageError)
    }

    pub async fn initialize(&self) -> DaskResult<()> {
        self.ensure_packages().await?;
        log::info!("Dask Scheduler Plugin initialized");
        Ok(())
    }

    // ─── Settings ─────────────────────────────────────────────────────────────

    pub async fn get_settings(&self) -> DaskSettings {
        self.settings.read().await.clone()
    }

    pub async fn update_settings(&self, settings: DaskSettings) -> DaskResult<DaskSettings> {
        *self.settings.write().await = settings.clone();
        Ok(settings)
    }

    // ─── Scheduler lifecycle ──────────────────────────────────────────────────

    pub async fn start_scheduler(&self) -> DaskResult<SchedulerInfo> {
        self.ensure_packages().await?;
        self.scheduler.start().await
    }

    pub async fn stop_scheduler(&self) -> DaskResult<SchedulerInfo> {
        self.scheduler.stop().await
    }

    pub async fn restart_scheduler(&self) -> DaskResult<SchedulerInfo> {
        self.ensure_packages().await?;
        self.scheduler.restart().await
    }

    pub async fn scheduler_status(&self) -> SchedulerInfo {
        self.scheduler.status().await
    }

    pub async fn scheduler_health(&self) -> ComponentStatus {
        self.scheduler.health().await
    }

    // ─── Worker lifecycle ─────────────────────────────────────────────────────

    pub async fn start_worker(&self, scheduler_address: Option<String>) -> DaskResult<WorkerInfo> {
        self.ensure_packages().await?;
        // Persist address into settings when provided.
        if let Some(ref addr) = scheduler_address {
            if !addr.trim().is_empty() {
                self.settings.write().await.scheduler_address = addr.clone();
            }
        }
        self.worker.start(scheduler_address).await
    }

    pub async fn stop_worker(&self) -> DaskResult<WorkerInfo> {
        self.worker.stop().await
    }

    pub async fn restart_worker(&self) -> DaskResult<WorkerInfo> {
        self.ensure_packages().await?;
        self.worker.restart().await
    }

    pub async fn worker_status(&self) -> WorkerInfo {
        self.worker.status().await
    }

    pub async fn worker_health(&self) -> ComponentStatus {
        self.worker.health().await
    }

    // ─── Client ───────────────────────────────────────────────────────────────

    pub async fn connect_client(&self, address: Option<String>) -> DaskResult<String> {
        self.ensure_packages().await?;
        self.client.connect(address).await
    }

    pub async fn disconnect_client(&self) -> DaskResult<()> {
        self.client.disconnect().await
    }

    pub async fn cluster_info(&self) -> DaskResult<serde_json::Value> {
        self.client.cluster_info().await
    }

    // ─── Jobs ─────────────────────────────────────────────────────────────────

    pub async fn submit_python_function(
        &self,
        function_body: String,
        args: serde_json::Value,
    ) -> DaskResult<JobResult> {
        self.ensure_packages().await?;
        self.jobs.submit_python_function(function_body, args).await
    }

    pub async fn submit_script(&self, script: String) -> DaskResult<JobResult> {
        self.ensure_packages().await?;
        self.jobs.submit_script(script).await
    }

    pub async fn submit_module(&self, module: String) -> DaskResult<JobResult> {
        self.ensure_packages().await?;
        self.jobs.submit_module(module).await
    }

    pub async fn map(
        &self,
        function_body: String,
        items: serde_json::Value,
    ) -> DaskResult<JobResult> {
        self.ensure_packages().await?;
        self.jobs.map(function_body, items).await
    }

    pub async fn scatter(&self, data: serde_json::Value) -> DaskResult<JobResult> {
        self.jobs.scatter(data).await
    }

    pub async fn gather(&self, keys: serde_json::Value) -> DaskResult<JobResult> {
        self.jobs.gather(keys).await
    }

    pub async fn cancel_job(&self, job_id: String) -> DaskResult<()> {
        self.jobs.cancel_job(job_id).await
    }

    pub async fn job_status(&self, job_id: String) -> DaskResult<serde_json::Value> {
        self.jobs.job_status(job_id).await
    }

    pub async fn run_example(&self, example_id: String) -> DaskResult<ExampleJobResult> {
        if let Err(e) = self.ensure_packages().await {
            return Ok(example_failure(&example_id, e.to_string()));
        }
        let extra = crate::dask::examples::packages_for(&example_id);
        if !extra.is_empty() {
            if let Err(e) = self.ensure_packages_list(extra).await {
                return Ok(example_failure(&example_id, e.to_string()));
            }
        }
        if let Err(e) = self.ensure_cluster_ready().await {
            return Ok(example_failure(&example_id, e.to_string()));
        }
        self.jobs.run_example(&example_id).await
    }

    async fn ensure_cluster_ready(&self) -> DaskResult<()> {
        let sched = self.scheduler.status().await;
        if sched.status != ComponentStatus::Running {
            return Err(DaskError::SchedulerError(
                "Start the Dask scheduler first (Cluster page → Start Scheduler).".into(),
            ));
        }

        let snap = self.monitoring.snapshot().await?;
        if snap.workers.is_empty() {
            return Err(DaskError::WorkerError(
                "No workers are connected. Start at least one worker (Cluster page → Start Worker)."
                    .into(),
            ));
        }
        Ok(())
    }

    // ─── Monitoring / Dashboard ───────────────────────────────────────────────

    pub async fn cluster_snapshot(&self) -> DaskResult<ClusterSnapshot> {
        self.monitoring.snapshot().await
    }

    pub async fn metrics(&self) -> DaskResult<DaskMetrics> {
        self.monitoring.metrics().await
    }

    pub async fn dashboard(&self) -> DashboardView {
        let settings = self.settings.read().await;
        dashboard_view(&settings)
    }

    /// Stop worker, scheduler, and disconnect client.
    /// Remaining background processes are swept by PythonExecutionService::shutdown.
    pub async fn shutdown(&self) {
        log::info!("Dask Scheduler Plugin: shutting down...");
        let _ = self.client.disconnect().await;
        let _ = self.worker.stop().await;
        let _ = self.scheduler.stop().await;
    }
}

pub fn example_failure(example_id: &str, message: String) -> ExampleJobResult {
    let title = crate::dask::examples::get(example_id)
        .map(|spec| spec.title.to_string())
        .unwrap_or_else(|| "Example Job".to_string());
    ExampleJobResult {
        example_id: example_id.to_string(),
        title,
        success: false,
        execution_time_ms: 0,
        workers_used: 0,
        cpu_utilization: None,
        speedup: None,
        result_summary: String::new(),
        details: None,
        error: Some(message),
    }
}
