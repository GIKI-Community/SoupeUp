mod loader;
mod manager;
mod mock;
mod registry;
mod trait_def;

pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use mock::MockExecutionPlugin;
pub use registry::PluginRegistry;
pub use trait_def::Plugin;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    Enabled,
    Disabled,
    Error,
    Updating,
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

pub fn mock_plugins() -> Vec<PluginInfo> {
    vec![
        PluginInfo {
            id: "plugin-native".into(),
            name: "Native Runtime".into(),
            version: "0.1.0".into(),
            status: PluginStatus::Enabled,
            author: "Cluster Runtime Team".into(),
            description: "Built-in execution engine for local compute workloads.".into(),
        },
        PluginInfo {
            id: "plugin-ray".into(),
            name: "Ray Adapter".into(),
            version: "0.1.0".into(),
            status: PluginStatus::Enabled,
            author: "Cluster Runtime Team".into(),
            description: "Ray distributed computing framework integration.".into(),
        },
        PluginInfo {
            id: "plugin-htcondor".into(),
            name: "HTCondor Adapter".into(),
            version: "0.0.9".into(),
            status: PluginStatus::Disabled,
            author: "Community".into(),
            description: "HTCondor batch system scheduler integration.".into(),
        },
        PluginInfo {
            id: "plugin-metrics".into(),
            name: "Metrics Exporter".into(),
            version: "0.2.1".into(),
            status: PluginStatus::Enabled,
            author: "Cluster Runtime Team".into(),
            description: "Prometheus-compatible metrics export plugin.".into(),
        },
        PluginInfo {
            id: "plugin-auth".into(),
            name: "OAuth Provider".into(),
            version: "0.1.0".into(),
            status: PluginStatus::Disabled,
            author: "Community".into(),
            description: "OAuth 2.0 authentication provider for remote clusters.".into(),
        },
    ]
}
