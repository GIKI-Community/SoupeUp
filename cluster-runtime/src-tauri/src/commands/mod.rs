use crate::core::{mock_activity, mock_system_info, mock_system_status, ActivityEntry, SystemInfo, SystemStatus};
use crate::logging::{mock_logs, LogEntry};
use crate::metrics::{mock_metrics, MetricsSnapshot};
use crate::nodes::{mock_nodes};
use crate::network::{ClusterSummary, PeerInfo};
use crate::plugin_registry::PluginInfo;
use crate::python_runtime::types::{
    EnvironmentInfo, ExecutionContext, ExecutionResult, PackageInfo, PythonRuntimeHealth,
};

// ─── System Commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    mock_system_info()
}

#[tauri::command]
pub fn get_system_status() -> SystemStatus {
    mock_system_status()
}

#[tauri::command]
pub fn get_activity() -> Vec<ActivityEntry> {
    mock_activity()
}

#[tauri::command]
pub async fn get_nodes(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<crate::nodes::Node>, String> {
    let mut nodes = Vec::new();

    if let Some(svc) = state.dask_service.read().await.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            nodes.extend(crate::nodes::nodes_from_dask_snapshot(&snap));
        }
    }

    if let Some(svc) = state.ray_service.read().await.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            nodes.extend(crate::nodes::nodes_from_ray_snapshot(&snap));
        }
    }

    if nodes.is_empty() {
        Ok(mock_nodes())
    } else {
        Ok(nodes)
    }
}

#[tauri::command]
pub async fn get_jobs(state: tauri::State<'_, crate::AppState>) -> Result<Vec<crate::jobs::Job>, String> {
    Ok(state.job_manager.list().await)
}

#[tauri::command]
pub async fn get_cluster_summary(
    state: tauri::State<'_, crate::AppState>,
) -> Result<ClusterSummary, String> {
    let mut summary = ClusterSummary {
        total_nodes: 0,
        online_nodes: 0,
        total_cpus: 0,
        total_ram: 0,
        total_gpus: 0,
        total_workers: 0,
        total_available_compute: 0.0,
    };

    if let Some(svc) = state.dask_service.read().await.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            use crate::dask::ComponentStatus;
            let scheduler_up = snap.scheduler.status == ComponentStatus::Running;
            summary.total_nodes += snap.workers.len().max(1);
            summary.online_nodes += if scheduler_up {
                snap.workers.len().max(1)
            } else {
                snap.workers.len()
            };
            summary.total_cpus += snap.total_cores;
            summary.total_ram += snap.total_memory;
            summary.total_workers += snap.workers.len();
            summary.total_available_compute += snap.total_cores as f32;
        }
    }

    if let Some(svc) = state.ray_service.read().await.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            use crate::ray::ComponentStatus;
            let head_up = snap.head.status == ComponentStatus::Running;
            summary.total_nodes += snap.workers.len().max(1);
            summary.online_nodes += if head_up {
                snap.workers.len().max(1)
            } else {
                snap.workers.len()
            };
            summary.total_cpus += snap.total_cores;
            summary.total_ram += snap.total_memory;
            summary.total_workers += snap.workers.len();
            summary.total_available_compute += snap.total_cores as f32;
        }
    }

    Ok(summary)
}

