//! libp2p swarm service: listen, dial bootstrap peers, relay jobs.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use futures::StreamExt;
use libp2p::identify;
use libp2p::multiaddr::Protocol;
use libp2p::request_response::{Event as RrEvent, Message};
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{Multiaddr, PeerId, SwarmBuilder};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::jobs::JobApi;
use crate::network::{NodeStatus, PeerInfo, ResourceInfo};

use super::behaviour::{ClusterBehaviour, ClusterBehaviourEvent};
use super::identity;
use super::protocol::{RemoteJobRequest, RemoteJobResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerSnapshot {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub connected: bool,
}

enum Command {
    Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
    Request {
        peer: PeerId,
        request: RemoteJobRequest,
        reply: oneshot::Sender<Result<RemoteJobResponse, String>>,
    },
    ListPeers(oneshot::Sender<Vec<PeerInfo>>),
    LocalPeerId(oneshot::Sender<String>),
    ListenAddrs(oneshot::Sender<Vec<String>>),
    Shutdown,
}

/// WAN P2P mesh handle.
pub struct P2pService {
    cmd_tx: mpsc::Sender<Command>,
    local_peer_id: String,
}

impl P2pService {
    /// Start the swarm in a background task. Returns immediately.
    pub async fn start(
        data_dir: &Path,
        job_api: Arc<JobApi>,
    ) -> Result<Arc<Self>, String> {
        let (keypair, peer_id) = identity::load_or_generate(data_dir)?;
        let local_peer_id = peer_id.to_string();
        let node_name = std::env::var("CLUSTER_RUNTIME_NODE_NAME")
            .unwrap_or_else(|_| "cluster-node".into());

        let mut swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| e.to_string())?
            .with_websocket(
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .await
            .map_err(|e| e.to_string())?
            .with_behaviour(|key| ClusterBehaviour::new(key.public()))
            .map_err(|e| e.to_string())?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let listen_addrs = resolve_listen_addrs();
        for addr in &listen_addrs {
            match swarm.listen_on(addr.clone()) {
                Ok(_) => log::info!("P2P: listening on {addr}"),
                Err(e) => log::warn!("P2P: failed to listen on {addr}: {e}"),
            }
        }

        for addr in resolve_bootstrap_addrs() {
            log::info!("P2P: dialing bootstrap {addr}");
            if let Err(e) = swarm.dial(addr.clone()) {
                log::warn!("P2P: bootstrap dial failed for {addr}: {e}");
            }
        }

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(64);
        let peers: Arc<RwLock<HashMap<PeerId, PeerRecord>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let pending: Arc<
            RwLock<HashMap<libp2p::request_response::OutboundRequestId, oneshot::Sender<Result<RemoteJobResponse, String>>>>,
        > = Arc::new(RwLock::new(HashMap::new()));

        let peers_task = peers.clone();
        let pending_task = pending.clone();
        let local_id_for_task = peer_id;
        let listen_reported = Arc::new(RwLock::new(Vec::<Multiaddr>::new()));
        let listen_reported_task = listen_reported.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = swarm.select_next_some() => {
                        handle_swarm_event(
                            &mut swarm,
                            event,
                            &peers_task,
                            &pending_task,
                            &job_api,
                            &node_name,
                            local_id_for_task,
                            &listen_reported_task,
                        ).await;
                    }
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(Command::Dial(addr, reply)) => {
                                let r = swarm.dial(addr).map(|_| ()).map_err(|e| e.to_string());
                                let _ = reply.send(r);
                            }
                            Some(Command::Request { peer, request, reply }) => {
                                let id = swarm.behaviour_mut().jobs.send_request(&peer, request);
                                pending_task.write().await.insert(id, reply);
                            }
                            Some(Command::ListPeers(reply)) => {
                                let list = peers_task.read().await
                                    .values()
                                    .map(PeerRecord::to_peer_info)
                                    .collect();
                                let _ = reply.send(list);
                            }
                            Some(Command::LocalPeerId(reply)) => {
                                let _ = reply.send(local_id_for_task.to_string());
                            }
                            Some(Command::ListenAddrs(reply)) => {
                                let addrs = listen_reported_task.read().await
                                    .iter()
                                    .map(|a| a.to_string())
                                    .collect();
                                let _ = reply.send(addrs);
                            }
                            Some(Command::Shutdown) | None => {
                                log::info!("P2P: swarm task stopping");
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Arc::new(Self {
            cmd_tx,
            local_peer_id,
        }))
    }

    pub fn local_peer_id(&self) -> &str {
        &self.local_peer_id
    }

    pub async fn connect(&self, multiaddr: &str) -> Result<(), String> {
        let addr: Multiaddr = multiaddr.parse().map_err(|e: libp2p::multiaddr::Error| e.to_string())?;
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Dial(addr, tx))
            .await
            .map_err(|_| "P2P swarm stopped".to_string())?;
        rx.await.map_err(|_| "P2P dial dropped".to_string())?
    }

    pub async fn list_peers(&self) -> Result<Vec<PeerInfo>, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::ListPeers(tx))
            .await
            .map_err(|_| "P2P swarm stopped".to_string())?;
        rx.await.map_err(|_| "P2P list dropped".to_string())
    }

    pub async fn listen_addrs(&self) -> Result<Vec<String>, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::ListenAddrs(tx))
            .await
            .map_err(|_| "P2P swarm stopped".to_string())?;
        rx.await.map_err(|_| "P2P listen addrs dropped".to_string())
    }

    pub async fn remote_request(
        &self,
        peer_id: &str,
        request: RemoteJobRequest,
    ) -> Result<RemoteJobResponse, String> {
        let peer: PeerId = peer_id.parse().map_err(|e| format!("Invalid peer id: {e}"))?;
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Request {
                peer,
                request,
                reply: tx,
            })
            .await
            .map_err(|_| "P2P swarm stopped".to_string())?;
        rx.await.map_err(|_| "P2P request dropped".to_string())?
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

    pub async fn shutdown(&self) {
        let _ = self.cmd_tx.send(Command::Shutdown).await;
    }
}

