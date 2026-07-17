# Python Runtime Plugin — Implementation Plan

## Overview

Implement the **Python Runtime Plugin** as a first-class runtime that embeds and manages Python inside the Cluster Runtime platform. It will be entirely scheduler-agnostic — no references to Dask, Ray, Celery, or any distributed computing framework. All future Python-based plugins will consume this runtime via the `PythonExecutionService`.

This milestone keeps everything inside the existing Tauri monorepo (no separate crate yet). The plugin lives in `src-tauri/src/python_runtime/` and is wired into the app as a built-in plugin that is always registered and always running. A separate `python_runtime` crate can be split off in a later milestone when dynamic loading is needed.

---

## Architecture Decision

> [!IMPORTANT]
> **Built-in plugin, not a dynamic .dll.** The existing `PluginLoader` loads `.dll` files via FFI. Implementing Python management (subprocess spawning, async I/O, environment management) as a stable FFI-safe `.dll` in safe Rust is enormously complex and fragile. Instead, the Python Runtime Plugin is implemented as a **built-in Rust module** that registers itself into the existing `PluginRegistry` at startup — identical in interface to a dynamically-loaded plugin, but without the FFI boundary. This is exactly how production runtimes (JetBrains, VS Code) handle first-party language servers. A dynamic ABI can be added later once the runtime is proven stable.

---

## Proposed Changes

### 1. Backend — `python_runtime` Module

#### [NEW] `src-tauri/src/python_runtime/mod.rs`
Top-level module wiring all sub-modules together and exposing `PythonRuntimePlugin`.

#### [NEW] `src-tauri/src/python_runtime/types/mod.rs`
Core shared types:
- `ExecutionResult { stdout, stderr, exit_code, execution_time_ms, return_value, exception, success }`
- `ExecutionContext { working_directory, env_vars, args, timeout_secs, stdin }`
- `PackageInfo { name, version, location }`
- `EnvironmentInfo { name, path, python_version, package_count, active }`
- `PythonRuntimeHealth { status, python_version, active_environment, environment_path, interpreter_path }`
- `RuntimeStatus` enum: `Initializing | Ready | Degraded | Failed`
- `PythonError` (thiserror-derived)

#### [NEW] `src-tauri/src/python_runtime/interpreter/mod.rs`
Python discovery engine:
- `find_existing_python()` — walks PATH for `python3`/`python`/`python.exe`, validates via `--version`
- `embedded_python()` — stub for future embedded CPython (returns `None` for now)
- `future_download()` — stub returning `Err("not yet implemented")`
- `PythonInterpreter { path, version, arch }` struct

#### [NEW] `src-tauri/src/python_runtime/environment/mod.rs`
`EnvironmentManager`:
- Owns `runtime/python/environments/` directory tree (relative to the Tauri app data dir)
- `create_environment(name)` — runs `python -m venv <path>`
- `delete_environment(name)` — removes directory
- `activate_environment(name)` — sets active env in state
- `list_environments()` — scans the environments directory
- `default_environment()` — creates/validates `default` env on startup
- `environment_path(name)` — resolves full path

#### [NEW] `src-tauri/src/python_runtime/pip/mod.rs`
`PipManager`:
- `install(env, package, version?)` — runs `pip install` inside the venv
- `uninstall(env, package)` — runs `pip uninstall -y`
- `list(env)` — runs `pip list --format=json`, parses output
- `freeze(env)` — runs `pip freeze`
- `upgrade(env, package)` — runs `pip install --upgrade`
- All methods are async, capture stdout/stderr

#### [NEW] `src-tauri/src/python_runtime/execution/mod.rs`
`ExecutionEngine`:
- `execute_code(code, context)` — writes code to a temp `.py` file, runs it
- `execute_script(path, context)` — runs a `.py` file directly
- `execute_module(module, context)` — runs `python -m <module>`
- `execute_file(path, context)` — alias for `execute_script`
- `execute_directory(path, context)` — runs `__main__.py` inside a package dir
- All capture stdout, stderr, exit code, timing; return `ExecutionResult`
- Timeout support via `tokio::time::timeout`

