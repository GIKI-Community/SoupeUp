use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ray::client::ClientManager;
use crate::ray::dashboard::{dashboard_view, DashboardView};
use crate::ray::head::HeadManager;
use crate::ray::jobs::JobService;
use crate::ray::monitoring::{MonitoringService, RayMetrics};
use crate::ray::settings::RaySettings;
use crate::ray::types::{
    ClusterSnapshot, ComponentStatus, ExampleJobResult, HeadInfo, JobResult, RayError,
    RayResult, WorkerInfo,
};
use crate::ray::worker::WorkerManager;
use crate::python_runtime::PythonExecutionService;

const REQUIRED_PACKAGES: &[&str] = &["ray", "numpy"];

pub struct RayService {
    python: Arc<PythonExecutionService>,
    settings: Arc<RwLock<RaySettings>>,
    head: Arc<HeadManager>,
    worker: Arc<WorkerManager>,
    client: Arc<ClientManager>,
    jobs: Arc<JobService>,
    monitoring: Arc<MonitoringService>,
    packages_ready: Arc<RwLock<bool>>,
}

impl RayService {
    pub fn new(python: Arc<PythonExecutionService>) -> Self {
        let settings = Arc::new(RwLock::new(RaySettings::default()));
        let head = Arc::new(HeadManager::new(python.clone(), settings.clone()));
        let worker = Arc::new(WorkerManager::new(python.clone(), settings.clone()));
        let client = Arc::new(ClientManager::new(python.clone(), settings.clone()));
        let jobs = Arc::new(JobService::new(client.clone()));
        let monitoring = Arc::new(MonitoringService::new(
            head.clone(),
            worker.clone(),
            client.clone(),
        ));

        Self {
            python,
            settings,
            head,
            worker,
            client,
            jobs,
            monitoring,
            packages_ready: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn ensure_packages(&self) -> RayResult<Vec<String>> {
        self.ensure_packages_list(REQUIRED_PACKAGES).await
    }

    pub async fn ensure_packages_list(&self, packages: &[&str]) -> RayResult<Vec<String>> {
        if let Some(msg) = self.unsupported_python_hint().await {
            return Err(RayError::PackageError(msg));
        }

        let installed = self
            .python
            .list_packages()
            .await
            .map_err(|e| RayError::PackageError(e.to_string()))?;

        let installed_names: std::collections::HashSet<String> = installed
            .iter()
            .map(|p| p.name.to_ascii_lowercase())
            .collect();

        let mut newly = Vec::new();
        for pkg in packages {
            if !installed_names.contains(&pkg.to_ascii_lowercase()) {
                log::info!("Ray plugin: installing missing package '{}'", pkg);
                self.python
                    .install_package(pkg, None)
                    .await
                    .map_err(|e| RayError::PackageError(format!("{}: {}", pkg, e)))?;
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
    ) -> RayResult<crate::jobs::DependencyReport> {
        crate::jobs::dependencies::resolve_and_install(&self.python, spec)
            .await
            .map_err(RayError::PackageError)
    }

    /// Ray Windows wheels do not support Python 3.13+.
    async fn unsupported_python_hint(&self) -> Option<String> {
        let version = self.python.python_version().await.ok()?;
        let major_minor = version
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".");
        if cfg!(windows) && major_minor.as_str() >= "3.13" {
            Some(format!(
                "Ray is not installable on Windows with Python {version}. \
                 Re-stage the bundled runtime: \
                 `scripts/Setup-PythonRuntime.ps1 -Force`, then restart Cluster Runtime."
            ))
        } else {
            None
        }
    }

    pub async fn initialize(&self) -> RayResult<()> {
        self.ensure_packages().await?;
        log::info!("Ray Plugin initialized");
        Ok(())
    }

    pub async fn get_settings(&self) -> RaySettings {
        self.settings.read().await.clone()
    }

    pub async fn update_settings(&self, settings: RaySettings) -> RayResult<RaySettings> {
        *self.settings.write().await = settings.clone();
        Ok(settings)
    }

    pub async fn start_head(&self) -> RayResult<HeadInfo> {
        self.ensure_packages().await?;
        self.head.start().await
    }

    pub async fn stop_head(&self) -> RayResult<HeadInfo> {
        self.head.stop().await
    }

    pub async fn restart_head(&self) -> RayResult<HeadInfo> {
        self.ensure_packages().await?;
        self.head.restart().await
    }

    pub async fn head_status(&self) -> HeadInfo {
        self.head.status().await
    }

    pub async fn start_worker(&self, head_address: Option<String>) -> RayResult<WorkerInfo> {
        self.ensure_packages().await?;
        if let Some(ref addr) = head_address {
            if !addr.trim().is_empty() {
                self.settings.write().await.head_address = addr.clone();
            }
        }
        self.worker.start(head_address).await
    }

    pub async fn stop_worker(&self) -> RayResult<WorkerInfo> {
        self.worker.stop().await
    }

    pub async fn restart_worker(&self) -> RayResult<WorkerInfo> {
        self.ensure_packages().await?;
        self.worker.restart().await
    }

    pub async fn worker_status(&self) -> WorkerInfo {
        self.worker.status().await
    }

    pub async fn connect_client(&self, address: Option<String>) -> RayResult<String> {
        self.ensure_packages().await?;
        self.client.connect(address).await
    }

    pub async fn disconnect_client(&self) -> RayResult<()> {
        self.client.disconnect().await
    }

    pub async fn cluster_info(&self) -> RayResult<serde_json::Value> {
        self.client.cluster_info().await
    }

    pub async fn submit_python_function(
        &self,
        function_body: String,
        args: serde_json::Value,
    ) -> RayResult<JobResult> {
        self.ensure_packages().await?;
        self.jobs.submit_python_function(function_body, args).await
    }

    pub async fn submit_script(&self, script: String) -> RayResult<JobResult> {
        self.ensure_packages().await?;
        self.jobs.submit_script(script).await
    }

    pub async fn submit_module(&self, module: String) -> RayResult<JobResult> {
        self.ensure_packages().await?;
        self.jobs.submit_module(module).await
    }

    pub async fn map(
        &self,
        function_body: String,
        items: serde_json::Value,
    ) -> RayResult<JobResult> {
        self.ensure_packages().await?;
        self.jobs.map(function_body, items).await
    }

    pub async fn scatter(&self, data: serde_json::Value) -> RayResult<JobResult> {
        self.jobs.scatter(data).await
    }

    pub async fn gather(&self, keys: serde_json::Value) -> RayResult<JobResult> {
        self.jobs.gather(keys).await
    }

    pub async fn cancel_job(&self, job_id: String) -> RayResult<()> {
        self.jobs.cancel_job(job_id).await
    }

    pub async fn job_status(&self, job_id: String) -> RayResult<serde_json::Value> {
        self.jobs.job_status(job_id).await
    }

    pub async fn run_example(&self, example_id: String) -> RayResult<ExampleJobResult> {
        if let Err(e) = self.ensure_packages().await {
            return Ok(example_failure(&example_id, e.to_string()));
        }
        let extra = crate::ray::examples::packages_for(&example_id);
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

    async fn ensure_cluster_ready(&self) -> RayResult<()> {
        let head = self.head.status().await;
        if head.status != ComponentStatus::Running {
            return Err(RayError::HeadError(
                "Start the Ray head first (Cluster page → Ray → Start Head).".into(),
            ));
        }

        let snap = self.monitoring.snapshot().await?;
        if snap.workers.is_empty() {
            return Err(RayError::WorkerError(
                "No workers are connected. Start at least one worker (Cluster page → Ray → Start Worker)."
                    .into(),
            ));
        }
        Ok(())
    }

    pub async fn cluster_snapshot(&self) -> RayResult<ClusterSnapshot> {
        self.monitoring.snapshot().await
    }

    pub async fn metrics(&self) -> RayResult<RayMetrics> {
        self.monitoring.metrics().await
    }

    pub async fn dashboard(&self) -> DashboardView {
        let settings = self.settings.read().await;
        dashboard_view(&settings)
    }

    pub async fn shutdown(&self) {
        log::info!("Ray Plugin: shutting down...");
        let _ = self.client.disconnect().await;
        // Tear down the Ray cluster before killing wrapper processes.
        self.python.ray_stop_force().await;
        let _ = self.worker.stop().await;
        // Cluster already stopped — only kill the wrapper process.
        let _ = self.head.stop_inner(false).await;
    }
}

pub fn example_failure(example_id: &str, message: String) -> ExampleJobResult {
    let title = crate::ray::examples::get(example_id)
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
