use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::plugin_api::PluginApi;
use crate::plugin_loader::manifest::PluginManifest;
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
    Incompatible,
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
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub plugin_type: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub mandatory: bool,
    #[serde(default)]
    pub app_compat: String,
    #[serde(default)]
    pub is_default: bool,
}

impl PluginInfo {
    pub fn from_manifest(m: &PluginManifest, status: PluginStatus, enabled: bool) -> Self {
        Self {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            status,
            author: m.author.clone(),
            description: m.description.clone(),
            capabilities: m.capabilities.clone(),
            plugin_type: m.display_plugin_type(),
            enabled,
            mandatory: m.mandatory,
            app_compat: m.app_compat.clone(),
            is_default: m.default,
        }
    }
}

#[allow(dead_code)]
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn PluginApi>>,
    info: HashMap<String, PluginInfo>,
    manifests: HashMap<String, PluginManifest>,
    loader: PluginLoader,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            info: HashMap::new(),
            manifests: HashMap::new(),
            loader: PluginLoader::new(),
        }
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let mut list: Vec<PluginInfo> = self.info.values().cloned().collect();
        list.sort_by(|a, b| {
            if a.id == "plugin-python-runtime" {
                std::cmp::Ordering::Less
            } else if b.id == "plugin-python-runtime" {
                std::cmp::Ordering::Greater
            } else if a.mandatory != b.mandatory {
                b.mandatory.cmp(&a.mandatory)
            } else {
                a.name.cmp(&b.name)
            }
        });
        list
    }

    pub fn get_plugin_info(&self, id: &str) -> Option<&PluginInfo> {
        self.info.get(id)
    }

    pub fn get_manifest(&self, id: &str) -> Option<&PluginManifest> {
        self.manifests.get(id)
    }

    pub fn upsert(&mut self, info: PluginInfo, manifest: PluginManifest) {
        self.manifests.insert(info.id.clone(), manifest);
        self.info.insert(info.id.clone(), info);
    }

    pub fn update_plugin_status(&mut self, id: &str, status: PluginStatus) {
        if let Some(info) = self.info.get_mut(id) {
            info.status = status;
        }
    }

    pub fn set_enabled_flag(&mut self, id: &str, enabled: bool) {
        if let Some(info) = self.info.get_mut(id) {
            info.enabled = enabled;
            if !enabled && info.status != PluginStatus::Incompatible {
                info.status = PluginStatus::Disabled;
            }
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.info.remove(id);
        self.manifests.remove(id);
    }

    pub fn default_scheduler_id(&self) -> Option<String> {
        self.manifests
            .values()
            .find(|m| m.default && m.is_scheduler())
            .map(|m| m.id.clone())
            .or_else(|| {
                self.manifests
                    .values()
                    .find(|m| m.is_scheduler() && m.mandatory)
                    .map(|m| m.id.clone())
            })
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
