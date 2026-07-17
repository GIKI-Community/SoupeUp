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
    Vec::new()
}

pub fn nodes_from_dask_snapshot(snap: &crate::dask::ClusterSnapshot) -> Vec<Node> {
    let now = snap.updated_at.unwrap_or_else(Utc::now);
    let mut nodes = Vec::new();

    if snap.scheduler.status == crate::dask::ComponentStatus::Running {
        nodes.push(Node {
            id: snap
                .scheduler
                .process_id
                .clone()
                .unwrap_or_else(|| "local-scheduler".to_string()),
            name: format!("scheduler ({})", snap.scheduler.host),
            platform: local_platform(),
            status: NodeStatus::Online,
            cpu_percent: 0.0,
            memory_percent: 0.0,
            backend: "dask-scheduler".to_string(),
            version: snap
                .scheduler
                .address
                .clone()
                .unwrap_or_else(|| "tcp://127.0.0.1:8786".to_string()),
            last_seen: now,
        });
    }

    for worker in &snap.workers {
        let memory_percent = if worker.memory_limit > 0 {
            (worker.memory_used as f64 / worker.memory_limit as f64) * 100.0
        } else {
            0.0
        };
        nodes.push(Node {
            id: worker.id.clone(),
            name: worker.name.clone(),
            platform: NodePlatform::Other,
            status: map_worker_status(&worker.status),
            cpu_percent: worker.cpu,
            memory_percent,
            backend: "dask-worker".to_string(),
            version: worker.address.clone(),
            last_seen: now,
        });
    }

    // Include the local worker process when it is running but not yet visible on the scheduler.
    if snap.local_worker.status == crate::dask::ComponentStatus::Running
        && !nodes.iter().any(|n| n.name == snap.local_worker.name)
    {
        nodes.push(Node {
            id: snap
                .local_worker
                .process_id
                .clone()
                .unwrap_or_else(|| format!("local-{}", snap.local_worker.name)),
            name: snap.local_worker.name.clone(),
            platform: local_platform(),
            status: NodeStatus::Online,
            cpu_percent: 0.0,
            memory_percent: 0.0,
            backend: "dask-worker".to_string(),
            version: snap.local_worker.scheduler_address.clone(),
            last_seen: now,
        });
    }

    nodes
}

fn local_platform() -> NodePlatform {
    if cfg!(windows) {
        NodePlatform::Windows
    } else if cfg!(target_os = "macos") {
        NodePlatform::MacOS
    } else if cfg!(target_os = "linux") {
        NodePlatform::Linux
    } else {
        NodePlatform::Other
    }
}

fn map_worker_status(status: &str) -> NodeStatus {
    match status.to_ascii_lowercase().as_str() {
        "running" | "ready" | "alive" => NodeStatus::Online,
        "busy" | "paused" => NodeStatus::Degraded,
        "closed" | "stopped" | "offline" => NodeStatus::Offline,
        _ => NodeStatus::Online,
    }
}
