use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default = "default_author")]
    pub author: String,
    #[serde(default)]
    pub description: String,
    /// `runtime` or `scheduler` (also accepts legacy Title Case).
    #[serde(default = "default_plugin_type")]
    pub plugin_type: String,
    #[serde(default = "default_api_version")]
    pub api_version: String,
    /// Host factory key; defaults to `id`.
    #[serde(default)]
    pub factory: Option<String>,
    #[serde(default)]
    pub entry: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub mandatory: bool,
    /// Semver range of host app versions this plugin supports, e.g. `>=0.1.0,<0.2.0`.
    #[serde(default = "default_app_compat")]
    pub app_compat: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_author() -> String {
    "Cluster Runtime Team".into()
}
fn default_plugin_type() -> String {
    "scheduler".into()
}
fn default_api_version() -> String {
    "1.0".into()
}
fn default_app_compat() -> String {
    ">=0.1.0".into()
}

impl PluginManifest {
    pub fn factory_id(&self) -> &str {
        self.factory.as_deref().unwrap_or(self.id.as_str())
    }

    pub fn is_scheduler(&self) -> bool {
        self.plugin_type.eq_ignore_ascii_case("scheduler")
    }

    pub fn is_runtime(&self) -> bool {
        self.plugin_type.eq_ignore_ascii_case("runtime")
    }

    pub fn display_plugin_type(&self) -> String {
        if self.is_runtime() {
            "Runtime".into()
        } else if self.is_scheduler() {
            "Scheduler".into()
        } else {
            self.plugin_type.clone()
        }
    }

    /// Whether `app_version` satisfies this plugin's `app_compat` range.
    pub fn is_compatible_with_app(&self, app_version: &str) -> bool {
        let Ok(req) = semver::VersionReq::parse(&self.app_compat) else {
            return true;
        };
        let ver = normalize_version(app_version);
        match semver::Version::parse(&ver) {
            Ok(v) => req.matches(&v),
            Err(_) => true,
        }
    }
}

fn normalize_version(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

pub fn is_newer_version(candidate: &str, installed: &str) -> bool {
    let Ok(c) = semver::Version::parse(&normalize_version(candidate)) else {
        return normalize_version(candidate) > normalize_version(installed);
    };
    let Ok(i) = semver::Version::parse(&normalize_version(installed)) else {
        return true;
    };
    c > i
}
