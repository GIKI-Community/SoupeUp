//! Inbound ALPN handler for remote job RPCs.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh::EndpointId;
use tokio::sync::RwLock;

use crate::jobs::JobApi;

use super::framing::{read_msg, write_msg};
use super::peer_record::PeerRecord;
use super::protocol::{RemoteJobRequest, RemoteJobResponse};

#[derive(Clone)]
pub struct JobProtocolHandler {
    pub job_api: Arc<JobApi>,
    pub node_name: String,
    pub local_endpoint_id: String,
    pub peers: Arc<RwLock<HashMap<EndpointId, PeerRecord>>>,
}

impl std::fmt::Debug for JobProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobProtocolHandler")
            .field("node_name", &self.node_name)
            .field("local_endpoint_id", &self.local_endpoint_id)
            .finish_non_exhaustive()
    }
}

impl ProtocolHandler for JobProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();
        {
            let mut map = self.peers.write().await;
            let now = Utc::now();
            map.entry(remote)
                .and_modify(|r| r.last_seen = now)
                .or_insert(PeerRecord {
                    endpoint_id: remote,
                    node_name: remote.to_string(),
                    connected_since: now,
                    last_seen: now,
                });
        }

        // Serve requests until the peer closes the connection.
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(s) => s,
                Err(_) => break,
            };

            let request: RemoteJobRequest = match read_msg(&mut recv).await {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("iroh job: failed to read request from {remote}: {e}");
                    break;
                }
            };

            if let RemoteJobRequest::Hello { node_name } = &request {
                if let Some(rec) = self.peers.write().await.get_mut(&remote) {
                    rec.node_name = node_name.clone();
                    rec.last_seen = Utc::now();
                }
            }

            let response =
                handle_remote_request(&self.job_api, &self.node_name, &self.local_endpoint_id, request)
                    .await;

            if let Err(e) = write_msg(&mut send, &response).await {
                log::warn!("iroh job: failed to write response to {remote}: {e}");
                break;
            }
            if let Err(e) = send.finish() {
                log::warn!("iroh job: finish failed for {remote}: {e}");
                break;
            }
        }

        // Keep peer in the map after the connection ends (last_seen already updated).
        Ok(())
    }
}

pub async fn handle_remote_request(
    job_api: &Arc<JobApi>,
    node_name: &str,
    local_endpoint_id: &str,
    request: RemoteJobRequest,
) -> RemoteJobResponse {
    match request {
        RemoteJobRequest::Hello { .. } => RemoteJobResponse::Hello {
            peer_id: local_endpoint_id.to_string(),
            node_name: node_name.to_string(),
        },
        RemoteJobRequest::Submit { owner, spec } => match job_api.submit(spec, &owner).await {
            Ok(ack) => RemoteJobResponse::SubmitAck(ack),
            Err(e) => RemoteJobResponse::Error {
                message: e.to_string(),
            },
        },
        RemoteJobRequest::Status { job_id } => match job_api.status(&job_id).await {
            Ok(status) => RemoteJobResponse::Status { status },
            Err(e) => RemoteJobResponse::Error {
                message: e.to_string(),
            },
        },
        RemoteJobRequest::Cancel { job_id } => match job_api.cancel(&job_id).await {
            Ok(()) => RemoteJobResponse::Cancelled,
            Err(e) => RemoteJobResponse::Error {
                message: e.to_string(),
            },
        },
        RemoteJobRequest::Result { job_id } => match job_api.result(&job_id).await {
            Ok(result) => RemoteJobResponse::Result(result),
            Err(e) => RemoteJobResponse::Error {
                message: e.to_string(),
            },
        },
    }
}
