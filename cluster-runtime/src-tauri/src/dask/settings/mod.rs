use serde::{Deserialize, Serialize};

/// User-configurable Dask settings persisted in memory for this session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaskSettings {
    /// Host the scheduler binds to (use 0.0.0.0 for multi-node).
    pub scheduler_host: String,
    /// Port the scheduler listens on.
    pub scheduler_port: u16,
    /// Dask diagnostics dashboard port.
    pub dashboard_port: u16,
    /// Address workers connect to (e.g. tcp://192.168.1.10:8786).
    pub scheduler_address: String,
    /// Worker thread count (0 = auto).
    pub worker_threads: usize,
    /// Optional memory limit string understood by Dask (e.g. "4GB").
    pub worker_memory_limit: String,
    /// Display name for the local worker.
    pub worker_name: String,
    /// Local directory for spill files.
    pub local_directory: String,
    /// Logging level passed into Dask processes.
    pub logging_level: String,
}

impl Default for DaskSettings {
    fn default() -> Self {
        Self {
            scheduler_host: "0.0.0.0".to_string(),
            scheduler_port: 8786,
            dashboard_port: 8787,
            scheduler_address: "tcp://127.0.0.1:8786".to_string(),
            worker_threads: 0,
            worker_memory_limit: String::new(),
            worker_name: "worker-1".to_string(),
            local_directory: String::new(),
            logging_level: "info".to_string(),
        }
    }
}

impl DaskSettings {
    pub fn dashboard_url(&self) -> String {
        // Dashboard is typically reached via localhost from the scheduler machine.
        format!("http://127.0.0.1:{}", self.dashboard_port)
    }

    pub fn advertised_scheduler_address(&self) -> String {
        if self.scheduler_address.starts_with("tcp://") {
            self.scheduler_address.clone()
        } else {
            format!("tcp://{}:{}", self.scheduler_host, self.scheduler_port)
        }
    }
}