#[derive(Clone)]
struct PeerRecord {
    peer_id: PeerId,
    addresses: Vec<Multiaddr>,
    connected_since: chrono::DateTime<Utc>,
    last_seen: chrono::DateTime<Utc>,
}

impl PeerRecord {
    fn to_peer_info(&self) -> PeerInfo {
        let (host, port) = extract_host_port(self.addresses.first());
        PeerInfo {
            node_id: self.peer_id.to_string(),
            node_name: self.peer_id.to_string(),
            host,
            port,
            status: NodeStatus::Online,
            resources: ResourceInfo::default(),
            version: "1.0.0".into(),
            connected_since: self.connected_since,
            last_heartbeat: self.last_seen,
            latency_ms: 0,
        }
    }
}

fn extract_host_port(addr: Option<&Multiaddr>) -> (String, u16) {
    let Some(addr) = addr else {
        return ("0.0.0.0".into(), 0);
    };
    let mut host = "0.0.0.0".to_string();
    let mut port = 0u16;
    for p in addr.iter() {
        match p {
            Protocol::Ip4(ip) => host = ip.to_string(),
            Protocol::Ip6(ip) => host = ip.to_string(),
            Protocol::Tcp(p) | Protocol::Udp(p) => port = p,
            _ => {}
        }
    }
    (host, port)
}

fn resolve_listen_addrs() -> Vec<Multiaddr> {
    if let Ok(raw) = std::env::var("CLUSTER_RUNTIME_P2P_LISTEN") {
        return raw
            .split(',')
            .filter_map(|s| {
                let t = s.trim();
                if t.is_empty() {
                    None
                } else {
                    t.parse().ok()
                }
            })
            .collect();
    }
    // Firewall-friendly defaults: WS on 8080, then try 80 / 443.
    let defaults = [
        "/ip4/0.0.0.0/tcp/8080/ws",
        "/ip4/0.0.0.0/tcp/80/ws",
        "/ip4/0.0.0.0/tcp/443/ws",
    ];
    defaults.iter().filter_map(|s| s.parse().ok()).collect()
}