#[tauri::command]
pub async fn get_cluster_peers(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<PeerInfo>, String> {
    if let Some(p2p) = state.p2p_service.read().await.clone() {
        return p2p.list_peers().await;
    }
    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_plugins(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<PluginInfo>, String> {
    let registry = state.plugin_registry.read().await;
    Ok(registry.list_plugins())
}

#[tauri::command]
pub fn get_metrics() -> MetricsSnapshot {
    mock_metrics()
}

#[tauri::command]
pub fn get_logs() -> Vec<LogEntry> {
    mock_logs()
}

// ─── Python Runtime Commands ──────────────────────────────────────────────────
//
// All Python commands acquire the service from AppState.  If the runtime hasn't
// finished initializing yet they return an informative error string.

/// Resolve the Python service or return a clear user-facing error.
macro_rules! python_service {
    ($state:expr) => {{
        let lock = $state.python_service.read().await;
        match lock.as_ref() {
            Some(svc) => std::sync::Arc::clone(svc),
            None => {
                return Err(
                    "Python Runtime is still initializing. Please wait a moment and try again."
                        .to_string(),
                )
            }
        }
    }};
}

/// Execute a Python code string and return a structured result.
#[tauri::command]
pub async fn python_execute_code(
    state: tauri::State<'_, crate::AppState>,
    code: String,
    context: Option<ExecutionContext>,
) -> Result<ExecutionResult, String> {
    let svc = python_service!(state);
    svc.execute_code(&code, context)
        .await
        .map_err(|e| e.to_string())
}

/// Execute a Python script at the given filesystem path.
#[tauri::command]
pub async fn python_execute_script(
    state: tauri::State<'_, crate::AppState>,
    script_path: String,
    context: Option<ExecutionContext>,
) -> Result<ExecutionResult, String> {
    let svc = python_service!(state);
    svc.execute_script(&script_path, context)
        .await
        .map_err(|e| e.to_string())
}

/// Run `python -m <module>` inside the active managed environment.
#[tauri::command]
pub async fn python_execute_module(
    state: tauri::State<'_, crate::AppState>,
    module: String,
    context: Option<ExecutionContext>,
) -> Result<ExecutionResult, String> {
    let svc = python_service!(state);
    svc.execute_module(&module, context)
        .await
        .map_err(|e| e.to_string())
}

/// Install a pip package into the active environment.
#[tauri::command]
pub async fn python_install_package(
    state: tauri::State<'_, crate::AppState>,
    package: String,
    version: Option<String>,
) -> Result<PackageInfo, String> {
    let svc = python_service!(state);
    svc.install_package(&package, version.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Uninstall a pip package from the active environment.
#[tauri::command]
pub async fn python_uninstall_package(
    state: tauri::State<'_, crate::AppState>,
    package: String,
) -> Result<(), String> {
    let svc = python_service!(state);
    svc.uninstall_package(&package)
        .await
        .map_err(|e| e.to_string())
}

/// List all packages installed in the active environment.
#[tauri::command]
pub async fn python_list_packages(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<PackageInfo>, String> {
    let svc = python_service!(state);
    svc.list_packages().await.map_err(|e| e.to_string())
}

/// Create a new isolated virtual environment.
#[tauri::command]
pub async fn python_create_environment(
    state: tauri::State<'_, crate::AppState>,
    name: String,
) -> Result<EnvironmentInfo, String> {
    let svc = python_service!(state);
    svc.create_environment(&name)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a managed virtual environment.
#[tauri::command]
pub async fn python_delete_environment(
    state: tauri::State<'_, crate::AppState>,
    name: String,
) -> Result<(), String> {
    let svc = python_service!(state);
    svc.delete_environment(&name)
        .await
        .map_err(|e| e.to_string())
}

/// Switch the active virtual environment.
#[tauri::command]
pub async fn python_activate_environment(
    state: tauri::State<'_, crate::AppState>,
    name: String,
) -> Result<(), String> {
    let svc = python_service!(state);
    svc.activate_environment(&name)
        .await
        .map_err(|e| e.to_string())
}

/// List all managed virtual environments.
#[tauri::command]
pub async fn python_list_environments(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<EnvironmentInfo>, String> {
    let svc = python_service!(state);
    svc.list_environments().await.map_err(|e| e.to_string())
}

/// Get a health report for the Python Runtime.
#[tauri::command]
pub async fn python_runtime_health(
    state: tauri::State<'_, crate::AppState>,
) -> Result<PythonRuntimeHealth, String> {
    let lock = state.python_service.read().await;
    match lock.as_ref() {
        Some(svc) => svc.runtime_health().await.map_err(|e| e.to_string()),
        None => Ok(PythonRuntimeHealth {
            status: crate::python_runtime::types::RuntimeStatus::Initializing,
            python_version: None,
            active_environment: None,
            environment_path: None,
            interpreter_path: None,
            is_bundled: false,
        }),
    }
}

/// Return the Python version string used by the active interpreter.
#[tauri::command]
pub async fn python_version(
    state: tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    let svc = python_service!(state);
    svc.python_version().await.map_err(|e| e.to_string())
}

/// Return the configured pip package index URL.
#[tauri::command]
pub async fn python_package_index(
    state: tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    let svc = python_service!(state);
    Ok(svc.package_index().await)
}

/// Update the package index URL used for pip installs.
#[tauri::command]
pub async fn python_set_package_index(
    state: tauri::State<'_, crate::AppState>,
    index_url: String,
) -> Result<(), String> {
    let svc = python_service!(state);
    svc.set_package_index(index_url).await;
    Ok(())
}

// ─── Dask Scheduler Plugin Commands ───────────────────────────────────────────

macro_rules! dask_service {
    ($state:expr) => {{
        let lock = $state.dask_service.read().await;
        match lock.as_ref() {
            Some(svc) => std::sync::Arc::clone(svc),
            None => {
                return Err(
                    "Dask Scheduler Plugin is still initializing. Ensure the Python Runtime is ready."
                        .to_string(),
                )
            }
        }
    }};
}

#[tauri::command]
pub async fn dask_ensure_packages(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    let svc = dask_service!(state);
    svc.ensure_packages().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_get_settings(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::settings::DaskSettings, String> {
    let svc = dask_service!(state);
    Ok(svc.get_settings().await)
}

#[tauri::command]
pub async fn dask_update_settings(
    state: tauri::State<'_, crate::AppState>,
    settings: crate::dask::settings::DaskSettings,
) -> Result<crate::dask::settings::DaskSettings, String> {
    let svc = dask_service!(state);
    svc.update_settings(settings)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_start_scheduler(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::SchedulerInfo, String> {
    let svc = dask_service!(state);
    svc.start_scheduler().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_stop_scheduler(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::SchedulerInfo, String> {
    let svc = dask_service!(state);
    svc.stop_scheduler().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_restart_scheduler(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::SchedulerInfo, String> {
    let svc = dask_service!(state);
    svc.restart_scheduler().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_scheduler_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::SchedulerInfo, String> {
    let svc = dask_service!(state);
    Ok(svc.scheduler_status().await)
}

#[tauri::command]
pub async fn dask_start_worker(
    state: tauri::State<'_, crate::AppState>,
    scheduler_address: Option<String>,
) -> Result<crate::dask::WorkerInfo, String> {
    let svc = dask_service!(state);
    svc.start_worker(scheduler_address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_stop_worker(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::WorkerInfo, String> {
    let svc = dask_service!(state);
    svc.stop_worker().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_restart_worker(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::WorkerInfo, String> {
    let svc = dask_service!(state);
    svc.restart_worker().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_worker_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::WorkerInfo, String> {
    let svc = dask_service!(state);
    Ok(svc.worker_status().await)
}

#[tauri::command]
pub async fn dask_connect_client(
    state: tauri::State<'_, crate::AppState>,
    address: Option<String>,
) -> Result<String, String> {
    let svc = dask_service!(state);
    svc.connect_client(address).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_disconnect_client(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let svc = dask_service!(state);
    svc.disconnect_client().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_cluster_snapshot(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::ClusterSnapshot, String> {
    let svc = dask_service!(state);
    svc.cluster_snapshot().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_cluster_info(
    state: tauri::State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    let svc = dask_service!(state);
    svc.cluster_info().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_dashboard(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::dashboard::DashboardView, String> {
    let svc = dask_service!(state);
    Ok(svc.dashboard().await)
}

#[tauri::command]
pub async fn dask_metrics(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::dask::monitoring::DaskMetrics, String> {
    let svc = dask_service!(state);
    svc.metrics().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_submit_python_function(
    state: tauri::State<'_, crate::AppState>,
    function_body: String,
    args: serde_json::Value,
) -> Result<crate::dask::JobResult, String> {
    let svc = dask_service!(state);
    svc.submit_python_function(function_body, args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_map(
    state: tauri::State<'_, crate::AppState>,
    function_body: String,
    items: serde_json::Value,
) -> Result<crate::dask::JobResult, String> {
    let svc = dask_service!(state);
    svc.map(function_body, items)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_run_example(
    state: tauri::State<'_, crate::AppState>,
    example_id: String,
) -> Result<crate::dask::ExampleJobResult, String> {
    let _ = state
        .scheduler_registry
        .set_active(crate::scheduler::selection::DASK_PLUGIN_ID)
        .await;

    let title = crate::jobs::examples::get(&example_id)
        .map(|spec| spec.title.to_string())
        .unwrap_or_else(|| example_id.clone());

    let spec = crate::jobs::JobSpec::example(&title, &example_id);
    let ack = state.job_manager.submit(spec, "dask-example").await?;
    let result = state.job_manager.result(&ack.job_id).await?;
    Ok(example_result_from_job(&example_id, &title, &result))
}

#[tauri::command]
pub async fn dask_cancel_job(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<(), String> {
    let svc = dask_service!(state);
    svc.cancel_job(job_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_submit_script(
    state: tauri::State<'_, crate::AppState>,
    script: String,
) -> Result<crate::dask::JobResult, String> {
    let svc = dask_service!(state);
    svc.submit_script(script).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_submit_module(
    state: tauri::State<'_, crate::AppState>,
    module: String,
) -> Result<crate::dask::JobResult, String> {
    let svc = dask_service!(state);
    svc.submit_module(module).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_scatter(
    state: tauri::State<'_, crate::AppState>,
    data: serde_json::Value,
) -> Result<crate::dask::JobResult, String> {
    let svc = dask_service!(state);
    svc.scatter(data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_gather(
    state: tauri::State<'_, crate::AppState>,
    keys: serde_json::Value,
) -> Result<crate::dask::JobResult, String> {
    let svc = dask_service!(state);
    svc.gather(keys).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dask_job_status(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let svc = dask_service!(state);
    svc.job_status(job_id).await.map_err(|e| e.to_string())
}

// ─── Ray Plugin Commands ──────────────────────────────────────────────────────

macro_rules! ray_service {
    ($state:expr) => {{
        let lock = $state.ray_service.read().await;
        match lock.as_ref() {
            Some(svc) => std::sync::Arc::clone(svc),
            None => {
                return Err(
                    "Ray Plugin is still initializing. Ensure the Python Runtime is ready."
                        .to_string(),
                )
            }
        }
    }};
}

#[tauri::command]
pub async fn ray_ensure_packages(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    let svc = ray_service!(state);
    svc.ensure_packages().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_get_settings(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::settings::RaySettings, String> {
    let svc = ray_service!(state);
    Ok(svc.get_settings().await)
}

#[tauri::command]
pub async fn ray_update_settings(
    state: tauri::State<'_, crate::AppState>,
    settings: crate::ray::settings::RaySettings,
) -> Result<crate::ray::settings::RaySettings, String> {
    let svc = ray_service!(state);
    svc.update_settings(settings)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_start_head(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::HeadInfo, String> {
    let svc = ray_service!(state);
    svc.start_head().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_stop_head(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::HeadInfo, String> {
    let svc = ray_service!(state);
    svc.stop_head().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_restart_head(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::HeadInfo, String> {
    let svc = ray_service!(state);
    svc.restart_head().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_head_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::HeadInfo, String> {
    let svc = ray_service!(state);
    Ok(svc.head_status().await)
}

#[tauri::command]
pub async fn ray_start_worker(
    state: tauri::State<'_, crate::AppState>,
    head_address: Option<String>,
) -> Result<crate::ray::WorkerInfo, String> {
    let svc = ray_service!(state);
    svc.start_worker(head_address)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_stop_worker(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::WorkerInfo, String> {
    let svc = ray_service!(state);
    svc.stop_worker().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_restart_worker(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::WorkerInfo, String> {
    let svc = ray_service!(state);
    svc.restart_worker().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_worker_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::WorkerInfo, String> {
    let svc = ray_service!(state);
    Ok(svc.worker_status().await)
}

#[tauri::command]
pub async fn ray_connect_client(
    state: tauri::State<'_, crate::AppState>,
    address: Option<String>,
) -> Result<String, String> {
    let svc = ray_service!(state);
    svc.connect_client(address).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_disconnect_client(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    let svc = ray_service!(state);
    svc.disconnect_client().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_cluster_snapshot(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::ClusterSnapshot, String> {
    let svc = ray_service!(state);
    svc.cluster_snapshot().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_cluster_info(
    state: tauri::State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    let svc = ray_service!(state);
    svc.cluster_info().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_dashboard(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::dashboard::DashboardView, String> {
    let svc = ray_service!(state);
    Ok(svc.dashboard().await)
}

#[tauri::command]
pub async fn ray_metrics(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::ray::monitoring::RayMetrics, String> {
    let svc = ray_service!(state);
    svc.metrics().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_submit_python_function(
    state: tauri::State<'_, crate::AppState>,
    function_body: String,
    args: serde_json::Value,
) -> Result<crate::ray::JobResult, String> {
    let svc = ray_service!(state);
    svc.submit_python_function(function_body, args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_map(
    state: tauri::State<'_, crate::AppState>,
    function_body: String,
    items: serde_json::Value,
) -> Result<crate::ray::JobResult, String> {
    let svc = ray_service!(state);
    svc.map(function_body, items)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_run_example(
    state: tauri::State<'_, crate::AppState>,
    example_id: String,
) -> Result<crate::ray::ExampleJobResult, String> {
    let _ = state
        .scheduler_registry
        .set_active(crate::scheduler::selection::RAY_PLUGIN_ID)
        .await;

    let title = crate::jobs::examples::get(&example_id)
        .map(|spec| spec.title.to_string())
        .unwrap_or_else(|| example_id.clone());

    let spec = crate::jobs::JobSpec::example(&title, &example_id);
    let ack = state.job_manager.submit(spec, "ray-example").await?;
    let result = state.job_manager.result(&ack.job_id).await?;
    Ok(example_result_from_job_ray(&example_id, &title, &result))
}

#[tauri::command]
pub async fn ray_cancel_job(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<(), String> {
    let svc = ray_service!(state);
    svc.cancel_job(job_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_submit_script(
    state: tauri::State<'_, crate::AppState>,
    script: String,
) -> Result<crate::ray::JobResult, String> {
    let svc = ray_service!(state);
    svc.submit_script(script).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_submit_module(
    state: tauri::State<'_, crate::AppState>,
    module: String,
) -> Result<crate::ray::JobResult, String> {
    let svc = ray_service!(state);
    svc.submit_module(module).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_scatter(
    state: tauri::State<'_, crate::AppState>,
    data: serde_json::Value,
) -> Result<crate::ray::JobResult, String> {
    let svc = ray_service!(state);
    svc.scatter(data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_gather(
    state: tauri::State<'_, crate::AppState>,
    keys: serde_json::Value,
) -> Result<crate::ray::JobResult, String> {
    let svc = ray_service!(state);
    svc.gather(keys).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ray_job_status(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let svc = ray_service!(state);
    svc.job_status(job_id).await.map_err(|e| e.to_string())
}

// ─── Unified Job API ──────────────────────────────────────────────────────────

fn example_result_from_job(
    example_id: &str,
    title: &str,
    result: &crate::jobs::JobResult,
) -> crate::dask::ExampleJobResult {
    crate::dask::ExampleJobResult {
        example_id: example_id.to_string(),
        title: title.to_string(),
        success: result.status == crate::jobs::JobStatus::Completed,
        execution_time_ms: result.metrics.execution_time_ms,
        workers_used: result.metrics.workers_used,
        cpu_utilization: result.metrics.cpu_utilization,
        speedup: result.metrics.speedup,
        result_summary: result.result_summary.clone().unwrap_or_default(),
        details: result.output.clone(),
        error: result.errors.first().cloned(),
    }
}

fn example_result_from_job_ray(
    example_id: &str,
    title: &str,
    result: &crate::jobs::JobResult,
) -> crate::ray::ExampleJobResult {
    crate::ray::ExampleJobResult {
        example_id: example_id.to_string(),
        title: title.to_string(),
        success: result.status == crate::jobs::JobStatus::Completed,
        execution_time_ms: result.metrics.execution_time_ms,
        workers_used: result.metrics.workers_used,
        cpu_utilization: result.metrics.cpu_utilization,
        speedup: result.metrics.speedup,
        result_summary: result.result_summary.clone().unwrap_or_default(),
        details: result.output.clone(),
        error: result.errors.first().cloned(),
    }
}

#[tauri::command]
pub async fn job_submit(
    state: tauri::State<'_, crate::AppState>,
    spec: crate::jobs::JobSpec,
    owner: Option<String>,
) -> Result<crate::jobs::SubmitAck, String> {
    state
        .job_api
        .submit(spec, owner.as_deref().unwrap_or("user"))
        .await
}

#[tauri::command]
pub async fn job_cancel(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<(), String> {
    state.job_api.cancel(&job_id).await
}

#[tauri::command]
pub async fn job_status(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<crate::jobs::JobStatus, String> {
    state.job_api.status(&job_id).await
}

#[tauri::command]
pub async fn job_progress(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<crate::jobs::JobProgress, String> {
    state.job_api.progress(&job_id).await
}

#[tauri::command]
pub async fn job_result(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<crate::jobs::JobResult, String> {
    state.job_api.result(&job_id).await
}

#[tauri::command]
pub async fn job_list(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<crate::jobs::Job>, String> {
    Ok(state.job_api.list().await)
}

#[tauri::command]
pub async fn job_get(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<crate::jobs::JobDetail, String> {
    state.job_api.get(&job_id).await
}

#[tauri::command]
pub async fn job_retry(
    state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<crate::jobs::SubmitAck, String> {
    state.job_api.retry(&job_id).await
}

#[tauri::command]
pub async fn scheduler_list(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<crate::jobs::SchedulerListEntry>, String> {
    Ok(state.job_api.scheduler_list().await)
}

#[tauri::command]
pub async fn scheduler_get_active(
    state: tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    Ok(state.job_api.scheduler_get_active().await)
}

#[tauri::command]
pub async fn scheduler_set_active(
    state: tauri::State<'_, crate::AppState>,
    plugin_id: String,
) -> Result<(), String> {
    state.job_api.scheduler_set_active(&plugin_id).await
}

// ─── MPI Plugin ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn mpi_ensure_toolchain(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::mpi::MpiToolchain, String> {
    let svc = state
        .mpi_service
        .read()
        .await
        .clone()
        .ok_or_else(|| "MPI service not initialized".to_string())?;
    svc.ensure_toolchain()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mpi_get_settings(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::mpi::settings::MpiSettings, String> {
    let svc = state
        .mpi_service
        .read()
        .await
        .clone()
        .ok_or_else(|| "MPI service not initialized".to_string())?;
    Ok(svc.settings().await)
}

#[tauri::command]
pub async fn mpi_update_settings(
    state: tauri::State<'_, crate::AppState>,
    settings: crate::mpi::settings::MpiSettings,
) -> Result<(), String> {
    let svc = state
        .mpi_service
        .read()
        .await
        .clone()
        .ok_or_else(|| "MPI service not initialized".to_string())?;
    svc.update_settings(settings).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mpi_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    let svc = state
        .mpi_service
        .read()
        .await
        .clone()
        .ok_or_else(|| "MPI service not initialized".to_string())?;
    let tc = svc.toolchain().await;
    Ok(serde_json::json!({
        "ready": svc.is_ready().await,
        "toolchain": tc,
    }))
}

#[tauri::command]
pub async fn p2p_local_peer_id(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Option<String>, String> {
    Ok(state
        .p2p_service
        .read()
        .await
        .as_ref()
        .map(|p| p.local_peer_id().to_string()))
}

#[tauri::command]
pub async fn p2p_listen_addrs(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    if let Some(p2p) = state.p2p_service.read().await.clone() {
        return p2p.listen_addrs().await;
    }
    Ok(Vec::new())
}

#[tauri::command]
pub async fn p2p_connect(
    state: tauri::State<'_, crate::AppState>,
    multiaddr: String,
) -> Result<(), String> {
    let p2p = state
        .p2p_service
        .read()
        .await
        .clone()
        .ok_or_else(|| "P2P not started".to_string())?;
    p2p.connect(&multiaddr).await
}

// ─── Updates (check + notify) ─────────────────────────────────────────────────

#[tauri::command]
pub async fn check_for_updates() -> Result<crate::updates::UpdateCheckResult, String> {
    crate::updates::check_for_updates().await
}

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

