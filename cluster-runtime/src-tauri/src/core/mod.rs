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
    // TODO: Get real system info from sysinfo crate
    SystemInfo {
        total_nodes: 0,
        online_nodes: 0,
        active_jobs: 0,
        installed_plugins: 0,
        cpu_usage_percent: 0.0,
        memory_usage_percent: 0.0,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
    }
}

pub fn mock_activity() -> Vec<ActivityEntry> {
    // TODO: Get real activity from event bus
    Vec::new()
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
