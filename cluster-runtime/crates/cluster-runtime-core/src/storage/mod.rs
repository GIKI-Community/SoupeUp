use serde::{Deserialize, Serialize};

/// Persistent storage abstraction. Future: SQLite, RocksDB, or cloud sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    pub data_dir: String,
    pub backend: StorageBackend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    Sqlite,
    Memory,
}

pub struct StorageService {
    config: StorageConfig,
}

impl StorageService {
    pub fn new(config: StorageConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    pub fn is_ready(&self) -> bool {
        true
    }
}

impl Default for StorageService {
    fn default() -> Self {
        Self::new(StorageConfig {
            data_dir: "./data".into(),
            backend: StorageBackend::Memory,
        })
    }
}
