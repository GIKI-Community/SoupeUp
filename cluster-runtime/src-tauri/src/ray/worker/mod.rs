use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ray::process_util::{background_process_error, format_process_logs, is_ready};
use crate::ray::scripts;
use crate::ray::settings::RaySettings;
use crate::ray::types::{ComponentStatus, RayError, RayResult, WorkerInfo};
use crate::python_runtime::PythonExecutionService;
use crate::python_runtime::ProcessStatus;

pub struct WorkerManager {
    python: Arc<PythonExecutionService>,
    settings: Arc<RwLock<RaySettings>>,
    info: Arc<RwLock<WorkerInfo>>,
}

impl WorkerManager {
    pub fn new(
        python: Arc<PythonExecutionService>,
        settings: Arc<RwLock<RaySettings>>,
    ) -> Self {
        Self {
            python,
            settings,
            info: Arc::new(RwLock::new(WorkerInfo::default())),
        }
    }

    pub async fn status(&self) -> WorkerInfo {
        let mut info = self.info.write().await;
        if let Some(pid) = info.process_id.clone() {
            match self.python.process_status(&pid).await {
                Ok(proc) => {
                    info.logs = format_process_logs(&proc);
                    match proc.status {
                        ProcessStatus::Running | ProcessStatus::Starting => {
                            info.status = ComponentStatus::Running;
                            if let Some(line) = proc
                                .stdout_tail
                                .lines()
                                .rev()
                                .find(|l| l.starts_with("RAY_WORKER_READY "))
                            {
                                let json = line.trim_start_matches("RAY_WORKER_READY ");
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
                                    if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
                                        info.name = name.to_string();
                                    }
                                }
                            }
                        }
                        ProcessStatus::Failed => {
                            info.status = ComponentStatus::Error;
                            info.error = background_process_error(&proc);
                        }
                        ProcessStatus::Exited | ProcessStatus::Stopped => {
                            info.status = ComponentStatus::Stopped;
                            info.process_id = None;
                            info.error = None;
                        }
                    }
                }
                Err(_) => {
                    info.status = ComponentStatus::Unknown;
                }
            }
        }
        info.clone()
    }

    pub async fn start(&self, head_address: Option<String>) -> RayResult<WorkerInfo> {
        {
            let current = self.status().await;
            if current.status == ComponentStatus::Running {
                return Ok(current);
            }
        }

        {
            let old_pid = self.info.read().await.process_id.clone();
            if let Some(pid) = old_pid {
                let _ = self.python.stop_process(&pid).await;
            }
        }

        let settings = self.settings.read().await.clone();
        let address = head_address
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| settings.head_address.clone());

        {
            let mut info = self.info.write().await;
            info.status = ComponentStatus::Starting;
            info.head_address = address.clone();
            info.name = settings.worker_name.clone();
            info.num_cpus = settings.worker_cpus;
            info.object_store_memory = if settings.object_store_memory.is_empty() {
                None
            } else {
                Some(settings.object_store_memory.clone())
            };
            info.error = None;
        }

        let code = scripts::worker_script(
            &address,
            &settings.worker_name,
            settings.worker_cpus,
            &settings.object_store_memory,
            &settings.logging_level,
        );

        let proc = self
            .python
            .spawn_code(&code, "ray-worker", None)
            .await
            .map_err(|e| RayError::WorkerError(e.to_string()))?;

        let mut ready = false;
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Ok(status) = self.python.process_status(&proc.id).await {
                if is_ready(&status, "RAY_WORKER_READY ") {
                    ready = true;
                    break;
                }
                if status.status == ProcessStatus::Failed
                    || status.status == ProcessStatus::Exited
                {
                    let err = background_process_error(&status)
                        .unwrap_or_else(|| "Worker process exited before becoming ready".to_string());
                    {
                        let mut info = self.info.write().await;
                        info.logs = format_process_logs(&status);
                        info.status = ComponentStatus::Error;
                        info.error = Some(err.clone());
                    }
                    return Err(RayError::WorkerError(err));
                }
            }
        }

        if !ready {
            if let Ok(status) = self.python.process_status(&proc.id).await {
                let mut info = self.info.write().await;
                info.logs = format_process_logs(&status);
                info.status = ComponentStatus::Error;
                info.error = Some(
                    "Worker did not become ready in time. Check that the Ray head is running and reachable."
                        .to_string(),
                );
            }
            let _ = self.python.stop_process(&proc.id).await;
            return Err(RayError::WorkerError(
                "Worker did not become ready in time. Check that the Ray head is running and reachable."
                    .to_string(),
            ));
        }

        let mut info = self.info.write().await;
        info.process_id = Some(proc.id.clone());
        info.started_at = Some(Utc::now());
        info.status = ComponentStatus::Running;

        log::info!(
            "Ray worker started (process={}, head={})",
            proc.id,
            address
        );

        Ok(info.clone())
    }

    pub async fn stop(&self) -> RayResult<WorkerInfo> {
        let pid = {
            let info = self.info.read().await;
            info.process_id.clone()
        };

        if let Some(pid) = pid {
            let _ = self.python.stop_process(&pid).await;
        }

        let mut info = self.info.write().await;
        info.status = ComponentStatus::Stopped;
        info.process_id = None;
        info.error = None;
        log::info!("Ray worker stopped");
        Ok(info.clone())
    }

    pub async fn restart(&self) -> RayResult<WorkerInfo> {
        let address = self.info.read().await.head_address.clone();
        let _ = self.stop().await;
        self.start(Some(address)).await
    }

    pub async fn health(&self) -> ComponentStatus {
        self.status().await.status
    }
}
