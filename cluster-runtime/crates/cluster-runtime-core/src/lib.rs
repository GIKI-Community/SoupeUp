#![allow(dead_code)]

mod api_server;
pub mod bootstrap;
mod config;
pub mod core;
pub mod dask;
pub mod mpi;
pub mod ray;
mod events;
pub mod jobs;
pub mod logging;
pub mod metrics;
pub mod network;
pub mod nodes;
pub mod scheduler;
mod sdk;
mod security;
mod storage;
pub mod updates;

pub mod plugin_api;
pub mod plugin_host;
pub mod plugin_loader;
pub mod plugin_registry;
pub mod plugin_security;
pub mod plugin_store;
pub mod plugins;
pub mod python_runtime;
pub mod runtime;

use std::path::PathBuf;
use std::sync::Arc;

use dask::DaskService;
use events::EventBus;
use jobs::progress::ProgressTracker;
use jobs::results::ResultStore;
use jobs::{JobApi, JobHistoryStore, JobManager};
use mpi::MpiService;
use plugin_registry::PluginRegistry;
use python_runtime::PythonExecutionService;
use ray::RayService;
use scheduler::SchedulerRegistry;

#[derive(Clone)]
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
