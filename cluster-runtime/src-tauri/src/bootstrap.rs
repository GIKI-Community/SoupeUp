//! Shared runtime bootstrap used by both the Tauri GUI and the headless server.
//!
//! Init order: history → API → plugins → MPI (independent) → P2P → Python → Dask → Ray.

use std::path::PathBuf;
use std::sync::Arc;

use crate::api_server;
use crate::dask::adapter::DaskSchedulerAdapter;
use crate::dask::DaskService;
use crate::mpi::adapter::MpiSchedulerAdapter;
use crate::mpi::MpiService;
use crate::plugin_registry::{self, PluginRegistry};
use crate::python_runtime::{self, PythonExecutionService};
use crate::ray::adapter::RaySchedulerAdapter;
use crate::ray::RayService;
use crate::scheduler::SchedulerRegistry;
use crate::AppState;

/// Tauri bundle identifier; discovery clients look under this app data dir.
pub const APP_IDENTIFIER: &str = "dev.cluster-runtime.app";

/// Resolve the runtime data directory.
///
/// Order: `CLUSTER_RUNTIME_DATA_DIR` → platform app data dir for
/// [`APP_IDENTIFIER`] → `./data`.
pub fn resolve_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CLUSTER_RUNTIME_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    platform_app_data_dir().unwrap_or_else(|| PathBuf::from("./data"))
}

fn platform_app_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Some(PathBuf::from(appdata).join(APP_IDENTIFIER));
        }
        return std::env::var_os("USERPROFILE").map(|home| {
            PathBuf::from(home)
                .join("AppData")
                .join("Roaming")
                .join(APP_IDENTIFIER)
        });
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_IDENTIFIER)
        });
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            return Some(PathBuf::from(xdg).join(APP_IDENTIFIER));
        }
        return std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join(APP_IDENTIFIER)
        });
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        None
    }
}

/// Load persistence, start the API, register plugins, and kick off services.
///
/// Must be called from within a Tokio runtime (e.g. `tokio::main` or
/// `tauri::async_runtime::block_on`).
pub async fn start(state: &AppState) {
    state.job_history.load().await;
    state.scheduler_registry.load_active().await;
    state.job_manager.load_persisted().await;

    api_server::start(
        state.job_api.clone(),
        state.job_manager.clone(),
        state.scheduler_registry.clone(),
        state.python_service.clone(),
        state.dask_service.clone(),
        state.ray_service.clone(),
        state.p2p_service.clone(),
        state.event_bus.clone(),
        state.data_dir.clone(),
    );

    {
        let mut registry = state.plugin_registry.write().await;
        registry.register_python_runtime();
        registry.register_dask_scheduler();
        registry.register_ray();
        registry.register_mpi();
    }

    // MPI initializes independently of Python.
    {
        let mpi = Arc::new(MpiService::new());
        let registry_lock = state.plugin_registry.clone();
        let scheduler_registry = state.scheduler_registry.clone();
        let mpi_slot = state.mpi_service.clone();
        match mpi.initialize().await {
            Ok(()) => {
                log::info!("MPI: toolchain ready.");
                *mpi_slot.write().await = Some(mpi.clone());
                scheduler_registry
                    .register(Arc::new(MpiSchedulerAdapter::new(mpi.clone())))
                    .await;
                let mut registry = registry_lock.write().await;
                registry.update_plugin_status(
                    "plugin-mpi",
                    plugin_registry::PluginStatus::Running,
                );
            }
            Err(e) => {
                log::warn!("MPI: initialization failed: {e}");
                *mpi_slot.write().await = Some(mpi.clone());
                scheduler_registry
                    .register(Arc::new(MpiSchedulerAdapter::new(mpi)))
                    .await;
                let mut registry = registry_lock.write().await;
                registry.update_plugin_status(
                    "plugin-mpi",
                    plugin_registry::PluginStatus::Error,
                );
            }
        }
    }

    // libp2p WAN mesh (firewall-friendly ports; does not touch 8129).
    {
        let p2p_slot = state.p2p_service.clone();
        let data_dir = state.data_dir.clone();
        let job_api = state.job_api.clone();
        match crate::network::p2p::P2pService::start(&data_dir, job_api).await {
            Ok(p2p) => {
                log::info!(
                    "P2P: started (local peer {})",
                    p2p.local_peer_id()
                );
                *p2p_slot.write().await = Some(p2p);
            }
            Err(e) => {
                log::error!("P2P: failed to start: {e}");
            }
        }
    }

    let python_service_slot = state.python_service.clone();
    let dask_service_slot = state.dask_service.clone();
    let ray_service_slot = state.ray_service.clone();
    let mpi_service_slot = state.mpi_service.clone();
    let scheduler_registry = state.scheduler_registry.clone();
    let registry_lock = state.plugin_registry.clone();

    tokio::spawn(async move {
        init_python_dask_ray(
            python_service_slot,
            dask_service_slot,
            ray_service_slot,
            mpi_service_slot,
            scheduler_registry,
            registry_lock,
        )
        .await;
    });
}

pub async fn shutdown_services(state: &AppState) {
    log::info!("App exit: stopping background services...");

    if let Some(p2p) = state.p2p_service.read().await.clone() {
        p2p.shutdown().await;
    }
    if let Some(mpi) = state.mpi_service.read().await.clone() {
        mpi.shutdown().await;
    }
    if let Some(ray) = state.ray_service.read().await.clone() {
        ray.shutdown().await;
    }
    if let Some(dask) = state.dask_service.read().await.clone() {
        dask.shutdown().await;
    }
    if let Some(python) = state.python_service.read().await.clone() {
        python.shutdown().await;
    }

    log::info!("App exit: background services stopped.");
}

type ServiceSlot<T> = Arc<tokio::sync::RwLock<Option<Arc<T>>>>;

async fn init_python_dask_ray(
    python_service_slot: ServiceSlot<PythonExecutionService>,
    dask_service_slot: ServiceSlot<DaskService>,
    ray_service_slot: ServiceSlot<RayService>,
    mpi_service_slot: ServiceSlot<MpiService>,
    scheduler_registry: Arc<SchedulerRegistry>,
    registry_lock: Arc<tokio::sync::RwLock<PluginRegistry>>,
) {
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

                    // Attach Python to MPI for mpi4py jobs.
                    if let Some(mpi) = mpi_service_slot.read().await.clone() {
                        mpi.set_python(Some(python_arc.clone())).await;
                    }

                    {
                        let mut registry = registry_lock.write().await;
                        registry.update_plugin_status(
                            "plugin-python-runtime",
                            plugin_registry::PluginStatus::Running,
                        );
                    }

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
                            log::error!("Dask Scheduler: initialization failed: {}", e);
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
            registry.update_plugin_status("plugin-ray", plugin_registry::PluginStatus::Error);
        }
    }
}
