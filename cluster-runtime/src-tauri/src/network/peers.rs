//! Peer management module
//! 
//! Manages peer connections, tracking, and state

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use crate::network::{NodeStatus, PeerInfo};

/// Peer connection state
#[derive(Debug, Clone)]
pub struct PeerConnection {
    pub peer_id: String,
    pub address: String,
    pub port: u16,
    pub connected: bool,
    pub connected_since: Option<DateTime<Utc>>,
    pub last_seen: DateTime<Utc>,
    pub latency_ms: u32,
}

impl PeerConnection {
    pub fn new(peer_id: String, address: String, port: u16) -> Self {
        Self {
            peer_id,
            address,
            port,
            connected: false,
            connected_since: None,
            last_seen: Utc::now(),
            latency_ms: 0,
        }
    }

    pub fn mark_connected(&mut self) {
        self.connected = true;
        self.connected_since = Some(Utc::now());
        self.last_seen = Utc::now();
    }

    pub fn mark_disconnected(&mut self) {
        self.connected = false;
        self.connected_since = None;
        self.last_seen = Utc::now();
    }

    pub fn update_latency(&mut self, latency_ms: u32) {
        self.latency_ms = latency_ms;
        self.last_seen = Utc::now();
    }
}

/// Peer manager for tracking all peers in the cluster
pub struct PeerManager {
    /// Known peers by node ID
    peers: Arc<RwLock<HashMap<String, PeerConnection>>>,
    /// Peer information (resources, status, etc.)
    peer_info: Arc<RwLock<HashMap<String, PeerInfo>>>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            peer_info: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new peer
    pub async fn add_peer(&self, node_id: String, address: String, port: u16) {
        let mut peers = self.peers.write().await;
        peers.entry(node_id.clone()).or_insert(PeerConnection::new(
            node_id,
            address,
            port,
        ));
    }

    /// Remove a peer
    pub async fn remove_peer(&self, node_id: &str) {
        let mut peers = self.peers.write().await;
        peers.remove(node_id);
        let mut peer_info = self.peer_info.write().await;
        peer_info.remove(node_id);
    }

    /// Mark a peer as connected
    pub async fn mark_connected(&self, node_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.mark_connected();
        }
    }

    /// Mark a peer as disconnected
    pub async fn mark_disconnected(&self, node_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.mark_disconnected();
        }
    }

    /// Update peer latency
    pub async fn update_latency(&self, node_id: &str, latency_ms: u32) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.update_latency(latency_ms);
        }
    }

    /// Update peer information
    pub async fn update_peer_info(&self, node_id: String, peer_info: PeerInfo) {
        let mut info = self.peer_info.write().await;
        info.insert(node_id, peer_info);
    }

    /// Get a peer connection
    pub async fn get_peer(&self, node_id: &str) -> Option<PeerConnection> {
        let peers = self.peers.read().await;
        peers.get(node_id).cloned()
    }

    /// Get all peers
    pub async fn get_peers(&self) -> Vec<PeerConnection> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Get connected peers
    pub async fn get_connected_peers(&self) -> Vec<PeerConnection> {
        let peers = self.peers.read().await;
        peers.values()
            .filter(|p| p.connected)
            .cloned()
            .collect()
    }

    /// Get peer information
    pub async fn get_peer_info(&self, node_id: &str) -> Option<PeerInfo> {
        let info = self.peer_info.read().await;
        info.get(node_id).cloned()
    }

    /// Get all peer information
    pub async fn get_all_peer_info(&self) -> Vec<PeerInfo> {
        let info = self.peer_info.read().await;
        info.values().cloned().collect()
    }

    /// Check if a peer exists
    pub async fn has_peer(&self, node_id: &str) -> bool {
        let peers = self.peers.read().await;
        peers.contains_key(node_id)
    }

    /// Get the number of connected peers
    pub async fn connected_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.values().filter(|p| p.connected).count()
    }

    /// Get the total number of peers
    pub async fn total_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.len()
    }

    /// Clear all peers (for testing or shutdown)
    pub async fn clear(&self) {
        let mut peers = self.peers.write().await;
        peers.clear();
        let mut info = self.peer_info.write().await;
        info.clear();
    }

    /// Get peers by status
    pub async fn get_peers_by_status(&self, status: NodeStatus) -> Vec<PeerConnection> {
        let info = self.peer_info.read().await;
        let mut result = Vec::new();
        let peers = self.peers.read().await;
        
        for (node_id, peer) in peers.iter() {
            if let Some(peer_info) = info.get(node_id) {
                if peer_info.status == status {
                    result.push(peer.clone());
                }
            }
        }
        
        result
    }

    /// Get online peers
    pub async fn get_online_peers(&self) -> Vec<PeerConnection> {
        self.get_peers_by_status(NodeStatus::Online).await
    }
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Peer event
#[derive(Debug, Clone)]
pub enum PeerEvent {
    PeerAdded(String),
    PeerConnected(String),
    PeerDisconnected(String),
    PeerRemoved(String),
    PeerUpdated(String),
}