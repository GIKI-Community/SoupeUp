use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Online,
    Offline,
    Degraded,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodePlatform {
    Windows,
    Linux,
    MacOS,
    Android,
    RaspberryPi,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub name: String,
    pub platform: NodePlatform,
    pub status: NodeStatus,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub backend: String,
    pub version: String,
    pub last_seen: DateTime<Utc>,
}

pub fn mock_nodes() -> Vec<Node> {
    // TODO: Get real nodes from cluster manager
    Vec::new()
}
