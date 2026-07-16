use serde::{Deserialize, Serialize};

/// Network configuration and cluster connectivity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    pub listen_address: String,
    pub port: u16,
    pub enable_mdns: bool,
    pub enable_remote: bool,
}

pub struct NetworkService {
    config: NetworkConfig,
}

impl NetworkService {
    pub fn new(config: NetworkConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    pub fn is_listening(&self) -> bool {
        true
    }
}

impl Default for NetworkService {
    fn default() -> Self {
        Self::new(NetworkConfig {
            listen_address: "127.0.0.1".into(),
            port: 9470,
            enable_mdns: true,
            enable_remote: false,
        })
    }
}
