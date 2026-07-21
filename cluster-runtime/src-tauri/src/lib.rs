#![allow(dead_code)]

use tauri::Manager;

mod api_server;
pub mod bootstrap;
mod commands;
mod config;
mod core;
mod dask;
mod mpi;
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
mod updates;

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
use dask::DaskService;
use events::EventBus;
use jobs::{JobApi, JobHistoryStore, JobManager};
use jobs::progress::ProgressTracker;
use jobs::results::ResultStore;
use mpi::MpiService;
use plugin_registry::PluginRegistry;
use python_runtime::PythonExecutionService;
use ray::RayService;
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
    /// The MPI service. Initialized independently of Python.
    pub mpi_service: Arc<tokio::sync::RwLock<Option<Arc<MpiService>>>>,
    /// WAN libp2p mesh (optional until started).
    pub p2p_service: Arc<tokio::sync::RwLock<Option<Arc<crate::network::p2p::P2pService>>>>,
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
            mpi_service: Arc::new(tokio::sync::RwLock::new(None)),
            p2p_service: Arc::new(tokio::sync::RwLock::new(None)),
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

            {
                let state = app.state::<AppState>();
                tauri::async_runtime::block_on(bootstrap::start(state.inner()));
            }

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
            // MPI Plugin
            commands::mpi_ensure_toolchain,
            commands::mpi_get_settings,
            commands::mpi_update_settings,
            commands::mpi_status,
            // P2P
            commands::p2p_local_peer_id,
            commands::p2p_listen_addrs,
            commands::p2p_connect,
            // Updates (check + notify)
            commands::check_for_updates,
            commands::get_app_version,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app_handle.state::<AppState>();
                tauri::async_runtime::block_on(bootstrap::shutdown_services(state.inner()));
            }
        });
}
