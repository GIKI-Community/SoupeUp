//! Embedded Python scripts that drive Ray via its official Python API.

const LOGGING_PREAMBLE: &str = r#"
import os
import sys
import logging

def _configure_logging(level_name):
    level = getattr(logging, level_name.upper(), logging.INFO)
    logging.basicConfig(
        level=level,
        stream=sys.stdout,
        format="%(levelname)s:%(name)s: %(message)s",
    )
    os.environ["RAY_DEDUP_LOGS"] = "0"
"#;

pub fn head_script(
    host: &str,
    gcs_port: u16,
    dashboard_port: u16,
    num_cpus: usize,
    log_level: &str,
) -> String {
    let num_cpus_expr = if num_cpus == 0 {
        "None".to_string()
    } else {
        num_cpus.to_string()
    };
    let connect_host = if host == "0.0.0.0" {
        "127.0.0.1"
    } else {
        host
    };

    // Note: ray.init() does NOT accept `port=`. GCS port is set via `ray start --head --port`
    // (or defaults to 6379). We start the head with the Ray CLI so the configured port works.
    format!(
        r#"
import json
import shutil
import subprocess
import sys
import time

{logging_preamble}
_configure_logging({log_level:?})

HOST = {host:?}
GCS_PORT = {gcs_port}
DASHBOARD_PORT = {dashboard_port}
NUM_CPUS = {num_cpus_expr}
CONNECT_HOST = {connect_host:?}

def _ray_cmd():
    # Prefer the ray console script next to this interpreter (venv).
    sibling = os.path.join(os.path.dirname(sys.executable), "ray.exe" if os.name == "nt" else "ray")
    if os.path.isfile(sibling):
        return [sibling]
    found = shutil.which("ray")
    if found:
        return [found]
    return [sys.executable, "-m", "ray.scripts.scripts"]

try:
    cmd = _ray_cmd() + [
        "start",
        "--head",
        f"--port={{GCS_PORT}}",
        "--dashboard-host=0.0.0.0",
        f"--dashboard-port={{DASHBOARD_PORT}}",
        "--block",
    ]
    if HOST and HOST != "0.0.0.0":
        cmd.append(f"--node-ip-address={{HOST}}")
    if NUM_CPUS is not None:
        cmd.append(f"--num-cpus={{NUM_CPUS}}")

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )

    connect_address = f"{{CONNECT_HOST}}:{{GCS_PORT}}"
    dashboard = f"http://127.0.0.1:{{DASHBOARD_PORT}}"
    ready = False
    deadline = time.time() + 60
    while time.time() < deadline:
        if proc.poll() is not None:
            out = ""
            try:
                out = proc.stdout.read() if proc.stdout else ""
            except Exception:
                pass
            raise RuntimeError(
                f"ray start exited early (code={{proc.returncode}}): {{(out or '')[-2000:]}}"
            )
        try:
            import ray
            ray.init(address=connect_address, ignore_reinit_error=True, logging_level="warning")
            gcs = ray.get_runtime_context().gcs_address
            if gcs:
                connect_address = str(gcs)
            ray.shutdown()
            ready = True
            break
        except Exception:
            time.sleep(0.75)

    if not ready:
        try:
            proc.terminate()
        except Exception:
            pass
        raise RuntimeError("Ray head did not become ready in time.")

    info = {{
        "ok": True,
        "address": connect_address,
        "connectAddress": connect_address,
        "dashboard": dashboard,
    }}
    print("RAY_HEAD_READY " + json.dumps(info), flush=True)

    try:
        while True:
            if proc.poll() is not None:
                raise RuntimeError(f"ray start exited (code={{proc.returncode}})")
            time.sleep(1)
    finally:
        stop = _ray_cmd() + ["stop", "--force"]
        try:
            subprocess.run(stop, check=False, capture_output=True, text=True, timeout=45)
        except Exception:
            pass
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=10)
            except Exception:
                proc.kill()
