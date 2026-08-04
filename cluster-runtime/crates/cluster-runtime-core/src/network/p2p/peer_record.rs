//! Connected peer bookkeeping for the iroh mesh.

use chrono::{DateTime, Utc};
use iroh::EndpointId;

use crate::network::{NodeStatus, PeerInfo, ResourceInfo};

#[derive(Clone, Debug)]
pub struct PeerRecord {
    pub endpoint_id: EndpointId,
    pub node_name: String,
    pub connected_since: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

impl PeerRecord {
    pub fn to_peer_info(&self) -> PeerInfo {
        PeerInfo {
            node_id: self.endpoint_id.to_string(),
            node_name: self.node_name.clone(),
            host: String::new(),
            port: 0,
            status: NodeStatus::Online,
            resources: ResourceInfo::default(),
            version: "1.0.0".into(),
            connected_since: self.connected_since,
            last_heartbeat: self.last_seen,
            latency_ms: 0,
        }
    }
}
