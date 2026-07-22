//! Persist which plugins are enabled under `{data_dir}/plugins/enabled.json`.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::discover::plugins_dir;
use super::manifest::PluginManifest;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnabledConfig {
    /// plugin id → enabled
    #[serde(default)]
    pub enabled: HashMap<String, bool>,
}

fn path(data_dir: &Path) -> std::path::PathBuf {
    plugins_dir(data_dir).join("enabled.json")
}

pub fn load(data_dir: &Path) -> EnabledConfig {
    let p = path(data_dir);
    match std::fs::read_to_string(&p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => EnabledConfig::default(),
    }
}

pub fn save(data_dir: &Path, cfg: &EnabledConfig) -> Result<(), String> {
    let p = path(data_dir);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(p, json).map_err(|e| e.to_string())
}

/// Mandatory plugins are always enabled. Missing keys default to enabled for shipped plugins.
pub fn is_enabled(cfg: &EnabledConfig, manifest: &PluginManifest) -> bool {
    if manifest.mandatory {
        return true;
    }
    cfg.enabled.get(&manifest.id).copied().unwrap_or(true)
}

pub fn set_enabled(
    data_dir: &Path,
    id: &str,
    enabled: bool,
    manifest: &PluginManifest,
) -> Result<EnabledConfig, String> {
    if manifest.mandatory && !enabled {
        return Err(format!(
            "Plugin '{}' is required and cannot be disabled",
            manifest.name
        ));
    }
    let mut cfg = load(data_dir);
    cfg.enabled.insert(id.to_string(), enabled);
    save(data_dir, &cfg)?;
    Ok(cfg)
}
