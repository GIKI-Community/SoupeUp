use crate::core::{mock_activity, mock_system_info, mock_system_status, ActivityEntry, SystemInfo, SystemStatus};
use crate::jobs::{mock_jobs, Job};
use crate::logging::{mock_logs, LogEntry};
use crate::metrics::{mock_metrics, MetricsSnapshot};
use crate::nodes::{mock_nodes, Node};
// use crate::plugins::{mock_plugins, PluginInfo};


#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    mock_system_info()
}

#[tauri::command]
pub fn get_system_status() -> SystemStatus {
    mock_system_status()
}

#[tauri::command]
pub fn get_activity() -> Vec<ActivityEntry> {
    mock_activity()
}

#[tauri::command]
pub fn get_nodes() -> Vec<Node> {
    mock_nodes()
}

#[tauri::command]
pub fn get_jobs() -> Vec<Job> {
    mock_jobs()
}

use crate::plugin_registry::PluginInfo;

#[tauri::command]
pub async fn get_plugins(state: tauri::State<'_, crate::AppState>) -> Result<Vec<PluginInfo>, String> {
    let registry = state.plugin_registry.read().await;
    Ok(registry.list_plugins())
}

#[tauri::command]
pub fn get_metrics() -> MetricsSnapshot {
    mock_metrics()
}

#[tauri::command]
pub fn get_logs() -> Vec<LogEntry> {
    mock_logs()
}
