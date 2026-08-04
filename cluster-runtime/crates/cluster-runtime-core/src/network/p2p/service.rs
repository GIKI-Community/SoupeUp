//! iroh WAN mesh: dial by EndpointId, relay jobs, optional TCP tunnels.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use iroh::endpoint::presets;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, EndpointId};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::jobs::JobApi;
use crate::network::PeerInfo;

use super::framing::{read_msg, write_msg};
use super::identity;
use super::job_handler::JobProtocolHandler;
use super::peer_record::PeerRecord;
use super::protocol::{RemoteJobRequest, RemoteJobResponse, JOB_ALPN, TCP_PROXY_ALPN};
use super::tcp_proxy::{self, TcpProxyHandler, TcpTunnel};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerSnapshot {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub connected: bool,
}

/// WAN P2P mesh handle (iroh under the hood; name kept for API stability).
pub struct P2pService {
    endpoint: Endpoint,
    router: Router,
    local_peer_id: String,
    peers: Arc<RwLock<HashMap<EndpointId, PeerRecord>>>,
    /// Active local TCP tunnels (worker → remote scheduler).
    tunnels: Arc<RwLock<Vec<TcpTunnel>>>,
    node_name: String,
}

impl P2pService {
    /// Bind an iroh endpoint, register job + tcp-proxy ALPNs, dial bootstrap peers.
    pub async fn start(data_dir: &Path, job_api: Arc<JobApi>) -> Result<Arc<Self>, String> {
        let secret = identity::load_or_generate(data_dir)?;
        let node_name = std::env::var("CLUSTER_RUNTIME_NODE_NAME")
            .unwrap_or_else(|_| "cluster-node".into());

        let endpoint = Endpoint::builder(presets::N0)
            .secret_key(secret)
            .bind()
            .await
            .map_err(|e| format!("iroh bind: {e}"))?;

        let local_peer_id = endpoint.id().to_string();
        let peers: Arc<RwLock<HashMap<EndpointId, PeerRecord>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let job_handler = JobProtocolHandler {
            job_api,
            node_name: node_name.clone(),
            local_endpoint_id: local_peer_id.clone(),
            peers: peers.clone(),
        };

        let router = Router::builder(endpoint.clone())
            .accept(JOB_ALPN, job_handler)
            .accept(TCP_PROXY_ALPN, TcpProxyHandler)
            .spawn();

        // Wait briefly for relay/DNS so dials are more likely to succeed.
        tokio::spawn({
            let ep = endpoint.clone();
            async move {
                ep.online().await;
                log::info!("iroh: endpoint online ({})", ep.id());
            }
        });

        let service = Arc::new(Self {
            endpoint,
            router,
            local_peer_id: local_peer_id.clone(),
            peers,
            tunnels: Arc::new(RwLock::new(Vec::new())),
            node_name,
        });

        for id in resolve_bootstrap_endpoint_ids() {
            log::info!("iroh: dialing bootstrap {id}");
            if let Err(e) = service.connect(&id.to_string()).await {
                log::warn!("iroh: bootstrap dial failed for {id}: {e}");
            }
        }

        log::info!("iroh: started (endpoint {local_peer_id})");
        Ok(service)
    }

    pub fn local_peer_id(&self) -> &str {
        &self.local_peer_id
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Dial a peer by EndpointId (DNS/Pkarr address lookup via n0 preset).
    pub async fn connect(&self, endpoint_id: &str) -> Result<(), String> {
        let id: EndpointId = endpoint_id
            .parse()
            .map_err(|e| format!("Invalid endpoint id: {e}"))?;
        let addr = EndpointAddr::new(id);
        let conn = self
            .endpoint
            .connect(addr, JOB_ALPN)
            .await
            .map_err(|e| format!("iroh connect: {e}"))?;

        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| format!("open_bi: {e}"))?;

        write_msg(
            &mut send,
            &RemoteJobRequest::Hello {
                node_name: self.node_name.clone(),
            },
        )
        .await?;
        send.finish().map_err(|e| e.to_string())?;

        let response: RemoteJobResponse = read_msg(&mut recv).await?;
        let remote_name = match response {
            RemoteJobResponse::Hello { node_name, .. } => node_name,
            other => {
                return Err(format!("unexpected hello response: {other:?}"));
            }
        };

        let now = Utc::now();
        self.peers.write().await.insert(
            id,
            PeerRecord {
                endpoint_id: id,
                node_name: remote_name,
                connected_since: now,
                last_seen: now,
            },
        );
        log::info!("iroh: connected to {id}");
        Ok(())
    }

