use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ray::process_util::{background_process_error, format_process_logs, is_ready};
use crate::ray::scripts;
use crate::ray::settings::RaySettings;
use crate::ray::types::{ComponentStatus, HeadInfo, RayError, RayResult};
use crate::python_runtime::PythonExecutionService;
use crate::python_runtime::ProcessStatus;

pub struct HeadManager {
    python: Arc<PythonExecutionService>,
    settings: Arc<RwLock<RaySettings>>,
    info: Arc<RwLock<HeadInfo>>,
}

impl HeadManager {
    pub fn new(
        python: Arc<PythonExecutionService>,
        settings: Arc<RwLock<RaySettings>>,
    ) -> Self {
        Self {
            python,
            settings,
            info: Arc::new(RwLock::new(HeadInfo::default())),
        }
    }

    pub async fn status(&self) -> HeadInfo {
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
                                .find(|l| l.starts_with("RAY_HEAD_READY "))
                            {
                                let json = line.trim_start_matches("RAY_HEAD_READY ");
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
                                    info.address = v
                                        .get("connectAddress")
                                        .or_else(|| v.get("address"))
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

    pub async fn start(&self) -> RayResult<HeadInfo> {
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
            info.host = settings.head_host.clone();
            info.port = settings.gcs_port;
            info.dashboard_port = settings.dashboard_port;
            info.error = None;
        }

        let code = scripts::head_script(
            &settings.head_host,
            settings.gcs_port,
            settings.dashboard_port,
            settings.worker_cpus,
            &settings.logging_level,
        );

        let proc = self
            .python
            .spawn_code(&code, "ray-head", None)
            .await
            .map_err(|e| RayError::HeadError(e.to_string()))?;

        let mut ready = false;
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Ok(status) = self.python.process_status(&proc.id).await {
                if is_ready(&status, "RAY_HEAD_READY ") {
                    ready = true;
                    break;
                }
                if status.status == ProcessStatus::Failed
                    || status.status == ProcessStatus::Exited
                {
                    let err = background_process_error(&status).unwrap_or_else(|| {
                        "Ray head process exited before becoming ready".to_string()
                    });
                    {
                        let mut info = self.info.write().await;
                        info.logs = format_process_logs(&status);
                        info.status = ComponentStatus::Error;
                        info.error = Some(err.clone());
                    }
                    return Err(RayError::HeadError(err));
                }
            }
        }

        if !ready {
            if let Ok(status) = self.python.process_status(&proc.id).await {
                let mut info = self.info.write().await;
                info.logs = format_process_logs(&status);
                info.status = ComponentStatus::Error;
                info.error = Some("Ray head did not become ready in time.".to_string());
            }
            let _ = self.python.stop_process(&proc.id).await;
            return Err(RayError::HeadError(
                "Ray head did not become ready in time.".to_string(),
            ));
        }

        let connect_host = if settings.head_host == "0.0.0.0" {
            "127.0.0.1"
        } else {
            &settings.head_host
        };
        let mut info = self.info.write().await;
        info.process_id = Some(proc.id.clone());
        info.started_at = Some(Utc::now());
        info.status = ComponentStatus::Running;
        info.dashboard_url = Some(settings.dashboard_url());
        info.address = Some(format!("{}:{}", connect_host, settings.gcs_port));

        log::info!(
            "Ray head started (process={}, address={:?})",
            proc.id,
            info.address
        );

        Ok(info.clone())
    }

    pub async fn stop(&self) -> RayResult<HeadInfo> {
        self.stop_inner(true).await
    }

    /// Stop the head wrapper process. When `graceful_cluster` is true, run
    /// `ray stop --force` first so GCS/raylet children are torn down.
    pub async fn stop_inner(&self, graceful_cluster: bool) -> RayResult<HeadInfo> {
        if graceful_cluster {
            self.python.ray_stop_force().await;
        }

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
        log::info!("Ray head stopped");
        Ok(info.clone())
    }

    pub async fn restart(&self) -> RayResult<HeadInfo> {
        let _ = self.stop().await;
        self.start().await
    }

    pub async fn health(&self) -> ComponentStatus {
        self.status().await.status
    }
}
