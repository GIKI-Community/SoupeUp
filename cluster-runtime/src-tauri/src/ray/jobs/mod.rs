use std::sync::Arc;

use crate::ray::client::ClientManager;
use crate::ray::examples;
use crate::ray::types::{ExampleJobResult, JobResult, RayResult};

pub struct JobService {
    client: Arc<ClientManager>,
}

impl JobService {
    pub fn new(client: Arc<ClientManager>) -> Self {
        Self { client }
    }

    pub async fn submit_python_function(
        &self,
        function_body: String,
        args: serde_json::Value,
    ) -> RayResult<JobResult> {
        self.client.submit(&function_body, args).await
    }

    pub async fn submit_script(&self, script: String) -> RayResult<JobResult> {
        let body = format!(
            r#"
def user_fn(_unused=None):
    ns = {{}}
    exec({script:?}, ns, ns)
    if "main" in ns and callable(ns["main"]):
        return ns["main"]()
    return ns.get("result")
"#
        );
        self.client.submit(&body, serde_json::json!([null])).await
    }

    pub async fn submit_module(&self, module: String) -> RayResult<JobResult> {
        let body = format!(
            r#"
def user_fn(_unused=None):
    import importlib
    mod = importlib.import_module({module:?})
    if hasattr(mod, "main") and callable(mod.main):
        return mod.main()
    return str(mod)
"#
        );
        self.client.submit(&body, serde_json::json!([null])).await
    }

    pub async fn map(
        &self,
        function_body: String,
        items: serde_json::Value,
    ) -> RayResult<JobResult> {
        self.client.map(&function_body, items).await
    }

    pub async fn scatter(&self, data: serde_json::Value) -> RayResult<JobResult> {
        self.client.scatter(data).await
    }

    pub async fn gather(&self, keys: serde_json::Value) -> RayResult<JobResult> {
        self.client.gather(keys).await
    }

    pub async fn cancel_job(&self, job_id: String) -> RayResult<()> {
        self.client.cancel(&job_id).await
    }

    pub async fn job_status(&self, _job_id: String) -> RayResult<serde_json::Value> {
        Ok(serde_json::json!({ "status": "unknown" }))
    }

    pub async fn run_example(&self, example_id: &str) -> RayResult<ExampleJobResult> {
        let Some(spec) = examples::get(example_id) else {
            return Ok(ExampleJobResult {
                example_id: example_id.to_string(),
                title: "Unknown".to_string(),
                success: false,
                execution_time_ms: 0,
                workers_used: 0,
                cpu_utilization: None,
                speedup: None,
                result_summary: String::new(),
                details: None,
                error: Some(format!("Unknown example: {}", example_id)),
            });
        };

        let args = spec.args.clone();

        let distributed = match self
            .client
            .orchestrate(spec.distributed_body, args.clone())
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return Ok(failed_result(spec, e.to_string()));
            }
        };

        let speedup = if let Some(single) = spec.single_body {
            match self.client.submit(single, args).await {
                Ok(baseline) if baseline.success && baseline.execution_time_ms > 0 => {
                    Some(
                        baseline.execution_time_ms as f64
                            / distributed.execution_time_ms.max(1) as f64,
                    )
                }
                _ => None,
            }
        } else {
            None
        };

        let summary = if distributed.success {
            distributed
                .result
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "completed".to_string())
        } else {
            distributed
                .error
                .clone()
                .unwrap_or_else(|| "failed".to_string())
        };

        Ok(ExampleJobResult {
            example_id: spec.id.to_string(),
            title: spec.title.to_string(),
            success: distributed.success,
            execution_time_ms: distributed.execution_time_ms,
            workers_used: distributed.workers_used,
            cpu_utilization: distributed.cpu_utilization,
            speedup,
            result_summary: truncate(&summary, 400),
            details: distributed.result,
            error: distributed
                .error
                .clone()
                .or_else(|| (!distributed.success).then(|| summary.clone())),
        })
    }
}

fn failed_result(spec: examples::ResolvedExample<'_>, message: String) -> ExampleJobResult {
    ExampleJobResult {
        example_id: spec.id.to_string(),
        title: spec.title.to_string(),
        success: false,
        execution_time_ms: 0,
        workers_used: 0,
        cpu_utilization: None,
        speedup: None,
        result_summary: String::new(),
        details: None,
        error: Some(message),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
