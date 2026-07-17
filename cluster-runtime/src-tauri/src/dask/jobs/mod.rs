use std::sync::Arc;

use crate::dask::client::ClientManager;
use crate::dask::types::{DaskResult, ExampleJobResult, JobResult};

/// Built-in demonstration jobs — zero coding required in the UI.
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
    ) -> DaskResult<JobResult> {
        self.client.submit(&function_body, args).await
    }

    pub async fn submit_script(&self, script: String) -> DaskResult<JobResult> {
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

    pub async fn submit_module(&self, module: String) -> DaskResult<JobResult> {
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
    ) -> DaskResult<JobResult> {
        self.client.map(&function_body, items).await
    }

    pub async fn scatter(&self, data: serde_json::Value) -> DaskResult<JobResult> {
        self.client.scatter(data).await
    }

    pub async fn gather(&self, keys: serde_json::Value) -> DaskResult<JobResult> {
        self.client.gather(keys).await
    }

    pub async fn cancel_job(&self, job_id: String) -> DaskResult<()> {
        self.client.cancel(&job_id).await
    }

    pub async fn job_status(&self, _job_id: String) -> DaskResult<serde_json::Value> {
        Ok(serde_json::json!({ "status": "unknown" }))
    }

    pub async fn run_example(&self, example_id: &str) -> DaskResult<ExampleJobResult> {
        let (title, body, args, single_body) = match example_id {
            "mandelbrot" => (
                "Mandelbrot Renderer",
                MANDELBROT_FN,
                serde_json::json!([800, 600, 80]),
                Some(MANDELBROT_SINGLE),
            ),
            "monte_carlo_pi" => (
                "Monte Carlo π Estimation",
                MONTE_CARLO_FN,
                serde_json::json!([2_000_000]),
                Some(MONTE_CARLO_SINGLE),
            ),
            "matrix_multiply" => (
                "Matrix Multiplication",
                MATRIX_FN,
                serde_json::json!([256]),
                Some(MATRIX_SINGLE),
            ),
            "prime_search" => (
                "Prime Number Search",
                PRIME_FN,
                serde_json::json!([50_000]),
                Some(PRIME_SINGLE),
            ),
            "image_blur" => (
                "Image Blur",
                BLUR_FN,
                serde_json::json!([200, 200]),
                Some(BLUR_SINGLE),
            ),
            "word_count" => (
                "Word Count",
                WORD_COUNT_FN,
                serde_json::json!([[
                    "to be or not to be that is the question",
                    "whether tis nobler in the mind to suffer",
                    "the slings and arrows of outrageous fortune",
                    "or to take arms against a sea of troubles"
                ]]),
                Some(WORD_COUNT_SINGLE),
            ),
            other => {
                return Ok(ExampleJobResult {
                    example_id: other.to_string(),
                    title: "Unknown".to_string(),
                    success: false,
                    execution_time_ms: 0,
                    workers_used: 0,
                    cpu_utilization: None,
                    speedup: None,
                    result_summary: String::new(),
                    details: None,
                    error: Some(format!("Unknown example: {}", other)),
                });
            }
        };

        // Distributed run
        let distributed = self.client.submit(body, args.clone()).await?;

        // Optional single-node baseline for speedup (same machine, sequential).
        let speedup = if let Some(single) = single_body {
            match self.client.submit(single, args).await {
                Ok(baseline) if baseline.success && baseline.execution_time_ms > 0 => {
                    Some(baseline.execution_time_ms as f64 / distributed.execution_time_ms.max(1) as f64)
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
            example_id: example_id.to_string(),
            title: title.to_string(),
            success: distributed.success,
            execution_time_ms: distributed.execution_time_ms,
            workers_used: distributed.workers_used,
            cpu_utilization: distributed.cpu_utilization,
            speedup,
            result_summary: truncate(&summary, 400),
            details: distributed.result,
            error: distributed.error,
        })
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

const MANDELBROT_FN: &str = r#"
def user_fn(width, height, max_iter):
    from distributed import get_client
    import numpy as np

    def row(y):
        row_out = []
        for x in range(width):
            c = complex(-2.0 + 2.7 * x / width, -1.2 + 2.4 * y / height)
            z = 0j
            n = 0
            while abs(z) <= 2 and n < max_iter:
                z = z * z + c
                n += 1
            row_out.append(n)
        return row_out

    client = get_client()
    futures = client.map(row, range(height))
    rows = client.gather(futures)
    arr = np.array(rows, dtype=np.uint16)
    return {
        "width": width,
        "height": height,
        "maxIter": max_iter,
        "meanIterations": float(arr.mean()),
        "maxReached": int((arr == max_iter).sum()),
    }
"#;

const MANDELBROT_SINGLE: &str = r#"
def user_fn(width, height, max_iter):
    import numpy as np
    arr = np.zeros((height, width), dtype=np.uint16)
    for y in range(height):
        for x in range(width):
            c = complex(-2.0 + 2.7 * x / width, -1.2 + 2.4 * y / height)
            z = 0j
            n = 0
            while abs(z) <= 2 and n < max_iter:
                z = z * z + c
                n += 1
            arr[y, x] = n
    return {
        "width": width,
        "height": height,
        "meanIterations": float(arr.mean()),
    }
"#;

const MONTE_CARLO_FN: &str = r#"
def user_fn(samples):
    from distributed import get_client
    import random

    def chunk(n):
        inside = 0
        for _ in range(n):
            x = random.random()
            y = random.random()
            if x * x + y * y <= 1.0:
                inside += 1
        return inside

    client = get_client()
    workers = max(1, len(client.scheduler_info().get("workers", {})))
    chunk_size = samples // workers
    sizes = [chunk_size] * workers
    sizes[-1] += samples - chunk_size * workers
    futures = client.map(chunk, sizes)
    inside = sum(client.gather(futures))
    pi = 4.0 * inside / samples
    return {"pi": pi, "samples": samples, "workers": workers}
"#;

const MONTE_CARLO_SINGLE: &str = r#"
def user_fn(samples):
    import random
    inside = 0
    for _ in range(samples):
        x = random.random()
        y = random.random()
        if x * x + y * y <= 1.0:
            inside += 1
    return {"pi": 4.0 * inside / samples, "samples": samples}
"#;

const MATRIX_FN: &str = r#"
def user_fn(n):
    from distributed import get_client
    import numpy as np

    def mul_block(i):
        a = np.random.rand(n, n)
        b = np.random.rand(n, n)
        return float(np.linalg.norm(a @ b))

    client = get_client()
    workers = max(1, len(client.scheduler_info().get("workers", {})))
    futures = client.map(mul_block, list(range(workers * 2)))
    norms = client.gather(futures)
    return {"blocks": len(norms), "meanNorm": float(sum(norms) / len(norms)), "n": n}
"#;

const MATRIX_SINGLE: &str = r#"
def user_fn(n):
    import numpy as np
    a = np.random.rand(n, n)
    b = np.random.rand(n, n)
    return {"norm": float(np.linalg.norm(a @ b)), "n": n}
"#;

const PRIME_FN: &str = r#"
def user_fn(limit):
    from distributed import get_client

    def is_prime(n):
        if n < 2:
            return False
        if n % 2 == 0:
            return n == 2
        i = 3
        while i * i <= n:
            if n % i == 0:
                return False
            i += 2
        return True

    client = get_client()
    futures = client.map(is_prime, range(limit))
    flags = client.gather(futures)
    count = sum(1 for f in flags if f)
    return {"limit": limit, "primeCount": count}
"#;

const PRIME_SINGLE: &str = r#"
def user_fn(limit):
    def is_prime(n):
        if n < 2:
            return False
        if n % 2 == 0:
            return n == 2
        i = 3
        while i * i <= n:
            if n % i == 0:
                return False
            i += 2
        return True
    count = sum(1 for n in range(limit) if is_prime(n))
    return {"limit": limit, "primeCount": count}
"#;

const BLUR_FN: &str = r#"
def user_fn(width, height):
    from distributed import get_client
    import numpy as np

    def blur_row(y):
        # Synthetic grayscale "image" row with a box blur against neighbors.
        row = np.sin(np.linspace(0, 8, width) + y * 0.05)
        kernel = np.array([0.25, 0.5, 0.25])
        padded = np.pad(row, 1, mode="edge")
        out = np.convolve(padded, kernel, mode="valid")
        return float(out.mean())

    client = get_client()
    futures = client.map(blur_row, range(height))
    means = client.gather(futures)
    return {"width": width, "height": height, "meanIntensity": float(sum(means) / len(means))}
"#;

const BLUR_SINGLE: &str = r#"
def user_fn(width, height):
    import numpy as np
    img = np.sin(np.linspace(0, 8, width * height).reshape(height, width))
    return {"meanIntensity": float(img.mean()), "width": width, "height": height}
"#;

const WORD_COUNT_FN: &str = r#"
def user_fn(lines):
    from distributed import get_client
    from collections import Counter

    def count_line(line):
        return Counter(w.lower() for w in line.split())

    client = get_client()
    futures = client.map(count_line, lines)
    partials = client.gather(futures)
    total = Counter()
    for p in partials:
        total.update(p)
    top = total.most_common(10)
    return {"uniqueWords": len(total), "top": top, "totalWords": sum(total.values())}
"#;

const WORD_COUNT_SINGLE: &str = r#"
def user_fn(lines):
    from collections import Counter
    total = Counter()
    for line in lines:
        total.update(w.lower() for w in line.split())
    return {"uniqueWords": len(total), "top": total.most_common(10)}
"#;
