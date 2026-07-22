//! Cluster management module
//! 
//! Manages cluster state, node membership, and resource synchronization

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use crate::network::{
    NetworkConfig, NetworkError, Result,
    NodeIdentity, NodeStatus, ResourceInfo, PeerInfo, ClusterSummary,
    discovery::Discovery,
    heartbeat::HeartbeatManager,
    authentication::{Authenticator, SessionManager},
    peers::PeerManager,
};

/// Cluster manager - central coordination for cluster operations
pub struct ClusterManager {
    /// Local node identity
    local_node: Arc<RwLock<NodeIdentity>>,
    /// Cluster configuration
    config: NetworkConfig,
    /// Discovery service
    discovery: Arc<Discovery>,
    /// Heartbeat manager
    heartbeat_manager: Arc<HeartbeatManager>,
    /// Authenticator
    authenticator: Arc<Authenticator>,
    /// Session manager
    session_manager: Arc<SessionManager>,
    /// Peer manager
    peer_manager: Arc<PeerManager>,
    /// Cluster state
    cluster_state: Arc<RwLock<ClusterState>>,
}

/// Cluster state
#[derive(Debug, Clone)]
pub struct ClusterState {
    pub nodes: HashMap<String, PeerInfo>,
    pub local_node_id: String,
    pub updated_at: DateTime<Utc>,
}

impl ClusterState {
    pub fn new(local_node_id: String) -> Self {
        Self {
            nodes: HashMap::new(),
            local_node_id,
            updated_at: Utc::now(),
        }
    }

    pub fn add_node(&mut self, peer: PeerInfo) {
        self.nodes.insert(peer.node_id.clone(), peer);
        self.updated_at = Utc::now();
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.remove(node_id);
        self.updated_at = Utc::now();
    }

    pub fn update_node(&mut self, peer: PeerInfo) {
        if let Some(existing) = self.nodes.get_mut(&peer.node_id) {
            *existing = peer;
        } else {
            self.nodes.insert(peer.node_id.clone(), peer);
        }
        self.updated_at = Utc::now();
    }

    pub fn get_node(&self, node_id: &str) -> Option<&PeerInfo> {
        self.nodes.get(node_id)
    }

    pub fn get_online_nodes(&self) -> Vec<&PeerInfo> {
        self.nodes
            .values()
            .filter(|p| p.status == NodeStatus::Online)
            .collect()
    }

    pub fn get_summary(&self) -> ClusterSummary {
        let online_nodes: Vec<&PeerInfo> = self.get_online_nodes();
        let total_nodes = self.nodes.len();
        let online_count = online_nodes.len();
        
        let mut total_cpus = 0;
        let mut total_ram = 0;
        let mut total_gpus = 0;
        let mut total_workers = 0;
        let mut total_available = 0.0;
        
        for node in online_nodes {
            total_cpus += node.resources.cpu_cores;
            total_ram += node.resources.ram_available;
            total_gpus += node.resources.gpu_count;
            total_workers += node.resources.worker_count;
            total_available += node.resources.cpu_usage;
        }

        ClusterSummary {
            total_nodes,
            online_nodes: online_count,
            total_cpus,
            total_ram,
            total_gpus,
            total_workers,
            total_available_compute: total_available / (online_count as f32).max(1.0),
        }
    }
}

impl ClusterManager {
    /// Create a new cluster manager
    pub async fn new(
        config: NetworkConfig,
        node_id: String,
        node_name: String,
        host: String,
        public_key: String,
    ) -> Result<Self> {
        let node_identity = NodeIdentity {
            node_id: node_id.clone(),
            public_key,
            node_name: node_name.clone(),
            host: host.clone(),
            port: config.runtime_port,
        };

        let discovery = Arc::new(
            Discovery::new(
                crate::network::discovery::DiscoveryConfig {
                    discovery_port: config.discovery_port,
                    discovery_interval: config.discovery_interval,
                    peer_timeout: 60,
                },
                node_id.clone(),
                node_name.clone(),
                config.runtime_port,
            )
            .await
            .map_err(|e| NetworkError::Discovery(e.to_string()))?
        );

        let heartbeat_manager = Arc::new(HeartbeatManager::new(
            config.heartbeat_interval,
            config.heartbeat_interval * 6, // timeout after 6 missed heartbeats
        ));

        let authenticator = Arc::new(Authenticator::new());
        let session_manager = Arc::new(SessionManager::new());
        let peer_manager = Arc::new(PeerManager::new());

        let cluster_state = Arc::new(RwLock::new(ClusterState::new(node_id.clone())));

        Ok(Self {
            local_node: Arc::new(RwLock::new(node_identity)),
            config,
            discovery,
            heartbeat_manager,
            authenticator,
            session_manager,
            peer_manager,
            cluster_state,
        })
    }

