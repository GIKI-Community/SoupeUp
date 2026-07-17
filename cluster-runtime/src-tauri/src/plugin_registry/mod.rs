use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::plugin_api::PluginApi;
use crate::plugin_loader::PluginLoader;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    Discovered,
    Validated,
    Loaded,
    Initializing,
    Running,
    Error,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: PluginStatus,
    pub author: String,
    pub description: String,
    /// Optional list of capability tags shown in the UI.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Plugin type, e.g. "Runtime", "Scheduler", "Exporter"
    #[serde(default)]
    pub plugin_type: String,
}

#[allow(dead_code)]
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn PluginApi>>,
    info: HashMap<String, PluginInfo>,
    loader: PluginLoader,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            info: HashMap::new(),
            loader: PluginLoader::new(),
        }
    }

    // ─── Query ────────────────────────────────────────────────────────────────

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let mut list: Vec<PluginInfo> = self.info.values().cloned().collect();
        // Stable ordering: Python Runtime first, then alphabetical
        list.sort_by(|a, b| {
            if a.id == "plugin-python-runtime" {
                std::cmp::Ordering::Less
            } else if b.id == "plugin-python-runtime" {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });
        list
    }

    pub fn get_plugin_info(&self, id: &str) -> Option<&PluginInfo> {
        self.info.get(id)
    }

    // ─── Mutation ─────────────────────────────────────────────────────────────

    /// Update the status of a registered plugin.
    /// Used by the initialization background task to mark the Python Runtime
    /// as Running (or Error) once the async setup completes.
    pub fn update_plugin_status(&mut self, id: &str, status: PluginStatus) {
        if let Some(info) = self.info.get_mut(id) {
            info.status = status;
        }
    }

    /// Register the Python Runtime Plugin with `Initializing` status.
    /// Called synchronously during app startup before the async init task runs.
    pub fn register_python_runtime(&mut self) {
        let id = "plugin-python-runtime".to_string();
        self.info.insert(
            id.clone(),
            PluginInfo {
                id,
                name: "Python Runtime".to_string(),
                version: "0.1.0".to_string(),
                status: PluginStatus::Initializing,
                author: "Cluster Runtime Team".to_string(),
                description: "Embedded Python 3.13 runtime with virtual environment, \
                               package management, and code execution."
                    .to_string(),
                capabilities: vec![
                    "Python Execution".to_string(),
                    "Package Management".to_string(),
                    "Virtual Environment Management".to_string(),
                    "Script Execution".to_string(),
                ],
                plugin_type: "Runtime".to_string(),
            },
        );
    }

    /// Register the Dask Scheduler Plugin (starts as Initializing until packages are ready).
    pub fn register_dask_scheduler(&mut self) {
        let id = "plugin-dask-scheduler".to_string();
        self.info.insert(
            id.clone(),
            PluginInfo {
                id,
                name: "Dask Scheduler".to_string(),
                version: "0.1.0".to_string(),
                status: PluginStatus::Initializing,
                author: "Cluster Runtime Team".to_string(),
                description: "Distributed scheduling via Dask. Uses the Python Runtime \
                               Plugin for all execution and package management."
                    .to_string(),
                capabilities: vec![
                    "Distributed Scheduling".to_string(),
                    "Distributed Workers".to_string(),
                    "Task Submission".to_string(),
                    "Cluster Monitoring".to_string(),
                    "Dashboard Integration".to_string(),
                ],
                plugin_type: "Scheduler".to_string(),
            },
        );
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
