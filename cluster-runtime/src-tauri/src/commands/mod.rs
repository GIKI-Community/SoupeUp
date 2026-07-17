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
    let lock = state.dask_service.read().await;
    if let Some(svc) = lock.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            return Ok(crate::nodes::nodes_from_dask_snapshot(&snap));
        }
    }
    Ok(mock_nodes())
}

#[tauri::command]
pub async fn get_jobs(state: tauri::State<'_, crate::AppState>) -> Result<Vec<crate::jobs::Job>, String> {
    Ok(state.job_history.list().await)
}

#[tauri::command]
pub async fn get_cluster_summary(
    state: tauri::State<'_, crate::AppState>,
) -> Result<ClusterSummary, String> {
    let lock = state.dask_service.read().await;
    if let Some(svc) = lock.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            use crate::dask::ComponentStatus;
            let scheduler_up = snap.scheduler.status == ComponentStatus::Running;
            return Ok(ClusterSummary {
                total_nodes: snap.workers.len().max(1),
                online_nodes: if scheduler_up {
                    snap.workers.len().max(1)
                } else {
                    snap.workers.len()
                },
                total_cpus: snap.total_cores,
                total_ram: snap.total_memory,
                total_gpus: 0,
                total_workers: snap.workers.len(),
                total_available_compute: snap.total_cores as f32,
            });
        }
    }

    Ok(ClusterSummary {
        total_nodes: 0,
        online_nodes: 0,
        total_cpus: 0,
        total_ram: 0,
        total_gpus: 0,
        total_workers: 0,
        total_available_compute: 0.0,
    })
}

#[tauri::command]
pub async fn get_cluster_peers(
    _state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<PeerInfo>, String> {
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
    let svc = {
        let lock = state.dask_service.read().await;
        lock.as_ref().cloned()
    };
    let Some(svc) = svc else {
        return Ok(crate::dask::example_failure(
            &example_id,
            "Dask Scheduler Plugin is still initializing. Ensure the Python Runtime is ready."
                .into(),
        ));
    };

    let title = crate::dask::examples::get(&example_id)
        .map(|spec| spec.title.to_string())
        .unwrap_or_else(|| example_id.clone());

    let job_id = state
        .job_history
        .begin(&title, "dask-example")
        .await;

    let result = match svc.run_example(example_id.clone()).await {
        Ok(result) => result,
        Err(e) => crate::dask::example_failure(&example_id, e.to_string()),
    };

    state
        .job_history
        .finish(&job_id, result.success, result.execution_time_ms)
        .await;

    Ok(result)
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

