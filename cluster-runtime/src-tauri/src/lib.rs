#![allow(dead_code)]

use tauri::Manager;

mod api_server;
mod commands;
mod config;
mod core;
mod dask;
mod ray;
mod events;
mod jobs;
mod logging;
mod metrics;
mod network;
mod nodes;
mod scheduler;
mod sdk;
mod security;
mod storage;

pub mod plugin_api;
pub mod plugin_host;
pub mod plugin_loader;
pub mod plugin_registry;
pub mod plugin_security;
pub mod plugin_store;
pub mod python_runtime;
pub mod runtime;

use std::path::PathBuf;
use std::sync::Arc;
use dask::adapter::DaskSchedulerAdapter;
use dask::DaskService;
use ray::adapter::RaySchedulerAdapter;
use ray::RayService;
use events::EventBus;
use jobs::{JobApi, JobHistoryStore, JobManager};
use jobs::progress::ProgressTracker;
use jobs::results::ResultStore;
use plugin_registry::PluginRegistry;
use python_runtime::PythonExecutionService;
use scheduler::SchedulerRegistry;

pub struct AppState {
    pub plugin_registry: Arc<tokio::sync::RwLock<PluginRegistry>>,
    pub event_bus: Arc<EventBus>,
    /// The Python Runtime service. `None` while the runtime is still initializing.
    pub python_service: Arc<tokio::sync::RwLock<Option<Arc<PythonExecutionService>>>>,
    /// The Dask Scheduler service. `None` until Python is ready and packages are installed.
    pub dask_service: Arc<tokio::sync::RwLock<Option<Arc<DaskService>>>>,
    /// The Ray service. `None` until Python is ready and packages are installed.
    pub ray_service: Arc<tokio::sync::RwLock<Option<Arc<RayService>>>>,
    pub scheduler_registry: Arc<SchedulerRegistry>,
    pub job_history: Arc<JobHistoryStore>,
    pub job_manager: Arc<JobManager>,
    pub job_api: Arc<JobApi>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let event_bus = Arc::new(EventBus::default());
        let scheduler_registry = Arc::new(SchedulerRegistry::new(
            data_dir.join("scheduler").join("active_scheduler.json"),
        ));
        let job_history = Arc::new(JobHistoryStore::new(
            data_dir.join("jobs").join("history.jsonl"),
        ));
        let results = Arc::new(ResultStore::new(
            data_dir.join("jobs").join("results.json"),
        ));
        let progress = Arc::new(ProgressTracker::new());
        let job_manager = Arc::new(JobManager::new(
            scheduler_registry.clone(),
            job_history.clone(),
            results,
            progress,
            event_bus.clone(),
        ));
        let job_api = Arc::new(JobApi::new(
            job_manager.clone(),
            scheduler_registry.clone(),
            job_history.clone(),
        ));

        Self {
            plugin_registry: Arc::new(tokio::sync::RwLock::new(PluginRegistry::new())),
            event_bus,
            python_service: Arc::new(tokio::sync::RwLock::new(None)),
            dask_service: Arc::new(tokio::sync::RwLock::new(None)),
            ray_service: Arc::new(tokio::sync::RwLock::new(None)),
            scheduler_registry,
            job_history,
            job_manager,
            job_api,
            data_dir,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(PathBuf::from("./data"))
    }
}

