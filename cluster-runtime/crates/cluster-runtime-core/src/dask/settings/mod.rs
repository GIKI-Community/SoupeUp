use serde::{Deserialize, Serialize};

/// User-configurable Dask settings persisted in memory for this session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaskSettings {
    /// Host the scheduler binds to (use 0.0.0.0 for multi-node LAN; 127.0.0.1 when using iroh tunnels).
    pub scheduler_host: String,
    /// Port the scheduler listens on.
    pub scheduler_port: u16,
    /// Dask diagnostics dashboard port.
    pub dashboard_port: u16,
    /// Address workers connect to (e.g. tcp://192.168.1.10:8786).
    pub scheduler_address: String,
    /// When set, workers open an iroh TCP tunnel to this EndpointId instead of dialing LAN TCP.
    #[serde(default)]
    pub scheduler_endpoint_id: String,
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
            scheduler_endpoint_id: String::new(),
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

    /// True if `raw` looks like an iroh EndpointId (not a host/tcp address).
    pub fn looks_like_endpoint_id(raw: &str) -> bool {
        let s = raw.trim();
        if s.is_empty() || s.contains(':') || s.contains('/') || s.contains('.') {
            return false;
        }
        // iroh EndpointIds are z-base32 public keys (~52 chars of alphanumeric).
        s.len() >= 40
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric())
    }

    /// Normalize user input to `tcp://host:port`, filling CR defaults when omitted.
    /// Accepts `10.0.0.1`, `10.0.0.1:8786`, `tcp://10.0.0.1`, `tcp://10.0.0.1:8786`.
    pub fn normalize_scheduler_address(raw: &str) -> String {
        let s = raw.trim();
        if s.is_empty() {
            return format!("tcp://127.0.0.1:{}", Self::default().scheduler_port);
        }
        let rest = s
            .strip_prefix("tcp://")
            .or_else(|| s.strip_prefix("TCP://"))
            .unwrap_or(s);
        if rest.contains(':') {
            format!("tcp://{rest}")
        } else {
            format!("tcp://{rest}:{}", Self::default().scheduler_port)
        }
    }
}
