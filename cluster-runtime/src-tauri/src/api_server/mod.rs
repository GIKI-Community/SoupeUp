//! Local HTTP + WebSocket API server.
//!
//! This is the single external interface to Cluster Runtime. It binds to
//! loopback only and requires a bearer token, and it bridges the existing
//! `JobManager` / `SchedulerRegistry` / `EventBus` so that external clients
//! (the VS Code extension, a future CLI, etc.) never talk to Dask/Ray directly.

mod auth;
mod routes;
mod ws;

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

use crate::dask::DaskService;
use crate::events::{ClusterEvent, EventBus, EventHandler};
use crate::jobs::{JobApi, JobManager};
use crate::network::p2p::P2pService;
use crate::python_runtime::PythonExecutionService;
use crate::ray::RayService;
use crate::scheduler::SchedulerRegistry;

/// Default loopback bind address; override with `CLUSTER_RUNTIME_API_ADDR`.
pub const DEFAULT_ADDR: &str = "127.0.0.1:8129";

type ServiceSlot<T> = Arc<RwLock<Option<Arc<T>>>>;

/// Shared state handed to every axum handler. All fields are `Arc`, so the
/// context is cheap to clone (axum requires `Clone` state).
#[derive(Clone)]
pub struct ApiContext {
    pub job_api: Arc<JobApi>,
    pub job_manager: Arc<JobManager>,
    pub scheduler_registry: Arc<SchedulerRegistry>,
    pub python_service: ServiceSlot<PythonExecutionService>,
    pub dask_service: ServiceSlot<DaskService>,
    pub ray_service: ServiceSlot<RayService>,
    pub p2p_service: ServiceSlot<P2pService>,
    pub token: Arc<String>,
    /// Serialized `ClusterEvent`s fanned out to connected WebSocket clients.
    pub events_tx: broadcast::Sender<String>,
}

/// Bridges the internal `EventBus` into the WebSocket broadcast channel.
struct EventForwarder {
    tx: broadcast::Sender<String>,
}

impl EventHandler for EventForwarder {
    fn handle(&self, event: &ClusterEvent) {
        if let Ok(json) = serde_json::to_string(event) {
            // Ignore send errors (no subscribers is fine).
            let _ = self.tx.send(json);
        }
    }
}

/// Contents of the discovery file written to `{data_dir}/api/endpoint.json`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EndpointFile {
    url: String,
    token: String,
    pid: u32,
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32).map(|_| format!("{:02x}", rng.gen::<u8>())).collect()
}

/// Build the API context, register the event bridge, and spawn the server.
///
/// Returns immediately; the server runs on the provided async runtime.
pub fn start(
    job_api: Arc<JobApi>,
    job_manager: Arc<JobManager>,
    scheduler_registry: Arc<SchedulerRegistry>,
    python_service: ServiceSlot<PythonExecutionService>,
    dask_service: ServiceSlot<DaskService>,
    ray_service: ServiceSlot<RayService>,
    p2p_service: ServiceSlot<P2pService>,
    event_bus: Arc<EventBus>,
    data_dir: PathBuf,
) {
    let token = generate_token();
    let (events_tx, _) = broadcast::channel::<String>(256);

    // Bridge EventBus -> WebSocket broadcast.
    event_bus.subscribe(Arc::new(EventForwarder {
        tx: events_tx.clone(),
    }));

    let ctx = ApiContext {
        job_api,
        job_manager,
        scheduler_registry,
        python_service,
        dask_service,
        ray_service,
        p2p_service,
        token: Arc::new(token.clone()),
        events_tx,
    };

    let addr = std::env::var("CLUSTER_RUNTIME_API_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.to_string());

    // Tokio so this works under both Tauri's runtime and the headless binary.
    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                log::error!("API server: failed to bind {addr}: {e}");
                return;
            }
        };

        let bound = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| addr.clone());
        let url = format!("http://{bound}");

        if let Err(e) = write_endpoint_file(&data_dir, &url, &token).await {
            log::warn!("API server: could not write discovery file: {e}");
        }

        log::info!("API server: listening on {url}");

        let app = routes::router(ctx);
        if let Err(e) = axum::serve(listener, app).await {
            log::error!("API server: stopped with error: {e}");
        }
    });
}

async fn write_endpoint_file(
    data_dir: &std::path::Path,
    url: &str,
    token: &str,
) -> std::io::Result<()> {
    let dir = data_dir.join("api");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join("endpoint.json");
    let body = serde_json::to_vec_pretty(&EndpointFile {
        url: url.to_string(),
        token: token.to_string(),
        pid: std::process::id(),
    })
    .unwrap_or_default();
    tokio::fs::write(&path, body).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = tokio::fs::set_permissions(&path, perms).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{generate_token, EndpointFile};

    #[test]
    fn token_is_64_hex_chars() {
        let token = generate_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(generate_token(), generate_token());
    }

    #[test]
    fn endpoint_file_serializes_camel_case() {
        let json = serde_json::to_string(&EndpointFile {
            url: "http://127.0.0.1:8129".into(),
            token: "abc".into(),
            pid: 7,
        })
        .unwrap();
        assert!(json.contains("\"url\":\"http://127.0.0.1:8129\""));
        assert!(json.contains("\"token\":\"abc\""));
        assert!(json.contains("\"pid\":7"));
    }
}
