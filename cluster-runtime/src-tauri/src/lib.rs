#![allow(dead_code)]
mod commands;
mod config;
mod core;
mod events;
mod jobs;
mod logging;
mod metrics;
mod network;
mod nodes;
mod security;
mod storage;

pub mod plugin_api;
pub mod plugin_host;
pub mod plugin_loader;
pub mod plugin_registry;
pub mod plugin_security;
pub mod plugin_store;
pub mod runtime;

use std::sync::Arc;
use events::EventBus;
use plugin_registry::PluginRegistry;

pub struct AppState {
    pub plugin_registry: Arc<tokio::sync::RwLock<PluginRegistry>>,
    pub event_bus: Arc<EventBus>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            plugin_registry: Arc::new(tokio::sync::RwLock::new(PluginRegistry::new())),
            event_bus: Arc::new(EventBus::default()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_system_info,
            commands::get_system_status,
            commands::get_activity,
            commands::get_nodes,
            commands::get_jobs,
            commands::get_plugins,
            commands::get_metrics,
            commands::get_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
