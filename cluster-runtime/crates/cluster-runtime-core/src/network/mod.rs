//! Network layer for cluster communication
//!
//! Legacy UDP/TCP modules remain for reference; the active WAN plane is
//! [`p2p`] (libp2p on firewall-friendly ports).

pub mod authentication;
pub mod cluster;
pub mod discovery;
pub mod heartbeat;
pub mod messages;
pub mod p2p;
pub mod peers;
pub mod transport;




/// Configuration for the network layer
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    /// TCP port for runtime communication
    pub runtime_port: u16,
    /// UDP port for node discovery
    pub discovery_port: u16,
    /// Node name (friendly name)
    pub node_name: String,
    /// Whether to advertise compute resources
    pub advertise_compute: bool,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
    /// Discovery interval in seconds
    pub discovery_interval: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            runtime_port: 54321,
            discovery_port: 54322,
            node_name: "cluster-node".to_string(),
            advertise_compute: true,
            heartbeat_interval: 5,
            discovery_interval: 10,
        }
    }
}

/// Error types for the network layer
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Authentication failed: {0}")]
    Authentication(String),
    #[error("Connection refused")]
    ConnectionRefused,
    #[error("Connection timeout")]
    ConnectionTimeout,
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    #[error("Protocol version mismatch")]
    VersionMismatch,
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Discovery error: {0}")]
    Discovery(String),
    #[error("Heartbeat timeout: {0}")]
    HeartbeatTimeout(String),
}

pub type Result<T> = std::result::Result<T, NetworkError>;

/// Node identity
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeIdentity {
    pub node_id: String,
    pub public_key: String,
    pub node_name: String,
    pub host: String,
    pub port: u16,
}

/// Node status
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NodeStatus {
    Online,
    Offline,
    Connecting,
    Authenticating,
    Disconnected,
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self::Offline
    }
}

/// Resource information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceInfo {
    pub cpu_cores: usize,
    pub cpu_usage: f32,
    pub ram_total: u64,
    pub ram_used: u64,
    pub ram_available: u64,
    pub gpu_count: usize,
    pub worker_count: usize,
    pub active_jobs: usize,
}

impl Default for ResourceInfo {
    fn default() -> Self {
        Self {
            cpu_cores: 0,
            cpu_usage: 0.0,
            ram_total: 0,
            ram_used: 0,
            ram_available: 0,
            gpu_count: 0,
            worker_count: 0,
            active_jobs: 0,
        }
    }
}

/// Peer information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerInfo {
    pub node_id: String,
    pub node_name: String,
    pub host: String,
    pub port: u16,
    pub status: NodeStatus,
    pub resources: ResourceInfo,
    pub version: String,
    pub connected_since: chrono::DateTime<chrono::Utc>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub latency_ms: u32,
}

/// Cluster summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClusterSummary {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub total_cpus: usize,
    pub total_ram: u64,
    pub total_gpus: usize,
    pub total_workers: usize,
    pub total_available_compute: f32,
}
