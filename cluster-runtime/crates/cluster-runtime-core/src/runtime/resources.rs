use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceSnapshot {
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub disk_usage: f32,
    pub gpu_usage: Option<f32>,
    pub network_bandwidth: f32,
}

// In a full implementation, this uses sysinfo to gather metrics.
// We scaffold it here.
pub fn get_current_resources() -> ResourceSnapshot {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    
    let cpu_usage = sys.global_cpu_usage();
    let ram_usage = sys.used_memory() as f32 / sys.total_memory() as f32;

    ResourceSnapshot {
        cpu_usage,
        ram_usage,
        disk_usage: 0.0,
        gpu_usage: None,
        network_bandwidth: 0.0,
    }
}
