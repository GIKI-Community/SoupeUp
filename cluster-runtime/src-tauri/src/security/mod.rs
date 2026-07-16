use serde::{Deserialize, Serialize};

/// Security module placeholder for authentication and authorization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfig {
    pub auth_enabled: bool,
    pub tls_enabled: bool,
    pub certificate_path: Option<String>,
}

pub struct SecurityService {
    config: SecurityConfig,
}

impl SecurityService {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    pub fn is_authenticated(&self) -> bool {
        !self.config.auth_enabled
    }
}

impl Default for SecurityService {
    fn default() -> Self {
        Self::new(SecurityConfig {
            auth_enabled: false,
            tls_enabled: false,
            certificate_path: None,
        })
    }
}
