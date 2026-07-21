//! Headless Cluster Runtime server — no GUI.
//!
//! Starts the same Python / Dask / Ray stack and loopback HTTP API as the
//! desktop app. Do not run this alongside the Tauri GUI on the same port.

use cluster_runtime_lib::bootstrap::{self, resolve_data_dir};
use cluster_runtime_lib::AppState;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let data_dir = resolve_data_dir();
    log::info!(
        "cluster-runtime-server: data dir {}",
        data_dir.display()
    );

    let state = AppState::new(data_dir);
    bootstrap::start(&state).await;

    log::info!(
        "cluster-runtime-server: running (default API 127.0.0.1:8129). Press Ctrl+C to stop."
    );

    match tokio::signal::ctrl_c().await {
        Ok(()) => log::info!("cluster-runtime-server: Ctrl+C received"),
        Err(e) => log::error!("cluster-runtime-server: failed to listen for Ctrl+C: {e}"),
    }

    bootstrap::shutdown_services(&state).await;
    log::info!("cluster-runtime-server: exited");
}
