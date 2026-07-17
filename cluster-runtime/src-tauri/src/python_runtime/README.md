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

## Bundled Python setup

```powershell
scripts/Setup-PythonRuntime.ps1
```

Stages python-build-standalone (Python 3.10.x) into `src-tauri/resources/python/`.
Required for Dask and Ray on Windows.

## Tests

```bash
cargo test -p cluster-runtime python_runtime
```

Tests skip/panic clearly when no interpreter is available.
