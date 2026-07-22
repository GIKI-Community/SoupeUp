use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaySettings {
    /// Host the Ray head binds to (use 0.0.0.0 for multi-node).
    pub head_host: String,
    /// GCS port workers connect to.
    pub gcs_port: u16,
    /// Ray dashboard port.
    pub dashboard_port: u16,
    /// Address workers connect to (e.g. 192.168.1.10:6379).
    pub head_address: String,
    /// CPU count for local head/worker (0 = auto).
    pub worker_cpus: usize,
    /// Optional object store memory (e.g. "2GB").
    pub object_store_memory: String,
    /// Display name for the local worker.
    pub worker_name: String,
    /// Logging level passed into Ray processes.
    pub logging_level: String,
}

impl Default for RaySettings {
    fn default() -> Self {
        Self {
            head_host: "0.0.0.0".to_string(),
            gcs_port: 6379,
            dashboard_port: 8265,
            head_address: "127.0.0.1:6379".to_string(),
            worker_cpus: 0,
            object_store_memory: String::new(),
            worker_name: "ray-worker-1".to_string(),
            logging_level: "info".to_string(),
        }
    }
}

impl RaySettings {
    pub fn dashboard_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.dashboard_port)
    }
}