#### [NEW] `src-tauri/src/python_runtime/services/mod.rs`
`PythonExecutionService` — the public API registered into `AppState`:
```rust
pub struct PythonExecutionService { ... }
impl PythonExecutionService {
    pub async fn execute_code(...)         -> Result<ExecutionResult>
    pub async fn execute_script(...)       -> Result<ExecutionResult>
    pub async fn execute_module(...)       -> Result<ExecutionResult>
    pub async fn install_package(...)      -> Result<PackageInfo>
    pub async fn uninstall_package(...)    -> Result<()>
    pub async fn list_packages(...)        -> Result<Vec<PackageInfo>>
    pub async fn create_environment(...)   -> Result<EnvironmentInfo>
    pub async fn delete_environment(...)   -> Result<()>
    pub async fn activate_environment(...) -> Result<()>
    pub async fn python_version(...)       -> Result<String>
    pub async fn runtime_health(...)       -> Result<PythonRuntimeHealth>
}
```

#### [NEW] `src-tauri/src/python_runtime/plugin/mod.rs`
`PythonRuntimePlugin` implements the existing `PluginApi` trait:
- `metadata()` — returns name `"Python Runtime"`, type `"Runtime"`, version `"0.1.0"`
- `initialize()` — discovers Python, creates default env, registers `PythonExecutionService`
- `shutdown()` — graceful cleanup
- `health()` — returns env + interpreter health

#### [NEW] `src-tauri/src/python_runtime/utils/mod.rs`
- `temp_script_path()` — creates a temp `.py` file for code execution
- `parse_python_version(output)` — extracts semver from `python --version` output
- `venv_python_path(env_path)` — resolves `Scripts/python.exe` (Win) or `bin/python` (Unix)
- `run_command_captured(cmd, args, cwd, env, timeout)` — shared subprocess runner returning `(stdout, stderr, exit_code)`

---

### 2. `AppState` — Service Registry Extension

#### [MODIFY] [lib.rs](file:///c:/Users/Student/projects/SoupeUp/cluster-runtime/src-tauri/src/lib.rs)
Add `python_execution_service: Arc<RwLock<Option<PythonExecutionService>>>` to `AppState`. Initialize to `None`; the Python Runtime Plugin sets it during `initialize()`.

---

### 3. New Tauri Commands

#### [MODIFY] [commands/mod.rs](file:///c:/Users/Student/projects/SoupeUp/cluster-runtime/src-tauri/src/commands/mod.rs)

New commands wired to `PythonExecutionService`:

| Command | Description |
|---|---|
| `python_execute_code` | Execute a code string |
| `python_execute_script` | Execute a `.py` file path |
| `python_execute_module` | Run `python -m <module>` |
| `python_install_package` | Install a pip package |
| `python_uninstall_package` | Remove a pip package |
| `python_list_packages` | List installed packages |
| `python_create_environment` | Create a new venv |
| `python_delete_environment` | Remove a venv |
| `python_activate_environment` | Switch active environment |
| `python_runtime_health` | Get runtime health status |
| `python_version` | Get Python version string |

---

### 4. Plugin Registration

#### [MODIFY] [lib.rs](file:///c:/Users/Student/projects/SoupeUp/cluster-runtime/src-tauri/src/lib.rs)
On startup, call `PythonRuntimePlugin::initialize()` and register it in `PluginRegistry`. The Python Runtime entry will appear in the Plugins list with status `Running`.

---

### 5. Frontend — New Types

#### [MODIFY] [src/types/index.ts](file:///c:/Users/Student/projects/SoupeUp/cluster-runtime/src/types/index.ts)

```typescript
export interface ExecutionResult {
  stdout: string;
  stderr: string;
  exitCode: number;
  executionTimeMs: number;
  returnValue: string | null;
  exception: string | null;
  success: boolean;
}

export interface PackageInfo {
  name: string;
  version: string;
  location: string;
}

export interface PythonRuntimeHealth {
  status: 'ready' | 'initializing' | 'degraded' | 'failed';
  pythonVersion: string | null;
  activeEnvironment: string | null;
  environmentPath: string | null;
  interpreterPath: string | null;
}
```

