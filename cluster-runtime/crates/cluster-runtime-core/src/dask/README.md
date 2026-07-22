# Dask Scheduler Plugin

The Dask Scheduler Plugin is Cluster Runtime’s first real distributed scheduler adapter. It proves the platform can host a production scheduler **without** owning Python, package management, or inventing a new networking stack.

## Architecture

```text
Cluster Runtime
    ↓
Python Runtime Plugin          ← owns interpreters, venvs, pip, execution
    ↓
Dask Scheduler Plugin          ← adapter only
    ↓
PythonExecutionService
    ↓
distributed.Scheduler / Worker / Client   ← official Dask Python API
```

### Ownership boundaries

| Concern | Owner |
|---|---|
| UI, plugin registry, logging, settings shell | Cluster Runtime |
| Python discovery, venvs, pip, code execution, background processes | Python Runtime Plugin |
| Installing Dask packages, starting scheduler/workers, client jobs, monitoring | Dask Scheduler Plugin |
| Wire protocol / task graph / dashboard | Dask (`distributed`) |

The Dask plugin **never** shells out to `dask-scheduler` or `dask-worker`. It drives Dask through:

* `distributed.Scheduler`
* `distributed.Worker`
* `distributed.Client`

via short-lived and long-lived Python scripts executed exclusively through `PythonExecutionService`.

### Module layout

```text
src-tauri/src/dask/
  scheduler/     Scheduler lifecycle
  worker/        Worker lifecycle
  client/        ClientManager (connect, submit, map, …)
  jobs/          Job submission API + built-in examples
  dashboard/     Official dashboard URL tabs
  monitoring/    Cluster snapshot + metrics
  settings/      Scheduler / worker configuration
  services/      DaskService public surface
  scripts/       Embedded Python API scripts
  plugin/        PluginApi registration handle
  types.rs       Shared DTOs
```

---

## Scheduler lifecycle

| Action | Behavior |
|---|---|
| **Start** | Ensures packages → spawns background Python running `distributed.Scheduler` |
| **Stop** | Kills the managed background process |
| **Restart** | Stop then Start |
| **Status / Health** | Polls process state + parses `DASK_SCHEDULER_READY` stdout |

Default bind: `0.0.0.0:8786`, dashboard `:8787`.

Workers on other machines must connect to the **LAN IP**, e.g. `tcp://192.168.1.10:8786` — configure this under **Settings → Dask Scheduler → Scheduler Address**.

---

## Worker lifecycle

| Action | Behavior |
|---|---|
| **Start** | Spawns `distributed.Worker(scheduler_address, …)` in the managed Python env |
| **Stop / Restart** | Managed process stop / restart |
| **Status / Health** | Process poll + `DASK_WORKER_READY` line |

Manual scheduler address is supported from the Cluster page (“Scheduler address” field).

---

## Client API

`ClientManager` is the only path the rest of Cluster Runtime uses to talk to Dask:

```text
connect() · disconnect() · submit() · map() · scatter() · gather() · cancel() · shutdown() · cluster_info()
```

Each call runs a one-shot Python script through `PythonExecutionService.execute_code` and parses JSON from stdout.

Job service wrappers:

```text
submit_python_function() · submit_script() · submit_module()
map() · scatter() · gather() · cancel_job() · job_status()
```

---

## Cluster configuration

**Settings → Dask Scheduler**

* Scheduler Host / Port
* Dashboard Port
* Scheduler Address (what workers join)
* Worker Threads (0 = auto)
* Worker Memory Limit (e.g. `4GB`)
* Worker Name
* Local Directory
* Logging Level

Package auto-install (via Python Runtime `install_package`, not raw pip):

```text
dask · distributed · cloudpickle · msgpack · psutil · numpy
```

---

## Multi-node workflow

**Machine A (scheduler)**

1. Enable Python Runtime + Dask Scheduler plugins
2. Cluster → **Start Scheduler**
3. Note LAN IP; set Scheduler Address to `tcp://<LAN-IP>:8786` for workers

**Machines B / C (workers)**

1. Same plugins ready
2. Paste Machine A’s scheduler address
3. **Start Worker**

**Verify**

* Cluster page shows all workers
* Run **Mandelbrot Renderer**
* Compare single-node vs multi-node timing (speedup field)
* Stop workers + scheduler cleanly

---

## Dashboard

We embed Dask’s official dashboard (`http://127.0.0.1:<dashboardPort>/…`) with tabs:

* Task Stream · Worker Memory · Graph · Progress · System · Profile

If the iframe is blank (CSP / WebView limits), use **Open** (system browser via `tauri-plugin-opener`).

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| Plugin stuck Initializing / Error | No Python interpreter | Run `scripts/Setup-PythonRuntime.ps1` |
| Package install failures | Network / index | Check Settings → Python package index; retry Ensure Packages |
| Worker cannot connect | Wrong address / firewall | Use LAN IP, open port 8786, confirm scheduler Running |
| Dashboard blank | iframe blocked | Click Open |
| Jobs fail with import errors | Packages missing | Ensure Packages on Cluster page |
| Version mismatches | Mixed Dask installs | Use only the managed venv — never system pip |

---

## Developer guide

### Extending the client

1. Add a Python script helper in `dask/scripts/mod.rs`
2. Call it from `ClientManager` via `python.execute_code`
3. Return structured JSON; parse in Rust
4. Expose a Tauri command in `commands/mod.rs` if the UI needs it

### Adding another scheduler (e.g. Ray)

Mirror this adapter:

* Consume `PythonExecutionService` only
* Register as a built-in plugin with type `Scheduler`
* Keep lifecycle + client + monitoring behind a service struct
* Do not teach Cluster Runtime about Ray/Dask internals

### Background processes

Long-lived Scheduler/Worker processes use `PythonExecutionService::spawn_code` / `stop_process` (added for this milestone). One-shot jobs continue to use `execute_code`.

---

## Success criteria checklist

1. [ ] Cluster Runtime on three machines
2. [ ] Python Runtime Plugin running
3. [ ] Dask Scheduler Plugin running (packages installed)
4. [ ] Machine A: Start Scheduler
5. [ ] Machines B/C: Start Worker → join A
6. [ ] Cluster page lists all workers
7. [ ] Mandelbrot example completes
8. [ ] Work visible across machines (dashboard / worker table)
9. [ ] Progress + metrics visible
10. [ ] Single vs multi-node times compared
11. [ ] Clean shutdown of scheduler + workers
