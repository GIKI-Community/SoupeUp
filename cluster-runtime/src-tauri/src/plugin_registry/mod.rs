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
}

#[allow(dead_code)]
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn PluginApi>>,
    info: HashMap<String, PluginInfo>,
    loader: PluginLoader,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            plugins: HashMap::new(),
            info: HashMap::new(),
            loader: PluginLoader::new(),
        };
        registry.add_mock_plugins();
        registry
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.info.values().cloned().collect()
    }
    
    // Add dummy plugins for UI while backend dynamic loading is mocked in 0.50 effort mode
    pub fn add_mock_plugins(&mut self) {
        let id = "example-plugin".to_string();
        self.info.insert(id.clone(), PluginInfo {
            id,
            name: "Example Plugin".to_string(),
            version: "1.0.0".to_string(),
            status: PluginStatus::Running,
            author: "Cluster Runtime".to_string(),
            description: "An example dynamically loaded plugin.".to_string(),
        });
    }
}