---

### 6. Frontend — API Layer

#### [MODIFY] [src/api/index.ts](file:///c:/Users/Student/projects/SoupeUp/cluster-runtime/src/api/index.ts)
Add `PythonApi` namespace with all command bindings.

---

### 7. Frontend — Store

#### [NEW] `src/stores/python-runtime-store.ts`
Zustand store managing:
- `health: PythonRuntimeHealth | null`
- `packages: PackageInfo[]`
- `isExecuting: boolean`
- `lastResult: ExecutionResult | null`
- `fetchHealth()`, `fetchPackages()`, `executeCode()`, `installPackage()`, etc.

---

### 8. Frontend — Enhanced Plugins Page

#### [MODIFY] [src/pages/plugins-page.tsx](file:///c:/Users/Student/projects/SoupeUp/cluster-runtime/src/pages/plugins-page.tsx)

When the selected plugin is the Python Runtime Plugin, expand into a detail panel showing:
- **Python Version** badge
- **Environment Status** (active env name + path)
- **Installed Packages** table (name, version)
- **Execution Health** indicator (ready/degraded/failed)
- **Quick Execute** — inline code editor with run button + stdout/stderr result display

---

### 9. Frontend — Settings Page Extension

#### [MODIFY] [src/pages/settings-page.tsx](file:///c:/Users/Student/projects/SoupeUp/cluster-runtime/src/pages/settings-page.tsx)

Add a **Python Runtime** tab displaying (read-only initially):
- Interpreter path
- Active environment location
- Package cache path
- Package index URL (defaults to PyPI)

---

### 10. Tests

#### [NEW] `src-tauri/src/python_runtime/tests/`
- `test_python_discovery` — finds system Python
- `test_venv_creation` — creates and validates a venv
- `test_code_execution` — runs `print("hello")`, checks stdout
- `test_script_execution` — writes a temp script file, executes it
- `test_stdout_capture` — verifies stdout is captured correctly
- `test_stderr_capture` — runs code that writes to stderr
- `test_exception_capture` — runs code that raises, verifies `exception` field
- `test_package_list` — lists packages in default env
- `test_service_registration` — verifies `PythonExecutionService` resolves from `AppState`

---

### 11. Documentation

#### [NEW] `src-tauri/src/python_runtime/README.md`
Covers:
- Python Runtime Architecture
- Environment Lifecycle
- Package Management Flow
- Execution Flow (code → temp file → subprocess → result)
- Plugin Integration Guide
- Developer Guide (adding new execution modes)

---

## Verification Plan

### Automated Tests
```
cargo test -p cluster-runtime python_runtime
```

### Manual Verification
1. `cargo tauri dev` — app starts without panic
2. Plugins page shows **Python Runtime** with status `Running`
3. Python Runtime settings tab shows discovered interpreter path
4. Quick Execute panel: type `print("Hello World")` → run → stdout shows `Hello World`
5. Install `requests` from Plugins page → package appears in list
6. `python_runtime_health` returns `status: "ready"`

---

## Open Questions

> [!IMPORTANT]
> **Q1: Python discovery scope.** Should the runtime also support a bundled Python (e.g., `python-build-standalone` distribution placed in `resources/`) from day one, or is PATH discovery sufficient for this milestone? This affects whether we need to download a Python distribution during build.

> [!IMPORTANT]
> **Q2: Environment data directory.** The managed environments will live in Tauri's `app_data_dir()` (e.g., `%APPDATA%\cluster-runtime\runtime\python\environments\`). This directory is user-specific. Is that the right location, or should it be next to the binary for portability?

> [!NOTE]
> **Q3: Package index.** The pip manager will default to PyPI. Should it also support a local/private index from the start (just store the URL in config, pass `--index-url`)?
