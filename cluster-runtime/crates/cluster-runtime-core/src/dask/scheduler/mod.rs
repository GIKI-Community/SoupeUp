use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::dask::process_util::{background_process_error, format_process_logs, is_ready};
use crate::dask::scripts;
use crate::dask::settings::DaskSettings;
use crate::dask::types::{ComponentStatus, DaskError, DaskResult, SchedulerInfo};
use crate::python_runtime::PythonExecutionService;
use crate::python_runtime::ProcessStatus;

pub struct SchedulerManager {
    python: Arc<PythonExecutionService>,
    settings: Arc<RwLock<DaskSettings>>,
    info: Arc<RwLock<SchedulerInfo>>,
}

impl SchedulerManager {
    pub fn new(
        python: Arc<PythonExecutionService>,
        settings: Arc<RwLock<DaskSettings>>,
    ) -> Self {
        Self {
            python,
            settings,
            info: Arc::new(RwLock::new(SchedulerInfo::default())),
        }
    }

    pub async fn status(&self) -> SchedulerInfo {
        let mut info = self.info.write().await;
        if let Some(pid) = info.process_id.clone() {
            match self.python.process_status(&pid).await {
                Ok(proc) => {
                    info.logs = format_process_logs(&proc);
                    match proc.status {
                        ProcessStatus::Running | ProcessStatus::Starting => {
                            info.status = ComponentStatus::Running;
                            // Prefer READY line from stdout when available.
                            if let Some(line) = proc
                                .stdout_tail
                                .lines()
                                .rev()
                                .find(|l| l.starts_with("DASK_SCHEDULER_READY "))
                            {
                                let json = line.trim_start_matches("DASK_SCHEDULER_READY ");
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
                                    info.address = v
                                        .get("address")
                                        .and_then(|x| x.as_str())
                                        .map(|s| s.to_string());
                                    info.dashboard_url = v
                                        .get("dashboard")
                                        .and_then(|x| x.as_str())
                                        .map(|s| s.to_string());
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

    pub async fn start(&self) -> DaskResult<SchedulerInfo> {
        {
            let current = self.status().await;
            if current.status == ComponentStatus::Running {
                return Ok(current);
            }
        }

        let settings = self.settings.read().await.clone();
        {
            let mut info = self.info.write().await;
            info.status = ComponentStatus::Starting;
            info.host = settings.scheduler_host.clone();
            info.port = settings.scheduler_port;
            info.dashboard_port = settings.dashboard_port;
            info.error = None;
        }

        let code = scripts::scheduler_script(
            &settings.scheduler_host,
            settings.scheduler_port,
            settings.dashboard_port,
            &settings.logging_level,
        );

        let proc = self
            .python
            .spawn_code(&code, "dask-scheduler", None)
            .await
            .map_err(|e| DaskError::SchedulerError(e.to_string()))?;

        // Wait for the READY marker (up to ~15s).
        let mut ready = false;
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Ok(status) = self.python.process_status(&proc.id).await {
                if is_ready(&status, "DASK_SCHEDULER_READY ") {
                    ready = true;
                    break;
                }
                if status.status == ProcessStatus::Failed
                    || status.status == ProcessStatus::Exited
                {
                    let err = background_process_error(&status).unwrap_or_else(|| {
                        "Scheduler process exited before becoming ready".to_string()
                    });
                    {
                        let mut info = self.info.write().await;
                        info.logs = format_process_logs(&status);
                        info.status = ComponentStatus::Error;
                        info.error = Some(err.clone());
                    }
                    return Err(DaskError::SchedulerError(err));
                }
            }
        }

        if !ready {
            if let Ok(status) = self.python.process_status(&proc.id).await {
                let mut info = self.info.write().await;
                info.logs = format_process_logs(&status);
                info.status = ComponentStatus::Error;
                info.error = Some("Scheduler did not become ready in time.".to_string());
            }
            let _ = self.python.stop_process(&proc.id).await;
            return Err(DaskError::SchedulerError(
                "Scheduler did not become ready in time.".to_string(),
            ));
        }

        let mut info = self.info.write().await;
        info.process_id = Some(proc.id.clone());
        info.started_at = Some(Utc::now());
        info.status = ComponentStatus::Running;
        info.dashboard_url = Some(settings.dashboard_url());
        // Prefer a connectable address for local clients.
        let connect_host = if settings.scheduler_host == "0.0.0.0" {
            "127.0.0.1"
        } else {
            &settings.scheduler_host
        };
        info.address = Some(format!("tcp://{}:{}", connect_host, settings.scheduler_port));

        log::info!(
            "Dask scheduler started (process={}, address={:?})",
            proc.id,
            info.address
        );

        Ok(info.clone())
    }

    pub async fn stop(&self) -> DaskResult<SchedulerInfo> {
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
        info.address = None;
        info.error = None;
        log::info!("Dask scheduler stopped");
        Ok(info.clone())
    }

    pub async fn restart(&self) -> DaskResult<SchedulerInfo> {
        let _ = self.stop().await;
        self.start().await
    }

    pub async fn health(&self) -> ComponentStatus {
        self.status().await.status
    }
}