fn resolve_bootstrap_addrs() -> Vec<Multiaddr> {
    let Ok(raw) = std::env::var("CLUSTER_RUNTIME_P2P_BOOTSTRAP") else {
        return Vec::new();
    };
    raw.split(',')
        .filter_map(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        })
        .collect()
}

async fn handle_swarm_event(
    swarm: &mut Swarm<ClusterBehaviour>,
    event: SwarmEvent<ClusterBehaviourEvent>,
    peers: &Arc<RwLock<HashMap<PeerId, PeerRecord>>>,
    pending: &Arc<
        RwLock<
            HashMap<
                libp2p::request_response::OutboundRequestId,
                oneshot::Sender<Result<RemoteJobResponse, String>>,
            >,
        >,
    >,
    job_api: &Arc<JobApi>,
    node_name: &str,
    local_peer_id: PeerId,
    listen_reported: &Arc<RwLock<Vec<Multiaddr>>>,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            log::info!("P2P: new listen address {address}");
            listen_reported.write().await.push(address);
        }
        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
            let addr = endpoint.get_remote_address().clone();
            let mut map = peers.write().await;
            let now = Utc::now();
            map.entry(peer_id)
                .and_modify(|r| {
                    if !r.addresses.contains(&addr) {
                        r.addresses.push(addr.clone());
                    }
                    r.last_seen = now;
                })
                .or_insert(PeerRecord {
                    peer_id,
                    addresses: vec![addr],
                    connected_since: now,
                    last_seen: now,
                });
            log::info!("P2P: connected to {peer_id}");
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            peers.write().await.remove(&peer_id);
            log::info!("P2P: disconnected from {peer_id}");
        }
        SwarmEvent::Behaviour(ClusterBehaviourEvent::Jobs(RrEvent::Message { peer, message })) => {
            match message {
                Message::Request {
                    request, channel, ..
                } => {
                    let response =
                        handle_remote_request(job_api, node_name, local_peer_id, request).await;
                    let _ = swarm.behaviour_mut().jobs.send_response(channel, response);
                }
                Message::Response { request_id, response } => {
                    if let Some(tx) = pending.write().await.remove(&request_id) {
                        let _ = tx.send(Ok(response));
                    } else {
                        let _ = peer;
                    }
                }
            }
        }
        SwarmEvent::Behaviour(ClusterBehaviourEvent::Jobs(RrEvent::OutboundFailure {
            request_id,
            error,
            ..
        })) => {
            if let Some(tx) = pending.write().await.remove(&request_id) {
                let _ = tx.send(Err(error.to_string()));
            }
        }
        SwarmEvent::Behaviour(ClusterBehaviourEvent::Identify(ev)) => {
            if let identify::Event::Received { peer_id, info, .. } = ev {
                let mut map = peers.write().await;
                let now = Utc::now();
                map.entry(peer_id)
                    .and_modify(|r| {
                        for a in &info.listen_addrs {
                            if !r.addresses.contains(a) {
                                r.addresses.push(a.clone());
                            }
                        }
                        r.last_seen = now;
                    })
                    .or_insert(PeerRecord {
                        peer_id,
                        addresses: info.listen_addrs.clone(),
                        connected_since: now,
                        last_seen: now,
                    });
            }
        }
        _ => {}
    }
}

async fn handle_remote_request(
    job_api: &Arc<JobApi>,
    node_name: &str,
    local_peer_id: PeerId,
    request: RemoteJobRequest,
) -> RemoteJobResponse {
    match request {
        RemoteJobRequest::Hello { .. } => RemoteJobResponse::Hello {
            peer_id: local_peer_id.to_string(),
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
