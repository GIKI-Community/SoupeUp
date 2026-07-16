use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub plugin_type: String,
    pub api_version: String,
    pub entry: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}
