use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::dask::client::ClientManager;
use crate::dask::scheduler::SchedulerManager;
use crate::dask::types::{
    ClusterHealth, ClusterSnapshot, ComponentStatus, ConnectedWorker, DaskResult,
};
use crate::dask::worker::WorkerManager;

pub struct MonitoringService {
    scheduler: Arc<SchedulerManager>,
    worker: Arc<WorkerManager>,
    client: Arc<ClientManager>,
    /// Prevents stacked UI polls from spawning many concurrent Python probes.
    snapshot_gate: Mutex<()>,
    /// Last successful probe — returned when a poll overlaps so the UI does not flicker.
    last_snapshot: RwLock<Option<ClusterSnapshot>>,
}

impl MonitoringService {
    pub fn new(
        scheduler: Arc<SchedulerManager>,
        worker: Arc<WorkerManager>,
        client: Arc<ClientManager>,
    ) -> Self {
        Self {
            scheduler,
            worker,
            client,
            snapshot_gate: Mutex::new(()),
            last_snapshot: RwLock::new(None),
        }
    }

    pub async fn snapshot(&self) -> DaskResult<ClusterSnapshot> {
        // Overlapping poll: serve the last good snapshot (with fresh local status).
        let Ok(_guard) = self.snapshot_gate.try_lock() else {
            return Ok(self.cached_or_local().await);
        };

        let scheduler = self.scheduler.status().await;
        let local_worker = self.worker.status().await;

        let mut workers: Vec<ConnectedWorker> = Vec::new();
        let mut total_cores = 0usize;
        let mut total_memory = 0u64;
        let mut active_tasks = 0u64;
        let mut client_connected = false;

        let connect_addr = scheduler.address.clone().or_else(|| {
            if local_worker.status == ComponentStatus::Running {
                Some(local_worker.scheduler_address.clone())
            } else {
                None
            }
        });

        if let Some(addr) = connect_addr {
            // One Python probe only (cluster_info). Do not also call connect().
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
                    active_tasks = info
                        .get("activeTasks")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                }
                Err(e) => {
                    log::debug!("Cluster snapshot probe skipped: {}", e);
                    // Keep showing the previous worker list on transient probe failures.
                    if let Some(cached) = self.last_snapshot.read().await.clone() {
                        let mut snap = cached;
                        snap.scheduler = scheduler;
                        snap.local_worker = local_worker;
                        snap.updated_at = Some(Utc::now());
                        return Ok(snap);
                    }
                }
            }
        }

        let health = compute_health(&scheduler.status, workers.len(), &local_worker.status);

        let snap = ClusterSnapshot {
            health,
            scheduler,
            local_worker,
            workers,
            total_cores,
            total_memory,
            active_tasks,
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
        let scheduler = self.scheduler.status().await;
        let local_worker = self.worker.status().await;
        if let Some(mut snap) = self.last_snapshot.read().await.clone() {
            snap.scheduler = scheduler;
            snap.local_worker = local_worker;
            snap.updated_at = Some(Utc::now());
            return snap;
        }
        ClusterSnapshot {
            health: ClusterHealth::Unknown,
            scheduler,
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
    scheduler: &ComponentStatus,
    worker_count: usize,
    local_worker: &ComponentStatus,
) -> ClusterHealth {
    match scheduler {
        ComponentStatus::Running if worker_count > 0 => ClusterHealth::Healthy,
        ComponentStatus::Running if *local_worker == ComponentStatus::Running => {
            ClusterHealth::Degraded
        }
        ComponentStatus::Running => ClusterHealth::Degraded,
        ComponentStatus::Error => ClusterHealth::Unhealthy,
        ComponentStatus::Starting | ComponentStatus::Stopping => ClusterHealth::Degraded,
        _ => {
            if *local_worker == ComponentStatus::Running {
                // Worker-only node joined a remote scheduler.
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

/// Metrics slice for the existing Metrics page.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaskMetrics {
    pub scheduler_cpu: f64,
    pub scheduler_memory: f64,
    pub worker_cpu: f64,
    pub worker_memory: f64,
    pub tasks_per_sec: f64,
    pub data_transfer: f64,
    pub worker_load: f64,
    pub worker_count: usize,
}

impl MonitoringService {
    pub async fn metrics(&self) -> DaskResult<DaskMetrics> {
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

        Ok(DaskMetrics {
            scheduler_cpu: 0.0,
            scheduler_memory: 0.0,
            worker_cpu,
            worker_memory,
            tasks_per_sec: 0.0,
            data_transfer: snap.bandwidth_bytes_per_sec,
            worker_load: worker_cpu,
            worker_count: snap.workers.len(),
        })
    }
}
