use serde::{Deserialize, Serialize};

/// Application configuration management.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub theme: String,
    pub accent_color: String,
    pub language: String,
    pub auto_start: bool,
    pub telemetry_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            accent_color: "#6366f1".into(),
            language: "en".into(),
            auto_start: false,
            telemetry_enabled: false,
        }
    }
}

pub struct ConfigService {
    config: AppConfig,
}

impl ConfigService {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn get(&self) -> &AppConfig {
        &self.config
    }
}

impl Default for ConfigService {
    fn default() -> Self {
        Self::new(AppConfig::default())
    }
}
