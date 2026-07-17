//! Node discovery module
//! 
//! Implements automatic LAN discovery using UDP broadcast/multicast

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use serde::{Deserialize, Serialize};

/// Discovery message sent over UDP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryMessage {
    pub node_id: String,
    pub node_name: String,
    pub runtime_port: u16,
    pub timestamp: i64,
}

/// Discovered peer information
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub node_id: String,
    pub node_name: String,
    pub address: SocketAddr,
    pub runtime_port: u16,
    pub last_seen: i64,
    pub connected: bool,
}

/// Discovery service for LAN node discovery
pub struct Discovery {
    config: DiscoveryConfig,
    socket: Arc<UdpSocket>,
    peers: Arc<RwLock<HashMap<String, DiscoveredPeer>>>,
    node_id: String,
    node_name: String,
    runtime_port: u16,
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// UDP port for discovery
    pub discovery_port: u16,
    /// Discovery interval in seconds
    pub discovery_interval: u64,
    /// Peer timeout in seconds (remove after this many seconds of no response)
    pub peer_timeout: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            discovery_port: 54322,
            discovery_interval: 10,
            peer_timeout: 30,
        }
    }
}

impl Discovery {
    /// Create a new discovery service
    pub async fn new(
        config: DiscoveryConfig,
        node_id: String,
        node_name: String,
        runtime_port: u16,
    ) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", config.discovery_port))?;
        socket.set_broadcast(true)?;
        
        // Allow reuse of the address
        #[cfg(unix)]
        socket.set_reuse_address(true)?;
        
        Ok(Self {
            config,
            socket: Arc::new(socket),
            peers: Arc::new(RwLock::new(HashMap::new())),
            node_id,
            node_name,
            runtime_port,
        })
    }

    /// Start the discovery service
    pub async fn start(&self) -> Result<(), std::io::Error> {
        let socket = self.socket.clone();
        let peers = self.peers.clone();
        let node_id = self.node_id.clone();
        let node_name = self.node_name.clone();
        let runtime_port = self.runtime_port;
        let discovery_port = self.config.discovery_port;
        let broadcast_addr = "255.255.255.255";
        
        // Spawn broadcast task
        let broadcast_socket = socket.clone();
        let broadcast_node_id = node_id.clone();
        let broadcast_node_name = node_name.clone();
        let broadcast_runtime_port = runtime_port;
        let broadcast_interval = self.config.discovery_interval;
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(broadcast_interval));
            loop {
                interval.tick().await;
                
                let message = DiscoveryMessage {
                    node_id: broadcast_node_id.clone(),
                    node_name: broadcast_node_name.clone(),
                    runtime_port: broadcast_runtime_port,
                    timestamp: chrono::Utc::now().timestamp(),
                };
                
                if let Ok(data) = serde_json::to_vec(&message) {
                    // Broadcast to all hosts on the network
                    let addr = format!("{}:{}", broadcast_addr, discovery_port);
                    let _ = broadcast_socket.send_to(&data, addr);
                    
                    // Also send to localhost for local testing
                    let local_addr = format!("127.0.0.1:{}", discovery_port);
                    let _ = broadcast_socket.send_to(&data, local_addr);
                }
            }
        });
        
        // Spawn receive task
        let recv_socket = socket.clone();
        let recv_peers = peers.clone();
        let recv_node_id = node_id;
        
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match recv_socket.recv_from(&mut buf) {
                    Ok((size, addr)) => {
                        // Ignore packets from self
                        if addr.ip().is_loopback() {
                            // Still process localhost packets for testing
                        }
                        
                        if let Ok(message) = serde_json::from_slice::<DiscoveryMessage>(&buf[..size]) {
                            // Ignore self discovery
                            if message.node_id != recv_node_id {
                                let peer = DiscoveredPeer {
                                    node_id: message.node_id.clone(),
                                    node_name: message.node_name,
                                    address: SocketAddr::new(
                                        addr.ip(),
                                        message.runtime_port,
                                    ),
                                    runtime_port: message.runtime_port,
                                    last_seen: chrono::Utc::now().timestamp(),
                                    connected: false,
                                };
                                
                                let mut peers = recv_peers.blocking_write();
                                peers.insert(message.node_id, peer);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Discovery receive error: {}", e);
                    }
                }
            }
        });
        
        // Spawn cleanup task for stale peers
        let cleanup_peers = self.peers.clone();
        let timeout = self.config.peer_timeout;
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(timeout / 2));
            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp();
                let mut peers = cleanup_peers.blocking_write();
                peers.retain(|_, peer| now - peer.last_seen < timeout as i64);
            }
        });
        
        Ok(())
    }

    /// Get all discovered peers
    pub async fn get_peers(&self) -> Vec<DiscoveredPeer> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Mark a peer as connected
    pub async fn mark_connected(&self, node_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.connected = true;
        }
    }

    /// Mark a peer as disconnected
    pub async fn mark_disconnected(&self, node_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.connected = false;
        }
    }

    /// Remove a peer
    pub async fn remove_peer(&self, node_id: &str) {
        let mut peers = self.peers.write().await;
        peers.remove(node_id);
    }

    /// Get a specific peer by node ID
    pub async fn get_peer(&self, node_id: &str) -> Option<DiscoveredPeer> {
        let peers = self.peers.read().await;
        peers.get(node_id).cloned()
    }
}

/// Discovery event
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    PeerDiscovered(DiscoveredPeer),
    PeerUpdated(DiscoveredPeer),
    PeerRemoved(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discovery_creation() {
        let config = DiscoveryConfig::default();
        let discovery = Discovery::new(
            config,
            "test-node-1".to_string(),
            "Test Node 1".to_string(),
            54321,
        ).await;
        
        assert!(discovery.is_ok());
    }
}
