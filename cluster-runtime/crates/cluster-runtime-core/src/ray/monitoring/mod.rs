use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::ray::client::ClientManager;
use crate::ray::head::HeadManager;
use crate::ray::types::{
    ClusterHealth, ClusterSnapshot, ComponentStatus, ConnectedWorker, RayResult,
};
use crate::ray::worker::WorkerManager;

pub struct MonitoringService {
    head: Arc<HeadManager>,
    worker: Arc<WorkerManager>,
    client: Arc<ClientManager>,
    /// Prevents stacked UI polls from spawning many concurrent Python probes.
    snapshot_gate: Mutex<()>,
    /// Last successful probe — returned when a poll overlaps so the UI does not flicker.
    last_snapshot: RwLock<Option<ClusterSnapshot>>,
}

impl MonitoringService {
    pub fn new(
        head: Arc<HeadManager>,
        worker: Arc<WorkerManager>,
        client: Arc<ClientManager>,
    ) -> Self {
        Self {
            head,
            worker,
            client,
            snapshot_gate: Mutex::new(()),
            last_snapshot: RwLock::new(None),
        }
    }

    pub async fn snapshot(&self) -> RayResult<ClusterSnapshot> {
        let Ok(_guard) = self.snapshot_gate.try_lock() else {
            return Ok(self.cached_or_local().await);
        };

        let head = self.head.status().await;
        let local_worker = self.worker.status().await;

        let mut workers: Vec<ConnectedWorker> = Vec::new();
        let mut total_cores = 0usize;
        let mut total_memory = 0u64;
        let mut client_connected = false;

        let connect_addr = head.address.clone().or_else(|| {
            if local_worker.status == ComponentStatus::Running {
                Some(local_worker.head_address.clone())
            } else {
                None
            }
        });

        if let Some(addr) = connect_addr {
            self.client.set_address(addr).await;
            match self.client.cluster_info().await {
                Ok(info) => {
                    client_connected = true;
                    if let Some(arr) = info.get("workers").and_then(|v| v.as_array()) {
                        for w in arr {
                            let cw = ConnectedWorker {
                                id: w
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                name: w
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                address: w
                                    .get("address")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                nthreads: w
                                    .get("nthreads")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0) as usize,
                                memory_limit: w
                                    .get("memoryLimit")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0),
                                memory_used: w
                                    .get("memoryUsed")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0),
                                cpu: w.get("cpu").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                status: w
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                            };
                            total_cores += cw.nthreads;
                            total_memory += cw.memory_limit;
                            workers.push(cw);
                        }
                    }
                }
                Err(e) => {
                    log::debug!("Ray cluster snapshot probe skipped: {}", e);
                    if let Some(cached) = self.last_snapshot.read().await.clone() {
                        let mut snap = cached;
                        snap.head = head;
                        snap.local_worker = local_worker;
                        snap.updated_at = Some(Utc::now());
                        return Ok(snap);
                    }
                }
            }
        }

        let health = compute_health(&head.status, workers.len(), &local_worker.status);

        let snap = ClusterSnapshot {
            health,
            head,
            local_worker,
            workers,
            total_cores,
            total_memory,
            active_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            bandwidth_bytes_per_sec: 0.0,
            client_connected,
            updated_at: Some(Utc::now()),
        };
        *self.last_snapshot.write().await = Some(snap.clone());
        Ok(snap)
    }

    async fn cached_or_local(&self) -> ClusterSnapshot {
        let head = self.head.status().await;
        let local_worker = self.worker.status().await;
        if let Some(mut snap) = self.last_snapshot.read().await.clone() {
            snap.head = head;
            snap.local_worker = local_worker;
            snap.updated_at = Some(Utc::now());
            return snap;
        }
        ClusterSnapshot {
            health: ClusterHealth::Unknown,
            head,
            local_worker,
            workers: Vec::new(),
            total_cores: 0,
            total_memory: 0,
            active_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            bandwidth_bytes_per_sec: 0.0,
            client_connected: false,
            updated_at: Some(Utc::now()),
        }
    }
}

fn compute_health(
    head: &ComponentStatus,
    worker_count: usize,
    local_worker: &ComponentStatus,
) -> ClusterHealth {
    match head {
        ComponentStatus::Running if worker_count > 0 => ClusterHealth::Healthy,
        ComponentStatus::Running if *local_worker == ComponentStatus::Running => {
            ClusterHealth::Degraded
        }
        ComponentStatus::Running => ClusterHealth::Degraded,
        ComponentStatus::Error => ClusterHealth::Unhealthy,
        ComponentStatus::Starting | ComponentStatus::Stopping => ClusterHealth::Degraded,
        _ => {
            if *local_worker == ComponentStatus::Running {
                if worker_count > 0 {
                    ClusterHealth::Healthy
                } else {
                    ClusterHealth::Degraded
                }
            } else {
                ClusterHealth::Unknown
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RayMetrics {
    pub head_cpu: f64,
    pub head_memory: f64,
    pub worker_cpu: f64,
    pub worker_memory: f64,
    pub tasks_per_sec: f64,
    pub data_transfer: f64,
    pub worker_load: f64,
    pub worker_count: usize,
}

impl MonitoringService {
    pub async fn metrics(&self) -> RayResult<RayMetrics> {
        let snap = self.snapshot().await?;
        let worker_cpu = if snap.workers.is_empty() {
            0.0
        } else {
            snap.workers.iter().map(|w| w.cpu).sum::<f64>() / snap.workers.len() as f64
        };
        let worker_memory = if snap.total_memory == 0 {
            0.0
        } else {
            let used: u64 = snap.workers.iter().map(|w| w.memory_used).sum();
            (used as f64 / snap.total_memory as f64) * 100.0
        };

        Ok(RayMetrics {
            head_cpu: 0.0,
            head_memory: 0.0,
            worker_cpu,
            worker_memory,
            tasks_per_sec: 0.0,
            data_transfer: snap.bandwidth_bytes_per_sec,
            worker_load: worker_cpu,
            worker_count: snap.workers.len(),
        })
    }
}
