//! Embedded Python scripts that drive Dask via its official Python API.
//! These are never invoked via the `dask-scheduler` / `dask-worker` CLIs.

/// Shared Python preamble: log to stdout (not stderr) so Rust doesn't treat INFO logs as errors.
const LOGGING_PREAMBLE: &str = r#"
import sys
import logging

def _configure_logging(level_name):
    level = getattr(logging, level_name.upper(), logging.INFO)
    logging.basicConfig(
        level=level,
        stream=sys.stdout,
        format="%(levelname)s:%(name)s: %(message)s",
    )
    # Keep distributed chatter on stdout; stderr is reserved for real failures.
    for name in ("distributed", "dask"):
        logging.getLogger(name).setLevel(level)
"#;

/// Long-lived scheduler process using `distributed.Scheduler`.
pub fn scheduler_script(host: &str, port: u16, dashboard_port: u16, log_level: &str) -> String {
    format!(
        r#"
import asyncio
import json
import sys

{logging_preamble}
_configure_logging({log_level:?})

from distributed import Scheduler

HOST = {host:?}
PORT = {port}
DASHBOARD_PORT = {dashboard_port}

async def main():
    async with Scheduler(
        host=HOST,
        port=PORT,
        dashboard_address=f":{{DASHBOARD_PORT}}",
    ) as scheduler:
        info = {{
            "ok": True,
            "address": scheduler.address,
            "dashboard": f"http://127.0.0.1:{{DASHBOARD_PORT}}/status",
            "services": list(getattr(scheduler, "services", {{}}).keys()),
        }}
        print("DASK_SCHEDULER_READY " + json.dumps(info), flush=True)
        # Run until the parent process terminates us (reliable on Windows).
        await asyncio.Future()

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except Exception as exc:
        print("DASK_SCHEDULER_ERROR " + json.dumps({{"ok": False, "error": str(exc)}}), flush=True)
        sys.exit(1)
"#,
        logging_preamble = LOGGING_PREAMBLE,
        host = host,
        port = port,
        dashboard_port = dashboard_port,
        log_level = log_level,
    )
}

/// Long-lived worker process using `distributed.Worker`.
pub fn worker_script(
    scheduler_address: &str,
    name: &str,
    nthreads: usize,
    memory_limit: &str,
    local_directory: &str,
    log_level: &str,
) -> String {
    let nthreads_expr = if nthreads == 0 {
        "None".to_string()
    } else {
        nthreads.to_string()
    };
    let memory_expr = if memory_limit.trim().is_empty() {
        "None".to_string()
    } else {
        format!("{:?}", memory_limit)
    };
    let local_dir_expr = if local_directory.trim().is_empty() {
        "None".to_string()
    } else {
        format!("{:?}", local_directory)
    };

    format!(
        r#"
import asyncio
import json
import sys
import uuid

{logging_preamble}
_configure_logging({log_level:?})

from distributed import Worker

SCHEDULER = {scheduler_address:?}
BASE_NAME = {name:?}
# Unique name avoids collisions when restarting workers on the same machine.
NAME = f"{{BASE_NAME}}-{{uuid.uuid4().hex[:8]}}"
NTHREADS = {nthreads_expr}
MEMORY_LIMIT = {memory_expr}
LOCAL_DIRECTORY = {local_dir_expr}

async def main():
    kwargs = {{
        "name": NAME,
    }}
    if NTHREADS is not None:
        kwargs["nthreads"] = NTHREADS
    if MEMORY_LIMIT is not None:
        kwargs["memory_limit"] = MEMORY_LIMIT
    if LOCAL_DIRECTORY is not None:
        kwargs["local_directory"] = LOCAL_DIRECTORY

    async with Worker(SCHEDULER, **kwargs) as worker:
        thread_count = getattr(worker, "nthreads", None)
        if thread_count is None:
            thread_count = NTHREADS
        info = {{
            "ok": True,
            "name": worker.name,
            "address": worker.address,
            "scheduler": SCHEDULER,
            "nthreads": thread_count,
        }}
        print("DASK_WORKER_READY " + json.dumps(info), flush=True)
        # Run until the parent process terminates us (reliable on Windows).
        await asyncio.Future()

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except Exception as exc:
        print("DASK_WORKER_ERROR " + json.dumps({{"ok": False, "error": str(exc)}}), flush=True)
        sys.exit(1)
"#,
        logging_preamble = LOGGING_PREAMBLE,
        scheduler_address = scheduler_address,
        name = name,
        nthreads_expr = nthreads_expr,
        memory_expr = memory_expr,
        local_dir_expr = local_dir_expr,
        log_level = log_level,
    )
}

