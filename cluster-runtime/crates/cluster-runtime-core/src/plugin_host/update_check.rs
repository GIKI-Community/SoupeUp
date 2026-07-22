//! Compatibility-aware plugin update check (notify only).

use serde::{Deserialize, Serialize};

use crate::plugin_loader::discover::bundled_catalog;
use crate::plugin_loader::manifest::{is_newer_version, PluginManifest};
use crate::updates::UpdateCheckResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UpdateRecommendation {
    None,
    PluginUpdate,
    AppUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUpdateCheck {
    pub plugin_id: String,
    pub installed_version: String,
    pub available_version: Option<String>,
    pub update_available: bool,
    pub recommendation: UpdateRecommendation,
    pub message: String,
    pub release_url: Option<String>,
    pub app_compat: String,
    pub app_update: Option<UpdateCheckResult>,
}

pub async fn check_plugin_update(
    installed: &PluginManifest,
) -> Result<PluginUpdateCheck, String> {
    let app_version = env!("CARGO_PKG_VERSION");
    let catalog = bundled_catalog();
    let candidate = catalog.get(&installed.id);

    let Some(candidate) = candidate else {
        return Ok(PluginUpdateCheck {
            plugin_id: installed.id.clone(),
            installed_version: installed.version.clone(),
            available_version: None,
            update_available: false,
            recommendation: UpdateRecommendation::None,
            message: "No update catalog entry for this plugin.".into(),
            release_url: None,
            app_compat: installed.app_compat.clone(),
            app_update: None,
        });
    };

    if !is_newer_version(&candidate.version, &installed.version) {
        return Ok(PluginUpdateCheck {
            plugin_id: installed.id.clone(),
            installed_version: installed.version.clone(),
            available_version: Some(candidate.version.clone()),
            update_available: false,
            recommendation: UpdateRecommendation::None,
            message: format!(
                "Plugin '{}' is up to date (v{}).",
                installed.name, installed.version
            ),
            release_url: None,
            app_compat: candidate.app_compat.clone(),
            app_update: None,
        });
    }

    // Newer candidate exists.
    if candidate.is_compatible_with_app(app_version) {
        return Ok(PluginUpdateCheck {
            plugin_id: installed.id.clone(),
            installed_version: installed.version.clone(),
            available_version: Some(candidate.version.clone()),
            update_available: true,
            recommendation: UpdateRecommendation::PluginUpdate,
            message: format!(
                "Plugin update available: v{} → v{} (compatible with app v{app_version}). Apply is not automated yet.",
                installed.version, candidate.version
            ),
            release_url: None,
            app_compat: candidate.app_compat.clone(),
            app_update: None,
        });
    }

    // Needs newer app.
    let app_update = crate::updates::check_for_updates().await.ok();
    Ok(PluginUpdateCheck {
        plugin_id: installed.id.clone(),
        installed_version: installed.version.clone(),
        available_version: Some(candidate.version.clone()),
        update_available: true,
        recommendation: UpdateRecommendation::AppUpdate,
        message: format!(
            "Plugin '{}' v{} requires app {}, but this app is v{app_version}. Update Cluster Runtime.",
            installed.name, candidate.version, candidate.app_compat
        ),
        release_url: app_update
            .as_ref()
            .and_then(|u| u.release_url.clone()),
        app_compat: candidate.app_compat.clone(),
        app_update,
    })
}
