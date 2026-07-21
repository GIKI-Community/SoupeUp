//! REST route definitions and handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{auth, ws, ApiContext};
use crate::jobs::JobSpec;

/// Uniform JSON error response.
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
    fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

pub fn router(ctx: ApiContext) -> Router {
    let v1 = Router::new()
        .route("/system", get(get_system))
        .route("/schedulers", get(list_schedulers))
        .route(
            "/schedulers/active",
            get(get_active_scheduler).put(set_active_scheduler),
        )
        .route("/cluster", get(get_cluster))
        .route("/nodes", get(get_nodes))
        .route("/jobs", get(list_jobs).post(submit_job))
        .route("/jobs/:id", get(get_job))
        .route("/jobs/:id/result", get(get_job_result))
        .route("/jobs/:id/cancel", post(cancel_job))
        .route("/jobs/:id/retry", post(retry_job))
        .route("/logs", get(get_logs))
        .route("/events", get(ws::events))
        .route("/peers", get(list_peers).post(connect_peer))
        .route_layer(middleware::from_fn_with_state(ctx.clone(), auth::require_token));

    Router::new()
        .route("/health", get(health))
        .nest("/v1", v1)
        .with_state(ctx)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "cluster-runtime", "apiVersion": "v1" }))
}

async fn get_system() -> Json<Value> {
    Json(json!({
        "info": crate::core::mock_system_info(),
        "status": crate::core::mock_system_status(),
    }))
}

async fn list_schedulers(State(ctx): State<ApiContext>) -> ApiResult<Value> {
    let entries = ctx.job_api.scheduler_list().await;
    Ok(Json(json!(entries)))
}

async fn get_active_scheduler(State(ctx): State<ApiContext>) -> ApiResult<Value> {
    let active = ctx.job_api.scheduler_get_active().await;
    Ok(Json(json!({ "pluginId": active })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetActiveBody {
    plugin_id: String,
}

async fn set_active_scheduler(
    State(ctx): State<ApiContext>,
    Json(body): Json<SetActiveBody>,
) -> ApiResult<Value> {
    ctx.job_api
        .scheduler_set_active(&body.plugin_id)
        .await
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({ "pluginId": body.plugin_id })))
}

async fn get_cluster(State(ctx): State<ApiContext>) -> ApiResult<Value> {
    let active = ctx.job_api.scheduler_get_active().await;

    let mut worker_count = 0usize;
    let mut total_cores = 0usize;
    let mut total_memory = 0u64;
    let mut scheduler_running = false;
    let mut health = "unknown".to_string();

    if let Some(svc) = ctx.dask_service.read().await.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            use crate::dask::ComponentStatus;
            if snap.scheduler.status == ComponentStatus::Running {
                scheduler_running = true;
            }
            worker_count += snap.workers.len();
            total_cores += snap.total_cores;
            total_memory += snap.total_memory;
            health = format!("{:?}", snap.health).to_lowercase();
        }
    }

    if let Some(svc) = ctx.ray_service.read().await.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            use crate::ray::ComponentStatus;
            if snap.head.status == ComponentStatus::Running {
                scheduler_running = true;
            }
            worker_count += snap.workers.len();
            total_cores += snap.total_cores;
            total_memory += snap.total_memory;
            if health == "unknown" {
                health = format!("{:?}", snap.health).to_lowercase();
            }
        }
    }

    Ok(Json(json!({
        "activeScheduler": active,
        "schedulerRunning": scheduler_running,
        "health": health,
        "workerCount": worker_count,
        "totalCores": total_cores,
        "totalMemory": total_memory,
    })))
}

async fn get_nodes(State(ctx): State<ApiContext>) -> ApiResult<Value> {
    let mut nodes = Vec::new();

    if let Some(svc) = ctx.dask_service.read().await.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            nodes.extend(crate::nodes::nodes_from_dask_snapshot(&snap));
        }
    }
    if let Some(svc) = ctx.ray_service.read().await.as_ref() {
        if let Ok(snap) = svc.cluster_snapshot().await {
            nodes.extend(crate::nodes::nodes_from_ray_snapshot(&snap));
        }
    }

    Ok(Json(json!(nodes)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitQuery {
    owner: Option<String>,
    /// When set, forward the job to a remote peer over libp2p.
    target_peer: Option<String>,
}

async fn submit_job(
    State(ctx): State<ApiContext>,
    Query(q): Query<SubmitQuery>,
    Json(spec): Json<JobSpec>,
) -> ApiResult<Value> {
    let owner = q.owner.unwrap_or_else(|| "vscode".to_string());
    if let Some(peer) = q.target_peer.filter(|s| !s.is_empty()) {
        let p2p = ctx
            .p2p_service
            .read()
            .await
            .clone()
            .ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "P2P not started"))?;
        let ack = p2p
            .remote_submit(&peer, &owner, spec)
            .await
            .map_err(ApiError::internal)?;
        return Ok(Json(json!(ack)));
    }
    let ack = ctx
        .job_api
        .submit(spec, &owner)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!(ack)))
}

async fn list_jobs(State(ctx): State<ApiContext>) -> ApiResult<Value> {
    let jobs = ctx.job_api.list().await;
    Ok(Json(json!(jobs)))
}

async fn get_job(
    State(ctx): State<ApiContext>,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    let detail = ctx
        .job_api
        .get(&id)
        .await
        .map_err(ApiError::not_found)?;
    Ok(Json(json!(detail)))
}

async fn get_job_result(
    State(ctx): State<ApiContext>,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    let result = ctx
        .job_api
        .result(&id)
        .await
        .map_err(ApiError::not_found)?;
    Ok(Json(json!(result)))
}

async fn cancel_job(
    State(ctx): State<ApiContext>,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    ctx.job_api.cancel(&id).await.map_err(ApiError::internal)?;
    Ok(Json(json!({ "cancelled": id })))
}

async fn retry_job(
    State(ctx): State<ApiContext>,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    let ack = ctx.job_api.retry(&id).await.map_err(ApiError::internal)?;
    Ok(Json(json!(ack)))
}

async fn get_logs() -> Json<Value> {
    Json(json!(crate::logging::mock_logs()))
}

async fn list_peers(State(ctx): State<ApiContext>) -> ApiResult<Value> {
    let Some(p2p) = ctx.p2p_service.read().await.clone() else {
        return Ok(Json(json!({
            "localPeerId": null,
            "listenAddrs": [],
            "peers": [],
        })));
    };
    let peers = p2p.list_peers().await.map_err(ApiError::internal)?;
    let listen_addrs = p2p.listen_addrs().await.unwrap_or_default();
    Ok(Json(json!({
        "localPeerId": p2p.local_peer_id(),
        "listenAddrs": listen_addrs,
        "peers": peers,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectPeerBody {
    multiaddr: String,
}

async fn connect_peer(
    State(ctx): State<ApiContext>,
    Json(body): Json<ConnectPeerBody>,
) -> ApiResult<Value> {
    let p2p = ctx
        .p2p_service
        .read()
        .await
        .clone()
        .ok_or_else(|| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "P2P not started"))?;
    p2p.connect(&body.multiaddr)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "connected": body.multiaddr })))
}

#[cfg(test)]
mod tests {
    use super::health;

    #[tokio::test]
    async fn health_reports_ok() {
        let Json(body) = health().await;
        assert_eq!(body["status"], "ok");
        assert_eq!(body["apiVersion"], "v1");
    }
}
