use chrono::Utc;
use std::sync::Arc;

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
        }
    }

    pub async fn snapshot(&self) -> DaskResult<ClusterSnapshot> {
        let scheduler = self.scheduler.status().await;
        let local_worker = self.worker.status().await;

        let mut workers: Vec<ConnectedWorker> = Vec::new();
        let mut total_cores = 0usize;
        let mut total_memory = 0u64;
        let mut active_tasks = 0u64;
        let mut client_connected = false;

        // Prefer live scheduler view when a client can connect.
        let connect_addr = scheduler
            .address
            .clone()
            .or_else(|| {
                if local_worker.status == ComponentStatus::Running {
                    Some(local_worker.scheduler_address.clone())
                } else {
                    None
                }
            });

        if let Some(addr) = connect_addr {
            match self.client.connect(Some(addr)).await {
                Ok(_) => {
                    client_connected = true;
                    if let Ok(info) = self.client.cluster_info().await {
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
                }
                Err(e) => {
                    log::debug!("Cluster snapshot client connect skipped: {}", e);
                }
            }
        }

        let health = compute_health(&scheduler.status, workers.len(), &local_worker.status);

        Ok(ClusterSnapshot {
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
        })
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