except Exception as exc:
    print("RAY_HEAD_ERROR " + json.dumps({{"ok": False, "error": str(exc)}}), flush=True)
    sys.exit(1)
"#,
        logging_preamble = LOGGING_PREAMBLE,
        host = host,
        gcs_port = gcs_port,
        dashboard_port = dashboard_port,
        num_cpus_expr = num_cpus_expr,
        connect_host = connect_host,
        log_level = log_level,
    )
}

pub fn worker_script(
    head_address: &str,
    name: &str,
    num_cpus: usize,
    object_store_memory: &str,
    log_level: &str,
) -> String {
    let num_cpus_expr = if num_cpus == 0 {
        "None".to_string()
    } else {
        num_cpus.to_string()
    };
    let memory_expr = if object_store_memory.trim().is_empty() {
        "None".to_string()
    } else {
        format!("{:?}", object_store_memory)
    };

    format!(
        r#"
import json
import sys
import time
import uuid

{logging_preamble}
_configure_logging({log_level:?})

import ray

HEAD_ADDRESS = {head_address:?}
BASE_NAME = {name:?}
NAME = f"{{BASE_NAME}}-{{uuid.uuid4().hex[:8]}}"
NUM_CPUS = {num_cpus_expr}
OBJECT_STORE_MEMORY = {memory_expr}

kwargs = {{
    "address": HEAD_ADDRESS,
    "ignore_reinit_error": True,
    "logging_level": {log_level:?},
}}
if NUM_CPUS is not None:
    kwargs["num_cpus"] = NUM_CPUS
if OBJECT_STORE_MEMORY is not None:
    kwargs["object_store_memory"] = OBJECT_STORE_MEMORY

try:
    ray.init(**kwargs)
    info = {{
        "ok": True,
        "name": NAME,
        "address": HEAD_ADDRESS,
        "nodeId": ray.get_runtime_context().get_node_id(),
    }}
    print("RAY_WORKER_READY " + json.dumps(info), flush=True)
    while True:
        time.sleep(3600)
except Exception as exc:
    print("RAY_WORKER_ERROR " + json.dumps({{"ok": False, "error": str(exc)}}), flush=True)
    sys.exit(1)
"#,
        logging_preamble = LOGGING_PREAMBLE,
        head_address = head_address,
        name = name,
        num_cpus_expr = num_cpus_expr,
        memory_expr = memory_expr,
        log_level = log_level,
    )
}

pub fn cluster_info_script(head_address: &str) -> String {
    format!(
        r#"
import json
import sys

{logging_preamble}
_configure_logging("warning")

import ray

ADDRESS = {head_address:?}

try:
    ray.init(address=ADDRESS, ignore_reinit_error=True)
    nodes = ray.nodes()
    workers = []
    total_cores = 0
    total_memory = 0
    for node in nodes:
        if not node.get("Alive"):
            continue
        resources = node.get("Resources") or {{}}
        cpu = int(resources.get("CPU", 0))
        memory = int(resources.get("memory", 0))
        node_id = node.get("NodeID") or ""
        addr = node.get("NodeManagerAddress") or node_id
        workers.append({{
            "id": node_id,
            "name": addr,
            "address": addr,
            "nthreads": cpu,
            "memoryLimit": memory,
            "memoryUsed": 0,
            "cpu": 0.0,
            "status": "alive",
        }})
        total_cores += cpu
        total_memory += memory

    out = {{
        "ok": True,
        "address": ADDRESS,
        "workers": workers,
        "totalCores": total_cores,
        "totalMemory": total_memory,
        "activeTasks": 0,
        "workerCount": len(workers),
    }}
    print(json.dumps(out), flush=True)
    ray.shutdown()
except Exception as exc:
    print(json.dumps({{"ok": False, "error": str(exc)}}), flush=True)
    sys.exit(1)
"#,
        logging_preamble = LOGGING_PREAMBLE,
        head_address = head_address,
    )
}

