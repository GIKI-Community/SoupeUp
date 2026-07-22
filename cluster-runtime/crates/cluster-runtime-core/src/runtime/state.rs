use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeProviderLimits {
    pub enabled: bool,
    pub node_name: String,
    pub max_cpu_percent: u8,
    pub max_ram_percent: u8,
    pub max_gpu_percent: u8,
    pub max_workers: usize,
    pub allow_gpu: bool,
    pub allow_cpu: bool,
    pub allow_storage: bool,
    pub allowed_plugins: Vec<String>,
    pub authentication_mode: String,
}

impl Default for ComputeProviderLimits {
    fn default() -> Self {
        Self {
            enabled: false,
            node_name: "LocalNode".to_string(),
            max_cpu_percent: 80,
            max_ram_percent: 80,
            max_gpu_percent: 100,
            max_workers: 4,
            allow_gpu: true,
            allow_cpu: true,
            allow_storage: true,
            allowed_plugins: vec!["native".into()],
            authentication_mode: "none".into(),
        }
    }
}