async fn shutdown_services(state: &AppState) {
    log::info!("App exit: stopping background services...");

    // Stop scheduler plugins first (each stops its own head/worker processes).
    if let Some(ray) = state.ray_service.read().await.clone() {
        ray.shutdown().await;
    }
    if let Some(dask) = state.dask_service.read().await.clone() {
        dask.shutdown().await;
    }

    // Final sweep: any leftover background Python processes.
    if let Some(python) = state.python_service.read().await.clone() {
        python.shutdown().await;
    }

    log::info!("App exit: background services stopped.");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./data"));
            let state = AppState::new(data_dir);
            app.manage(state);

            // Load persisted job history and scheduler selection.
            {
                let state = app.state::<AppState>();
                let history = state.job_history.clone();
                let registry = state.scheduler_registry.clone();
                let job_manager = state.job_manager.clone();
                tauri::async_runtime::block_on(async {
                    history.load().await;
                    registry.load_active().await;
                    job_manager.load_persisted().await;
                });
            }

            // Start the local HTTP + WebSocket API server (external clients: VS Code, CLI).
            {
                let state = app.state::<AppState>();
                api_server::start(
                    state.job_api.clone(),
                    state.job_manager.clone(),
                    state.scheduler_registry.clone(),
                    state.python_service.clone(),
                    state.dask_service.clone(),
                    state.ray_service.clone(),
                    state.event_bus.clone(),
                    state.data_dir.clone(),
                );
            }

            // Register built-in plugins so the UI can show them while async setup runs.
            {
                let registry_lock = app.state::<AppState>().plugin_registry.clone();
                tauri::async_runtime::block_on(async {
                    let mut registry = registry_lock.write().await;
                    registry.register_python_runtime();
                    registry.register_dask_scheduler();
                    registry.register_ray();
                });
            }

            // Kick off async Python Runtime + Dask initialization in the background.
            let python_service_slot = app.state::<AppState>().python_service.clone();
            let dask_service_slot = app.state::<AppState>().dask_service.clone();
            let ray_service_slot = app.state::<AppState>().ray_service.clone();
            let scheduler_registry = app.state::<AppState>().scheduler_registry.clone();
            let registry_lock = app.state::<AppState>().plugin_registry.clone();

            tauri::async_runtime::spawn(async move {
                log::info!("Python Runtime: starting background initialization...");

                match python_runtime::interpreter::discover_python().await {
                    Some(interpreter) => {
                        log::info!(
                            "Python Runtime: found interpreter {} at {}",
                            interpreter.version,
                            interpreter.path.display()
                        );

                        let svc = PythonExecutionService::new(interpreter, None);

                        match svc.initialize().await {
                            Ok(()) => {
                                log::info!("Python Runtime: service ready.");
                                let python_arc = Arc::new(svc);
                                *python_service_slot.write().await = Some(python_arc.clone());

                                {
                                    let mut registry = registry_lock.write().await;
                                    registry.update_plugin_status(
                                        "plugin-python-runtime",
                                        plugin_registry::PluginStatus::Running,
                                    );
                                }

                                // Initialize Dask on top of the ready Python runtime.
                                log::info!("Dask Scheduler: starting initialization...");
                                let dask = DaskService::new(python_arc.clone());
                                match dask.initialize().await {
                                    Ok(()) => {
                                        log::info!("Dask Scheduler: service ready.");
                                        let dask_arc = Arc::new(dask);
                                        *dask_service_slot.write().await = Some(dask_arc.clone());
                                        scheduler_registry
                                            .register(Arc::new(DaskSchedulerAdapter::new(dask_arc)))
                                            .await;
                                        let mut registry = registry_lock.write().await;
                                        registry.update_plugin_status(
                                            "plugin-dask-scheduler",
                                            plugin_registry::PluginStatus::Running,
                                        );
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Dask Scheduler: initialization failed: {}",
                                            e
                                        );
                                        let dask_arc = Arc::new(dask);
                                        *dask_service_slot.write().await = Some(dask_arc.clone());
                                        scheduler_registry
                                            .register(Arc::new(DaskSchedulerAdapter::new(dask_arc)))
                                            .await;
                                        let mut registry = registry_lock.write().await;
                                        registry.update_plugin_status(
                                            "plugin-dask-scheduler",
                                            plugin_registry::PluginStatus::Error,
                                        );
                                    }
                                }

                                log::info!("Ray: starting initialization...");
                                let ray = RayService::new(python_arc);
                                match ray.initialize().await {
                                    Ok(()) => {
                                        log::info!("Ray: service ready.");
                                        let ray_arc = Arc::new(ray);
                                        *ray_service_slot.write().await = Some(ray_arc.clone());
                                        scheduler_registry
                                            .register(Arc::new(RaySchedulerAdapter::new(ray_arc)))
                                            .await;
                                        let mut registry = registry_lock.write().await;
                                        registry.update_plugin_status(
                                            "plugin-ray",
                                            plugin_registry::PluginStatus::Running,
                                        );
                                    }
                                    Err(e) => {
                                        log::error!("Ray: initialization failed: {}", e);
                                        let ray_arc = Arc::new(ray);
                                        *ray_service_slot.write().await = Some(ray_arc.clone());
                                        scheduler_registry
                                            .register(Arc::new(RaySchedulerAdapter::new(ray_arc)))
                                            .await;
                                        let mut registry = registry_lock.write().await;
                                        registry.update_plugin_status(
                                            "plugin-ray",
                                            plugin_registry::PluginStatus::Error,
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Python Runtime: initialization failed: {}", e);
                                let mut registry = registry_lock.write().await;
                                registry.update_plugin_status(
                                    "plugin-python-runtime",
                                    plugin_registry::PluginStatus::Error,
                                );
                                registry.update_plugin_status(
                                    "plugin-dask-scheduler",
                                    plugin_registry::PluginStatus::Error,
                                );
                                registry.update_plugin_status(
                                    "plugin-ray",
                                    plugin_registry::PluginStatus::Error,
                                );
                            }
                        }
                    }
                    None => {
                        log::error!(
                            "Python Runtime: no Python interpreter found. \
                             Run `scripts/Setup-PythonRuntime.ps1` to install the bundled Python."
                        );
                        let mut registry = registry_lock.write().await;
                        registry.update_plugin_status(
                            "plugin-python-runtime",
                            plugin_registry::PluginStatus::Error,
                        );
                        registry.update_plugin_status(
                            "plugin-dask-scheduler",
                            plugin_registry::PluginStatus::Error,
                        );
                        registry.update_plugin_status(
                            "plugin-ray",
                            plugin_registry::PluginStatus::Error,
                        );
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_system_info,
            commands::get_system_status,
            commands::get_activity,
            commands::get_nodes,
            commands::get_jobs,
            commands::get_plugins,
            commands::get_metrics,
            commands::get_logs,
            commands::get_cluster_summary,
            commands::get_cluster_peers,
            commands::python_execute_code,
            commands::python_execute_script,
            commands::python_execute_module,
            commands::python_install_package,
            commands::python_uninstall_package,
            commands::python_list_packages,
            commands::python_create_environment,
            commands::python_delete_environment,
            commands::python_activate_environment,
            commands::python_runtime_health,
            commands::python_version,
            commands::python_list_environments,
            commands::python_package_index,
            commands::python_set_package_index,
            // Dask Scheduler Plugin
            commands::dask_ensure_packages,
            commands::dask_get_settings,
            commands::dask_update_settings,
            commands::dask_start_scheduler,
            commands::dask_stop_scheduler,
            commands::dask_restart_scheduler,
            commands::dask_scheduler_status,
            commands::dask_start_worker,
            commands::dask_stop_worker,
            commands::dask_restart_worker,
            commands::dask_worker_status,
            commands::dask_connect_client,
            commands::dask_disconnect_client,
            commands::dask_cluster_snapshot,
            commands::dask_cluster_info,
            commands::dask_dashboard,
            commands::dask_metrics,
            commands::dask_submit_python_function,
            commands::dask_map,
            commands::dask_submit_script,
            commands::dask_submit_module,
            commands::dask_scatter,
            commands::dask_gather,
            commands::dask_job_status,
            commands::dask_run_example,
            commands::dask_cancel_job,
            // Ray Plugin
            commands::ray_ensure_packages,
            commands::ray_get_settings,
            commands::ray_update_settings,
            commands::ray_start_head,
            commands::ray_stop_head,
            commands::ray_restart_head,
            commands::ray_head_status,
            commands::ray_start_worker,
            commands::ray_stop_worker,
            commands::ray_restart_worker,
            commands::ray_worker_status,
            commands::ray_connect_client,
            commands::ray_disconnect_client,
            commands::ray_cluster_snapshot,
            commands::ray_cluster_info,
            commands::ray_dashboard,
            commands::ray_metrics,
            commands::ray_submit_python_function,
            commands::ray_map,
            commands::ray_submit_script,
            commands::ray_submit_module,
            commands::ray_scatter,
            commands::ray_gather,
            commands::ray_job_status,
            commands::ray_run_example,
            commands::ray_cancel_job,
            // Unified Job API
            commands::job_submit,
            commands::job_cancel,
            commands::job_status,
            commands::job_progress,
            commands::job_result,
            commands::job_list,
            commands::job_get,
            commands::job_retry,
            commands::scheduler_list,
            commands::scheduler_get_active,
            commands::scheduler_set_active,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app_handle.state::<AppState>();
                tauri::async_runtime::block_on(shutdown_services(state.inner()));
            }
        });
}
