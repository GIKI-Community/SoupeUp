#![allow(dead_code)]

use tauri::Manager;

mod commands;
mod config;
mod core;
mod dask;
mod events;
mod jobs;
mod logging;
mod metrics;
mod network;
mod nodes;
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

use std::sync::Arc;
use dask::DaskService;
use events::EventBus;
use jobs::JobHistory;
use plugin_registry::PluginRegistry;
use python_runtime::PythonExecutionService;

pub struct AppState {
    pub plugin_registry: Arc<tokio::sync::RwLock<PluginRegistry>>,
    pub event_bus: Arc<EventBus>,
    /// The Python Runtime service. `None` while the runtime is still initializing.
    pub python_service: Arc<tokio::sync::RwLock<Option<Arc<PythonExecutionService>>>>,
    /// The Dask Scheduler service. `None` until Python is ready and packages are installed.
    pub dask_service: Arc<tokio::sync::RwLock<Option<Arc<DaskService>>>>,
    /// Recent and in-flight jobs shown on the Jobs page.
    pub job_history: Arc<JobHistory>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            plugin_registry: Arc::new(tokio::sync::RwLock::new(PluginRegistry::new())),
            event_bus: Arc::new(EventBus::default()),
            python_service: Arc::new(tokio::sync::RwLock::new(None)),
            dask_service: Arc::new(tokio::sync::RwLock::new(None)),
            job_history: Arc::new(JobHistory::new()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

async fn shutdown_services(state: &AppState) {
    log::info!("App exit: stopping background services...");

    if let Some(dask) = state.dask_service.read().await.clone() {
        dask.shutdown().await;
    } else if let Some(python) = state.python_service.read().await.clone() {
        python.shutdown().await;
    }

    log::info!("App exit: background services stopped.");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = AppState::new();
            app.manage(state);

            // Register built-in plugins so the UI can show them while async setup runs.
            {
                let registry_lock = app.state::<AppState>().plugin_registry.clone();
                tauri::async_runtime::block_on(async {
                    let mut registry = registry_lock.write().await;
                    registry.register_python_runtime();
                    registry.register_dask_scheduler();
                });
            }

            // Kick off async Python Runtime + Dask initialization in the background.
            let python_service_slot = app.state::<AppState>().python_service.clone();
            let dask_service_slot = app.state::<AppState>().dask_service.clone();
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
                                let dask = DaskService::new(python_arc);
                                match dask.initialize().await {
                                    Ok(()) => {
                                        log::info!("Dask Scheduler: service ready.");
                                        *dask_service_slot.write().await = Some(Arc::new(dask));
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
                                        // Still expose the service so the UI can retry package install.
                                        *dask_service_slot.write().await = Some(Arc::new(dask));
                                        let mut registry = registry_lock.write().await;
                                        registry.update_plugin_status(
                                            "plugin-dask-scheduler",
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
