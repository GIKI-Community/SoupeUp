//! Shared runtime bootstrap used by both the Tauri GUI and the headless server.
//!
//! Init order: history → API → P2P → manifest-driven plugins (factories).

use std::path::PathBuf;

use crate::api_server;
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

/// Load persistence, start the API, then discover and start enabled plugins.
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

    // Seed manifests, discover, start enabled compatible plugins via host factories.
    log::info!("bootstrap: spawning plugin discovery/start");
    let state_for_plugins = state.clone();
    tokio::spawn(async move {
        crate::plugin_host::factories::load_and_start_plugins(&state_for_plugins).await;
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