pub fn submit_function_script(head_address: &str, function_body: &str, args_json: &str) -> String {
    format!(
        r#"
import json
import time
import sys
import ray

ADDRESS = {head_address:?}
ARGS = json.loads({args_json:?})

{function_body}

@ray.remote
def _remote_fn(*args):
    return user_fn(*args)

try:
    started = time.time()
    ray.init(address=ADDRESS, ignore_reinit_error=True)
    nodes = [n for n in ray.nodes() if n.get("Alive")]
    workers = max(1, len(nodes))
    if workers == 0:
        raise RuntimeError(
            "No Ray workers connected. Start at least one worker on the Cluster page."
        )
    result = ray.get(_remote_fn.remote(*ARGS))
    elapsed_ms = int((time.time() - started) * 1000)
    print(json.dumps({{
        "ok": True,
        "result": result,
        "executionTimeMs": elapsed_ms,
        "workersUsed": workers,
    }}, default=str))
    ray.shutdown()
except Exception as exc:
    print(json.dumps({{"ok": False, "error": str(exc)}}))
    sys.exit(1)
"#,
        head_address = head_address,
        args_json = args_json,
        function_body = function_body,
    )
}

fn indent_body(body: &str) -> String {
    // Indent into the `try:` block (4 spaces) — not into a nested `with` like Dask.
    body.replace('\r', "")
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("    {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

const ORCHESTRATION_PREAMBLE: &str = r#"
    nodes = [n for n in ray.nodes() if n.get("Alive")]
    workers = max(1, len(nodes))
    if workers == 0:
        raise RuntimeError(
            "No Ray workers connected. Start at least one worker on the Cluster page."
        )
"#;

pub fn orchestration_script(head_address: &str, body: &str, args_json: &str) -> String {
    let indented_body = indent_body(body);
    format!(
        r#"
import json
import time
import sys
import ray

ADDRESS = {head_address:?}
ARGS = json.loads({args_json:?})

try:
    started = time.time()
    ray.init(address=ADDRESS, ignore_reinit_error=True)
{orchestration_preamble}{indented_body}
    elapsed_ms = int((time.time() - started) * 1000)
    print(json.dumps({{
        "ok": True,
        "result": result,
        "executionTimeMs": elapsed_ms,
        "workersUsed": workers,
    }}, default=str))
    ray.shutdown()
except Exception as exc:
    print(json.dumps({{"ok": False, "error": str(exc)}}))
    sys.exit(1)
"#,
        head_address = head_address,
        args_json = args_json,
        orchestration_preamble = ORCHESTRATION_PREAMBLE,
        indented_body = indented_body,
    )
}

pub fn map_script(head_address: &str, function_body: &str, items_json: &str) -> String {
    format!(
        r#"
import json
import time
import sys
import ray

ADDRESS = {head_address:?}
ITEMS = json.loads({items_json:?})

{function_body}

@ray.remote
def _remote_fn(item):
    return user_fn(item)

try:
    started = time.time()
    ray.init(address=ADDRESS, ignore_reinit_error=True)
    nodes = [n for n in ray.nodes() if n.get("Alive")]
    workers = max(1, len(nodes))
    if workers == 0:
        raise RuntimeError(
            "No Ray workers connected. Start at least one worker on the Cluster page."
        )
    futures = [_remote_fn.remote(item) for item in ITEMS]
    result = ray.get(futures)
    elapsed_ms = int((time.time() - started) * 1000)
    print(json.dumps({{
        "ok": True,
        "result": result,
        "executionTimeMs": elapsed_ms,
        "workersUsed": workers,
    }}, default=str))
    ray.shutdown()
except Exception as exc:
    print(json.dumps({{"ok": False, "error": str(exc)}}))
    sys.exit(1)
"#,
        head_address = head_address,
        items_json = items_json,
        function_body = function_body,
    )
}
