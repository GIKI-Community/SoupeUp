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
    vec![
        Node {
            id: "node-001".into(),
            name: "alpha-workstation".into(),
            platform: NodePlatform::Windows,
            status: NodeStatus::Online,
            cpu_percent: 34.2,
            memory_percent: 62.8,
            backend: "native".into(),
            version: "0.1.0".into(),
            last_seen: Utc::now(),
        },
        Node {
            id: "node-002".into(),
            name: "beta-server".into(),
            platform: NodePlatform::Linux,
            status: NodeStatus::Online,
            cpu_percent: 71.5,
            memory_percent: 48.3,
            backend: "ray".into(),
            version: "0.1.0".into(),
            last_seen: Utc::now(),
        },
        Node {
            id: "node-003".into(),
            name: "gamma-mac".into(),
            platform: NodePlatform::MacOS,
            status: NodeStatus::Degraded,
            cpu_percent: 89.1,
            memory_percent: 91.2,
            backend: "native".into(),
            version: "0.1.0".into(),
            last_seen: Utc::now(),
        },
        Node {
            id: "node-004".into(),
            name: "delta-edge".into(),
            platform: NodePlatform::Android,
            status: NodeStatus::Online,
            cpu_percent: 22.0,
            memory_percent: 55.6,
            backend: "htcondor".into(),
            version: "0.0.9".into(),
            last_seen: Utc::now(),
        },
        Node {
            id: "node-005".into(),
            name: "epsilon-pi".into(),
            platform: NodePlatform::RaspberryPi,
            status: NodeStatus::Online,
            cpu_percent: 45.7,
            memory_percent: 73.4,
            backend: "native".into(),
            version: "0.1.0".into(),
            last_seen: Utc::now(),
        },
        Node {
            id: "node-006".into(),
            name: "zeta-compute".into(),
            platform: NodePlatform::Linux,
            status: NodeStatus::Offline,
            cpu_percent: 0.0,
            memory_percent: 0.0,
            backend: "ray".into(),
            version: "0.1.0".into(),
            last_seen: Utc::now() - chrono::Duration::hours(2),
        },
        Node {
            id: "node-007".into(),
            name: "eta-cluster".into(),
            platform: NodePlatform::Linux,
            status: NodeStatus::Maintenance,
            cpu_percent: 5.0,
            memory_percent: 12.0,
            backend: "native".into(),
            version: "0.1.0".into(),
            last_seen: Utc::now(),
        },
        Node {
            id: "node-008".into(),
            name: "theta-laptop".into(),
            platform: NodePlatform::Windows,
            status: NodeStatus::Online,
            cpu_percent: 18.3,
            memory_percent: 41.9,
            backend: "htcondor".into(),
            version: "0.0.9".into(),
            last_seen: Utc::now(),
        },
    ]
}