    /// Start the cluster manager
    pub async fn start(&self) -> Result<()> {
        // Start discovery service
        self.discovery.start().await
            .map_err(|e| NetworkError::Discovery(e.to_string()))?;

        // Register local node with heartbeat manager
        let local_node = self.local_node.read().await;
        self.heartbeat_manager.register_node(local_node.node_id.clone()).await;

        Ok(())
    }

    /// Get local node identity
    pub async fn get_local_node(&self) -> NodeIdentity {
        self.local_node.read().await.clone()
    }

    /// Get cluster state
    pub async fn get_cluster_state(&self) -> ClusterState {
        self.cluster_state.read().await.clone()
    }

    /// Get cluster summary
    pub async fn get_summary(&self) -> ClusterSummary {
        self.cluster_state.read().await.get_summary()
    }

    /// Update node status
    pub async fn update_node_status(&self, node_id: &str, status: NodeStatus) {
        let mut state = self.cluster_state.write().await;
        if let Some(node) = state.nodes.get_mut(node_id) {
            node.status = status;
        }
    }

    /// Add a peer to the cluster
    pub async fn add_peer(&self, peer_info: PeerInfo) {
        let mut state = self.cluster_state.write().await;
        state.add_node(peer_info);
    }

    /// Remove a peer from the cluster
    pub async fn remove_peer(&self, node_id: &str) {
        let mut state = self.cluster_state.write().await;
        state.remove_node(node_id);
        self.peer_manager.remove_peer(node_id).await;
        self.heartbeat_manager.remove_node(node_id).await;
    }

    /// Get a specific peer
    pub async fn get_peer(&self, node_id: &str) -> Option<PeerInfo> {
        let state = self.cluster_state.read().await;
        state.get_node(node_id).cloned()
    }

    /// Get all peers
    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let state = self.cluster_state.read().await;
        state.nodes.values().cloned().collect()
    }

    /// Get online peers
    pub async fn get_online_peers(&self) -> Vec<PeerInfo> {
        let state = self.cluster_state.read().await;
        state.get_online_nodes().into_iter().map(|p| (*p).clone()).collect()
    }

    /// Synchronize resources with a peer
    pub async fn sync_resources(&self, node_id: &str, resources: ResourceInfo) {
        let mut state = self.cluster_state.write().await;
        if let Some(node) = state.nodes.get_mut(node_id) {
            node.resources = resources;
            node.last_heartbeat = Utc::now();
        }
    }

    /// Get the authenticator
    pub fn authenticator(&self) -> Arc<Authenticator> {
        self.authenticator.clone()
    }

    /// Get the session manager
    pub fn session_manager(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }

    /// Get the peer manager
    pub fn peer_manager(&self) -> Arc<PeerManager> {
        self.peer_manager.clone()
    }

    /// Get the heartbeat manager
    pub fn heartbeat_manager(&self) -> Arc<HeartbeatManager> {
        self.heartbeat_manager.clone()
    }

    /// Get the discovery service
    pub fn discovery(&self) -> Arc<Discovery> {
        self.discovery.clone()
    }

    /// Get configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }
}

impl Clone for ClusterManager {
    fn clone(&self) -> Self {
        Self {
            local_node: self.local_node.clone(),
            config: self.config.clone(),
            discovery: self.discovery.clone(),
            heartbeat_manager: self.heartbeat_manager.clone(),
            authenticator: self.authenticator.clone(),
            session_manager: self.session_manager.clone(),
            peer_manager: self.peer_manager.clone(),
            cluster_state: self.cluster_state.clone(),
        }
    }
}