/// One-shot client probe: cluster info via `distributed.Client`.
pub fn cluster_info_script(scheduler_address: &str) -> String {
    format!(
        r#"
import json
import sys

{logging_preamble}
_configure_logging("warning")

from distributed import Client

ADDRESS = {scheduler_address:?}

try:
    with Client(ADDRESS, timeout="10s") as client:
        info = client.scheduler_info()
        workers = []
        total_cores = 0
        total_memory = 0
        for wid, w in info.get("workers", {{}}).items():
            nthreads = int(w.get("nthreads") or 0)
            mem_limit = int(w.get("memory_limit") or 0)
            metrics = w.get("metrics") or {{}}
            workers.append({{
                "id": wid,
                "name": w.get("name") or wid,
                "address": w.get("address") or wid,
                "nthreads": nthreads,
                "memoryLimit": mem_limit,
                "memoryUsed": int(metrics.get("memory") or 0),
                "cpu": float(metrics.get("cpu") or 0.0),
                "status": w.get("status") or "unknown",
            }})
            total_cores += nthreads
            total_memory += mem_limit

        processing = info.get("processing") or {{}}
        active = sum(len(v) for v in processing.values()) if isinstance(processing, dict) else 0

        out = {{
            "ok": True,
            "address": info.get("address"),
            "workers": workers,
            "totalCores": total_cores,
            "totalMemory": total_memory,
            "activeTasks": active,
            "services": info.get("services") or {{}},
            "workerCount": len(workers),
        }}
        print(json.dumps(out), flush=True)
except Exception as exc:
    print(json.dumps({{"ok": False, "error": str(exc)}}), flush=True)
    sys.exit(1)
"#,
        logging_preamble = LOGGING_PREAMBLE,
        scheduler_address = scheduler_address,
    )
}

/// Submit an arbitrary Python function body as a Dask future.
pub fn submit_function_script(scheduler_address: &str, function_body: &str, args_json: &str) -> String {
    format!(
        r#"
import json
import time
import sys
from distributed import Client

ADDRESS = {scheduler_address:?}
ARGS = json.loads({args_json:?})

{function_body}

try:
    started = time.time()
    with Client(ADDRESS, timeout="10s") as client:
        workers = len(client.scheduler_info().get("workers", {{}}))
        if workers == 0:
            raise RuntimeError(
                "No Dask workers connected. Start at least one worker on the Cluster page."
            )
        fut = client.submit(user_fn, *ARGS)
        result = fut.result(timeout=600)
        elapsed_ms = int((time.time() - started) * 1000)
        print(json.dumps({{
            "ok": True,
            "result": result,
            "executionTimeMs": elapsed_ms,
            "workersUsed": workers,
        }}, default=str))
except Exception as exc:
    print(json.dumps({{"ok": False, "error": str(exc)}}))
    sys.exit(1)
"#,
        scheduler_address = scheduler_address,
        args_json = args_json,
        function_body = function_body,
    )
}

fn indent_body(body: &str) -> String {
    body.replace('\r', "")
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("        {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

const ORCHESTRATION_PREAMBLE: &str = r#"
        workers = len(client.scheduler_info().get("workers", {}))
        if workers == 0:
            raise RuntimeError(
                "No Dask workers connected. Start at least one worker on the Cluster page."
            )
"#;

/// Run orchestration code in the client process (uses `client` + `ARGS`, sets `result`).
/// Use this for multi-step distributed jobs — not `client.submit` of a function that calls `get_client()`.
pub fn orchestration_script(scheduler_address: &str, body: &str, args_json: &str) -> String {
    let indented_body = indent_body(body);
    format!(
        r#"
import json
import time
import sys
from distributed import Client

ADDRESS = {scheduler_address:?}
ARGS = json.loads({args_json:?})

try:
    started = time.time()
    with Client(ADDRESS, timeout="10s") as client:
{orchestration_preamble}{indented_body}
        elapsed_ms = int((time.time() - started) * 1000)
        print(json.dumps({{
            "ok": True,
            "result": result,
            "executionTimeMs": elapsed_ms,
            "workersUsed": workers,
        }}, default=str))
except Exception as exc:
    print(json.dumps({{"ok": False, "error": str(exc)}}))
    sys.exit(1)
"#,
        scheduler_address = scheduler_address,
        args_json = args_json,
        orchestration_preamble = ORCHESTRATION_PREAMBLE,
        indented_body = indented_body,
    )
}

/// Map a function over an iterable via Client.map.
pub fn map_script(scheduler_address: &str, function_body: &str, items_json: &str) -> String {
    format!(
        r#"
import json
import time
import sys
from distributed import Client

ADDRESS = {scheduler_address:?}
ITEMS = json.loads({items_json:?})

{function_body}

try:
    started = time.time()
    with Client(ADDRESS, timeout="10s") as client:
        workers = len(client.scheduler_info().get("workers", {{}}))
        if workers == 0:
            raise RuntimeError(
                "No Dask workers connected. Start at least one worker on the Cluster page."
            )
        futures = client.map(user_fn, ITEMS)
        result = client.gather(futures)
        elapsed_ms = int((time.time() - started) * 1000)
        print(json.dumps({{
            "ok": True,
            "result": result,
            "executionTimeMs": elapsed_ms,
            "workersUsed": workers,
        }}, default=str))
except Exception as exc:
    print(json.dumps({{"ok": False, "error": str(exc)}}))
    sys.exit(1)
"#,
        scheduler_address = scheduler_address,
        items_json = items_json,
        function_body = function_body,
    )
}
