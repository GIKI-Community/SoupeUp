use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub uuid: String,
    pub hostname: String,
    pub os: String,
    pub architecture: String,
    pub cpu_model: String,
    pub logical_cores: usize,
    pub physical_cores: usize,
    pub ram: u64,
    pub available_ram: u64,
    pub gpu_list: Vec<String>,
    pub disk_space: u64,
    pub runtime_version: String,
    pub online_status: bool,
    pub worker_count: usize,
    pub uptime: u64,
}