    pub async fn list_peers(&self) -> Result<Vec<PeerInfo>, String> {
        Ok(self
            .peers
            .read()
            .await
            .values()
            .map(PeerRecord::to_peer_info)
            .collect())
    }

    /// Home relay + direct addresses for this endpoint (for display / sharing).
    pub async fn listen_addrs(&self) -> Result<Vec<String>, String> {
        let addr = self.endpoint.addr();
        let mut out = vec![format!("endpoint:{}", addr.id)];
        for relay in addr.relay_urls() {
            out.push(format!("relay:{relay}"));
        }
        for ip in addr.ip_addrs() {
            out.push(format!("ip:{ip}"));
        }
        Ok(out)
    }

    pub async fn remote_request(
        &self,
        peer_id: &str,
        request: RemoteJobRequest,
    ) -> Result<RemoteJobResponse, String> {
        let id: EndpointId = peer_id
            .parse()
            .map_err(|e| format!("Invalid endpoint id: {e}"))?;
        let addr = EndpointAddr::new(id);
        let conn = self
            .endpoint
            .connect(addr, JOB_ALPN)
            .await
            .map_err(|e| format!("iroh connect: {e}"))?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| format!("open_bi: {e}"))?;

        write_msg(&mut send, &request).await?;
        send.finish().map_err(|e| e.to_string())?;
        let response: RemoteJobResponse = read_msg(&mut recv).await?;

        // Refresh peer bookkeeping.
        let now = Utc::now();
        self.peers
            .write()
            .await
            .entry(id)
            .and_modify(|r| r.last_seen = now)
            .or_insert(PeerRecord {
                endpoint_id: id,
                node_name: id.to_string(),
                connected_since: now,
                last_seen: now,
            });

        Ok(response)
    }

    pub async fn remote_submit(
        &self,
        peer_id: &str,
        owner: &str,
        spec: crate::jobs::models::JobSpec,
    ) -> Result<crate::jobs::models::SubmitAck, String> {
        match self
            .remote_request(
                peer_id,
                RemoteJobRequest::Submit {
                    owner: owner.to_string(),
                    spec,
                },
            )
            .await?
        {
            RemoteJobResponse::SubmitAck(ack) => Ok(ack),
            RemoteJobResponse::Error { message } => Err(message),
            other => Err(format!("Unexpected remote response: {other:?}")),
        }
    }

    /// Open a local loopback TCP tunnel to `remote_port` on `endpoint_id`.
    /// Returns the local `tcp://127.0.0.1:<port>` address for Dask/etc.
    pub async fn open_tcp_tunnel(
        &self,
        endpoint_id: &str,
        remote_port: u16,
    ) -> Result<String, String> {
        let id: EndpointId = endpoint_id
            .parse()
            .map_err(|e| format!("Invalid endpoint id: {e}"))?;
        let tunnel =
            tcp_proxy::open_tunnel(self.endpoint.clone(), id, remote_port).await?;
        let addr = tunnel.local_tcp_address();
        log::info!(
            "iroh tcp-proxy: tunnel {} -> {endpoint_id}:{remote_port}",
            addr
        );
        self.tunnels.write().await.push(tunnel);
        Ok(addr)
    }

    pub async fn shutdown(&self) {
        self.tunnels.write().await.clear();
        if let Err(e) = self.router.shutdown().await {
            log::warn!("iroh: router shutdown: {e}");
        }
    }
}

fn resolve_bootstrap_endpoint_ids() -> Vec<EndpointId> {
    let raw = std::env::var("CLUSTER_RUNTIME_IROH_BOOTSTRAP")
        .or_else(|_| std::env::var("CLUSTER_RUNTIME_P2P_BOOTSTRAP"))
        .unwrap_or_default();
    raw.split(',')
        .filter_map(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                match t.parse::<EndpointId>() {
                    Ok(id) => Some(id),
                    Err(e) => {
                        log::warn!("iroh: skipping invalid bootstrap id '{t}': {e}");
                        None
                    }
                }
            }
        })
        .collect()
}
