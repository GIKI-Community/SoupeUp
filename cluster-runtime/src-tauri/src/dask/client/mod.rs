use std::sync::Arc;
use tokio::sync::RwLock;

use crate::dask::scripts;
use crate::dask::settings::DaskSettings;
use crate::dask::types::{DaskError, DaskResult, JobResult};
use crate::python_runtime::{ExecutionContext, PythonExecutionService};

/// Thin Rust adapter around `distributed.Client`.
/// Never talks to Dask sockets directly — always via PythonExecutionService.
pub struct ClientManager {
    python: Arc<PythonExecutionService>,
    settings: Arc<RwLock<DaskSettings>>,
    connected_address: Arc<RwLock<Option<String>>>,
}

impl ClientManager {
    pub fn new(
        python: Arc<PythonExecutionService>,
        settings: Arc<RwLock<DaskSettings>>,
    ) -> Self {
        Self {
            python,
            settings,
            connected_address: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn connect(&self, address: Option<String>) -> DaskResult<String> {
        let settings = self.settings.read().await.clone();
        let addr = address
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| settings.scheduler_address.clone());

        // Avoid spawning a new Python probe on every UI poll when already connected.
        if self.connected_address.read().await.as_ref() == Some(&addr) {
            return Ok(addr);
        }

        let code = scripts::cluster_info_script(&addr);
        let result = self
            .python
            .execute_code(
                &code,
                Some(ExecutionContext {
                    timeout_secs: Some(20),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| DaskError::ClientError(e.to_string()))?;

        let payload = parse_json_stdout(&result.stdout)?;
        if payload.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let err = payload
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to connect to scheduler");
            return Err(DaskError::ClientError(err.to_string()));
        }

        *self.connected_address.write().await = Some(addr.clone());
        log::info!("Dask client connected to {}", addr);
        Ok(addr)
    }

    pub async fn disconnect(&self) -> DaskResult<()> {
        *self.connected_address.write().await = None;
        log::info!("Dask client disconnected");
        Ok(())
    }

    /// Remember the scheduler address without spawning a Python probe.
    pub async fn set_address(&self, address: String) {
        *self.connected_address.write().await = Some(address);
    }

    pub async fn is_connected(&self) -> bool {
        self.connected_address.read().await.is_some()
    }

    pub async fn address(&self) -> Option<String> {
        self.connected_address.read().await.clone()
    }

    async fn require_address(&self) -> DaskResult<String> {
        if let Some(addr) = self.connected_address.read().await.clone() {
            return Ok(addr);
        }
        // Auto-connect using settings for convenience.
        self.connect(None).await
    }

    pub async fn cluster_info(&self) -> DaskResult<serde_json::Value> {
        let addr = self.require_address().await?;
        let code = scripts::cluster_info_script(&addr);
        let result = self
            .python
            .execute_code(
                &code,
                Some(ExecutionContext {
                    timeout_secs: Some(20),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| DaskError::ClientError(e.to_string()))?;
        parse_json_stdout(&result.stdout)
    }

    pub async fn submit(
        &self,
        function_body: &str,
        args: serde_json::Value,
    ) -> DaskResult<JobResult> {
        let addr = self.require_address().await?;
        let args_json = serde_json::to_string(&args)
            .map_err(|e| DaskError::JsonError(e.to_string()))?;
        let code = scripts::submit_function_script(&addr, function_body, &args_json);
        self.run_job_script(&code).await
    }

    /// Run distributed orchestration in the client process (`client` + `ARGS` in scope).
    pub async fn orchestrate(
        &self,
        body: &str,
        args: serde_json::Value,
    ) -> DaskResult<JobResult> {
        let addr = self.require_address().await?;
        let args_json = serde_json::to_string(&args)
            .map_err(|e| DaskError::JsonError(e.to_string()))?;
        let code = scripts::orchestration_script(&addr, body, &args_json);
        self.run_job_script(&code).await
    }

    pub async fn map(
        &self,
        function_body: &str,
        items: serde_json::Value,
    ) -> DaskResult<JobResult> {
        let addr = self.require_address().await?;
        let items_json = serde_json::to_string(&items)
            .map_err(|e| DaskError::JsonError(e.to_string()))?;
        let code = scripts::map_script(&addr, function_body, &items_json);
        self.run_job_script(&code).await
    }

    pub async fn scatter(&self, data: serde_json::Value) -> DaskResult<JobResult> {
        let body = r#"
def user_fn(payload):
    return payload
"#;
        self.submit(body, serde_json::json!([data])).await
    }

    pub async fn gather(&self, keys: serde_json::Value) -> DaskResult<JobResult> {
        // For MVP, gather is expressed as an identity map over already-materialized values.
        let body = r#"
def user_fn(item):
    return item
"#;
        self.map(body, keys).await
    }

    pub async fn cancel(&self, _job_id: &str) -> DaskResult<()> {
        // Short-lived clients finish or time out; explicit cancel hooks can be added later.
        Ok(())
    }

    pub async fn shutdown(&self) -> DaskResult<()> {
        self.disconnect().await
    }

    async fn run_job_script(&self, code: &str) -> DaskResult<JobResult> {
        let result = self
            .python
            .execute_code(
                code,
                Some(ExecutionContext {
                    timeout_secs: Some(660),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| DaskError::JobError(e.to_string()))?;

        let payload = match parse_json_stdout(&result.stdout) {
            Ok(payload) => payload,
            Err(e) => {
                let mut detail = result.stderr.trim().to_string();
                if detail.is_empty() {
                    detail = result.stdout.trim().to_string();
                }
                if detail.is_empty() {
                    detail = format!("Python exited with code {}", result.exit_code);
                }
                return Ok(JobResult {
                    job_id: uuid::Uuid::new_v4().to_string(),
                    success: false,
                    result: None,
                    error: Some(format!("{} Output: {}", e, truncate_output(&detail, 800))),
                    execution_time_ms: result.execution_time_ms,
                    workers_used: 0,
                    cpu_utilization: None,
                    speedup: None,
                });
            }
        };

        if payload.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let err = payload
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Job failed")
                .to_string();
            return Ok(JobResult {
                job_id: uuid::Uuid::new_v4().to_string(),
                success: false,
                result: None,
                error: Some(err),
                execution_time_ms: result.execution_time_ms,
                workers_used: 0,
                cpu_utilization: None,
                speedup: None,
            });
        }

        Ok(JobResult {
            job_id: uuid::Uuid::new_v4().to_string(),
            success: true,
            result: payload.get("result").cloned(),
            error: None,
            execution_time_ms: payload
                .get("executionTimeMs")
                .and_then(|v| v.as_u64())
                .unwrap_or(result.execution_time_ms),
            workers_used: payload
                .get("workersUsed")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            cpu_utilization: None,
            speedup: None,
        })
    }
}

fn parse_json_stdout(stdout: &str) -> DaskResult<serde_json::Value> {
    let trimmed = stdout.trim();
    // Take the last JSON object in case of logging noise.
    let json_line = trimmed
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or(trimmed);
    serde_json::from_str(json_line).map_err(|e| {
        DaskError::JsonError(format!(
            "Failed to parse client JSON: {} (stdout={:?})",
            e, stdout
        ))
    })
}

fn truncate_output(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
