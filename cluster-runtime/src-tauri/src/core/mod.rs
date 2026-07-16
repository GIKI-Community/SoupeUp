use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::jobs::JobStatus;
use crate::nodes::NodeStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub total_nodes: u32,
    pub online_nodes: u32,
    pub active_jobs: u32,
    pub installed_plugins: u32,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub version: String,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub category: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub api: ServiceStatus,
    pub storage: ServiceStatus,
    pub networking: ServiceStatus,
    pub plugin_manager: ServiceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Down,
}

pub fn mock_system_info() -> SystemInfo {
    SystemInfo {
        total_nodes: 8,
        online_nodes: 5,
        active_jobs: 3,
        installed_plugins: 5,
        cpu_usage_percent: 42.7,
        memory_usage_percent: 58.3,
        version: "0.1.0".into(),
        uptime_secs: 86400,
    }
}

pub fn mock_activity() -> Vec<ActivityEntry> {
    let now = Utc::now();
    vec![
        ActivityEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(1),
            category: "node".into(),
            message: "alpha-workstation came online".into(),
        },
        ActivityEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(3),
            category: "job".into(),
            message: "Job job-j0k1l2 started by dave@cluster.local".into(),
        },
        ActivityEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(8),
            category: "plugin".into(),
            message: "Ray Adapter plugin enabled".into(),
        },
        ActivityEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(15),
            category: "node".into(),
            message: "zeta-compute went offline".into(),
        },
        ActivityEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(22),
            category: "job".into(),
            message: "Job job-d4e5f6 completed successfully".into(),
        },
        ActivityEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(35),
            category: "system".into(),
            message: "Configuration reloaded".into(),
        },
    ]
}

pub fn mock_system_status() -> SystemStatus {
    SystemStatus {
        api: ServiceStatus::Healthy,
        storage: ServiceStatus::Healthy,
        networking: ServiceStatus::Degraded,
        plugin_manager: ServiceStatus::Healthy,
    }
}

#[allow(dead_code)]
pub fn count_by_status<T, F>(items: &[T], predicate: F) -> u32
where
    F: Fn(&T) -> bool,
{
    items.iter().filter(|item| predicate(item)).count() as u32
}

#[allow(dead_code)]
pub fn node_is_online(status: &NodeStatus) -> bool {
    matches!(status, NodeStatus::Online | NodeStatus::Degraded)
}

#[allow(dead_code)]
pub fn job_is_active(status: &JobStatus) -> bool {
    matches!(status, JobStatus::Running | JobStatus::Pending)
}
