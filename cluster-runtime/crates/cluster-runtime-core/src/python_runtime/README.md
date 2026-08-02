# Python Runtime Plugin

Embedded Python runtime for Cluster Runtime. Scheduler-agnostic — future Python
plugins consume it only through `PythonExecutionService`.

## Architecture

```
python_runtime/
├── interpreter/   # Discover bundled or PATH Python
├── environment/   # venv create / activate / delete / list
├── pip/           # install / uninstall / list / freeze / upgrade
├── execution/     # code, script, module, directory execution
├── services/      # PythonExecutionService (public API)
├── plugin/        # PluginApi registration
├── types/         # Shared DTOs + PythonError
└── utils/         # subprocess runner, path helpers
```

The plugin is a **built-in module**, not a dynamic `.dll`. On startup it is
registered in `PluginRegistry` and initialized in a background task. When ready,
`AppState.python_service` holds an `Arc<PythonExecutionService>`.

## Environment lifecycle

1. Discover interpreter (bundled `resources/python` preferred, else PATH).
2. Ensure `runtime/python/environments/default` exists (`python -m venv`).
3. Activate `default` and mark runtime `Ready`.
4. Optional: create additional named envs; switch with `activate_environment`.

Environments live next to the executable:

- Dev: `src-tauri/target/debug/runtime/python/environments/`
- Prod: `<install_dir>/runtime/python/environments/`

## Package management

All pip commands run inside the **active** venv’s Python (`-m pip`). The index
defaults to `https://pypi.org/simple` and can be changed via
`python_set_package_index`.

## Execution flow

1. Caller invokes `execute_code` / `execute_script` / `execute_module`.
2. Code strings are written to a temp `.py` file.
3. `tokio` spawns the venv Python with optional timeout.
4. Result is returned as `ExecutionResult` (stdout, stderr, exit code, timing).

## Plugin integration

Other plugins should resolve `PythonExecutionService` from `AppState` and call
its methods. Do not shell out to system Python directly.

Tauri commands mirror the service (`python_execute_code`, `python_list_packages`,
etc.) for the frontend.

## Python setup

Python is **not** bundled in the installer. On first run Cluster Runtime will:

1. Use `CLUSTER_RUNTIME_PYTHON` if set
2. Else a compatible system Python 3.x (packages go into CR-managed venvs)
3. Else reuse `{data_dir}/python/` if already downloaded
4. Else fetch [`python-runtime-manifest.json`](../../python-runtime-manifest.json)
   (override with `CLUSTER_RUNTIME_PYTHON_MANIFEST_URL`) and download the
   platform archive listed there into `{data_dir}/python/`

Update download links by editing that manifest and pushing to `main` — **no app
rebuild required**.

Optional manual staging (dev / offline):

```powershell
scripts/Setup-PythonRuntime.ps1
```

## Tests

```bash
cargo test -p cluster-runtime python_runtime
```

Tests skip/panic clearly when no interpreter is available.
