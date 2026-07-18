//! WebSocket endpoint that streams cluster events and periodic status frames.

use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use serde_json::json;

use super::ApiContext;

pub async fn events(ws: WebSocketUpgrade, State(ctx): State<ApiContext>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, ctx))
}

async fn handle_socket(mut socket: WebSocket, ctx: ApiContext) {
    let mut rx = ctx.events_tx.subscribe();
    let mut ticker = tokio::time::interval(Duration::from_secs(2));

    // Send an initial hello frame so clients know the stream is live.
    let hello = json!({ "type": "connected", "payload": { "apiVersion": "v1" } }).to_string();
    if socket.send(Message::Text(hello)).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            // Cluster events bridged from the internal EventBus.
            evt = rx.recv() => {
                match evt {
                    Ok(payload) => {
                        if socket.send(Message::Text(payload)).await.is_err() {
                            break;
                        }
                    }
                    // Lagged: drop missed events and continue.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            // Periodic status frame so the client stays fresh without polling.
            _ = ticker.tick() => {
                let frame = status_frame(&ctx).await;
                if socket.send(Message::Text(frame)).await.is_err() {
                    break;
                }
            }
            // Handle inbound frames (ping/close) to keep the socket healthy.
            inbound = socket.recv() => {
                match inbound {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

async fn status_frame(ctx: &ApiContext) -> String {
    let active = ctx.job_api.scheduler_get_active().await;
    let jobs = ctx.job_api.list().await;
    json!({
        "type": "status",
        "payload": {
            "activeScheduler": active,
            "jobCount": jobs.len(),
        }
    })
    .to_string()
}
