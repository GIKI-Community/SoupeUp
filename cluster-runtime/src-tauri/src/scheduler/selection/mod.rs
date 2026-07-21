use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::jobs::models::{JobSpec, ResourceRequirements, SchedulerCapabilities, SchedulerListEntry};
use crate::scheduler::abstraction::{SchedulerError, SchedulerService};

pub const DASK_PLUGIN_ID: &str = "plugin-dask-scheduler";
pub const RAY_PLUGIN_ID: &str = "plugin-ray";
pub const MPI_PLUGIN_ID: &str = "plugin-mpi";
pub const DEFAULT_SCHEDULER: &str = DASK_PLUGIN_ID;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveSchedulerConfig {
    plugin_id: String,
}

pub struct SchedulerRegistry {
    schedulers: RwLock<HashMap<String, Arc<dyn SchedulerService>>>,
    active: RwLock<String>,
    config_path: PathBuf,
}

impl SchedulerRegistry {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            schedulers: RwLock::new(HashMap::new()),
            active: RwLock::new(DEFAULT_SCHEDULER.to_string()),
            config_path,
        }
    }

    pub async fn register(&self, scheduler: Arc<dyn SchedulerService>) {
        let id = scheduler.plugin_id().to_string();
        self.schedulers.write().await.insert(id, scheduler);
    }

    pub async fn load_active(&self) {
        if let Ok(data) = tokio::fs::read_to_string(&self.config_path).await {
            if let Ok(cfg) = serde_json::from_str::<ActiveSchedulerConfig>(&data) {
                let schedulers = self.schedulers.read().await;
                if schedulers.contains_key(&cfg.plugin_id) {
                    *self.active.write().await = cfg.plugin_id;
                }
            }
        }
    }

    pub async fn set_active(&self, plugin_id: &str) -> Result<(), SchedulerError> {
        let schedulers = self.schedulers.read().await;
        if !schedulers.contains_key(plugin_id) {
            return Err(SchedulerError::NotReady(format!(
                "Scheduler '{}' is not registered",
                plugin_id
            )));
        }
        drop(schedulers);

        *self.active.write().await = plugin_id.to_string();
        let cfg = ActiveSchedulerConfig {
            plugin_id: plugin_id.to_string(),
        };
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(&cfg) {
            tokio::fs::write(&self.config_path, json).await.ok();
        }
        Ok(())
    }

    pub async fn active_id(&self) -> String {
        self.active.read().await.clone()
    }

    pub async fn active(&self) -> Result<Arc<dyn SchedulerService>, SchedulerError> {
        let id = self.active.read().await.clone();
        self.get(&id).await
    }

    pub async fn get(&self, plugin_id: &str) -> Result<Arc<dyn SchedulerService>, SchedulerError> {
        let schedulers = self.schedulers.read().await;
        schedulers
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| SchedulerError::NotReady(format!("Scheduler '{}' not available", plugin_id)))
    }

    pub async fn list(&self) -> Vec<SchedulerListEntry> {
        let schedulers = self.schedulers.read().await;
        let mut entries = Vec::new();
        for scheduler in schedulers.values() {
            let capabilities = scheduler.capabilities().await;
            let available = scheduler.cluster_info().await.is_ok();
            entries.push(SchedulerListEntry {
                plugin_id: scheduler.plugin_id().to_string(),
                display_name: scheduler.display_name().to_string(),
                capabilities,
                available,
            });
        }
        entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        entries
    }
}

pub fn validate_resources(
    resources: &ResourceRequirements,
    capabilities: &SchedulerCapabilities,
) -> Vec<String> {
    let mut warnings = Vec::new();

    if resources.gpu_count.unwrap_or(0) > 0 && !capabilities.supports_gpu {
        warnings.push("Scheduler does not advertise GPU support".to_string());
    }

    if !resources.packages.is_empty() && !capabilities.supports_python {
        warnings.push("Scheduler does not advertise Python support".to_string());
    }

    warnings
}

pub fn validate_job(spec: &JobSpec, capabilities: &SchedulerCapabilities) -> Vec<String> {
    validate_resources(&spec.resources, capabilities)
}
