//! Cluster protocol messages
//! 
//! Defines all message types for peer-to-peer cluster communication

use serde::{Deserialize, Serialize};

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Protocol version wrapper
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProtocolVersion(pub u32);

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self(PROTOCOL_VERSION)
    }
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Message {
    /// Handshake message
    Hello(HelloMessage),
    /// Authentication request
    Authenticate(AuthMessage),
    /// Authentication response
    AuthResponse(AuthResponse),
    /// Heartbeat message
    Heartbeat(HeartbeatMessage),
    /// Node information
    NodeInfo(NodeInfoMessage),
    /// Resource update
    ResourceUpdate(ResourceUpdateMessage),
    /// Node joined notification
    NodeJoined(NodeJoinedMessage),
    /// Node left notification
    NodeLeft(NodeLeftMessage),
    /// Ping message
    Ping(PingMessage),
    /// Pong response
    Pong(PongMessage),
    /// Full cluster state
    ClusterState(ClusterStateMessage),
    /// Worker update
    WorkerUpdate(WorkerUpdateMessage),
    /// Shutdown signal
    Shutdown(ShutdownMessage),
}

/// Hello handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloMessage {
    pub node_id: String,
    pub node_name: String,
    pub protocol_version: u32,
    pub public_key: String,
}

/// Authentication message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMessage {
    pub node_id: String,
    pub signature: String, // Signed challenge
    pub timestamp: i64,
}

/// Authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub error: Option<String>,
    pub session_id: Option<String>,
}

/// Heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub node_id: String,
    pub timestamp: i64,
    pub cpu_usage: f32,
    pub ram_usage: u64,
    pub ram_total: u64,
    pub worker_count: usize,
    pub active_jobs: usize,
    pub status: String,
}

/// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoMessage {
    pub node_id: String,
    pub node_name: String,
    pub host: String,
    pub port: u16,
    pub version: String,
    pub cpu_cores: usize,
    pub ram_total: u64,
    pub gpu_count: usize,
    pub max_workers: usize,
}

/// Resource update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUpdateMessage {
    pub node_id: String,
    pub cpu_usage: f32,
    pub ram_used: u64,
    pub ram_available: u64,
    pub worker_count: usize,
    pub active_jobs: usize,
}

/// Node joined notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeJoinedMessage {
    pub node_id: String,
    pub node_name: String,
    pub host: String,
    pub port: u16,
    pub version: String,
}

/// Node left notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLeftMessage {
    pub node_id: String,
    pub reason: String,
}

/// Ping message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    pub node_id: String,
    pub timestamp: i64,
}

/// Pong response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    pub node_id: String,
    pub timestamp: i64,
    pub original_timestamp: i64,
}

/// Full cluster state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStateMessage {
    pub node_id: String,
    pub nodes: Vec<NodeState>,
    pub timestamp: i64,
}

/// Node state in cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    pub node_id: String,
    pub node_name: String,
    pub host: String,
    pub port: u16,
    pub status: String,
    pub cpu_cores: usize,
    pub cpu_usage: f32,
    pub ram_total: u64,
    pub ram_used: u64,
    pub worker_count: usize,
    pub active_jobs: usize,
    pub version: String,
    pub last_seen: i64,
}

/// Worker update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerUpdateMessage {
    pub node_id: String,
    pub worker_count: usize,
    pub available_workers: usize,
}

/// Shutdown message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownMessage {
    pub node_id: String,
    pub reason: String,
}

impl Message {
    /// Create a new hello message
    pub fn hello(node_id: String, node_name: String, public_key: String) -> Self {
        Message::Hello(HelloMessage {
            node_id,
            node_name,
            protocol_version: PROTOCOL_VERSION,
            public_key,
        })
    }

    /// Create a new heartbeat message
    pub fn heartbeat(
        node_id: String,
        cpu_usage: f32,
        ram_usage: u64,
        ram_total: u64,
        worker_count: usize,
        active_jobs: usize,
        status: String,
    ) -> Self {
        Message::Heartbeat(HeartbeatMessage {
            node_id,
            timestamp: chrono::Utc::now().timestamp(),
            cpu_usage,
            ram_usage,
            ram_total,
            worker_count,
            active_jobs,
            status,
        })
    }

    /// Create a new ping message
    pub fn ping(node_id: String) -> Self {
        Message::Ping(PingMessage {
            node_id,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Create a new pong message
    pub fn pong(node_id: String, original_timestamp: i64) -> Self {
        Message::Pong(PongMessage {
            node_id,
            timestamp: chrono::Utc::now().timestamp(),
            original_timestamp,
        })
    }

    /// Get message type as string
    pub fn message_type(&self) -> &'static str {
        match self {
            Message::Hello(_) => "Hello",
            Message::Authenticate(_) => "Authenticate",
            Message::AuthResponse(_) => "AuthResponse",
            Message::Heartbeat(_) => "Heartbeat",
            Message::NodeInfo(_) => "NodeInfo",
            Message::ResourceUpdate(_) => "ResourceUpdate",
            Message::NodeJoined(_) => "NodeJoined",
            Message::NodeLeft(_) => "NodeLeft",
            Message::Ping(_) => "Ping",
            Message::Pong(_) => "Pong",
            Message::ClusterState(_) => "ClusterState",
            Message::WorkerUpdate(_) => "WorkerUpdate",
            Message::Shutdown(_) => "Shutdown",
        }
    }
}
