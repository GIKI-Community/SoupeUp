use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::process::Command;
use crate::python_runtime::types::{ExecutionResult, PythonError, PythonResult};

// ─── Path Helpers ─────────────────────────────────────────────────────────────

/// Resolve the Python binary inside a venv.
pub fn venv_python_path(venv_path: &Path) -> PathBuf {
    if cfg!(windows) {
        venv_path.join("Scripts").join("python.exe")
    } else {
        venv_path.join("bin").join("python3")
    }
}

/// Resolve the pip binary inside a venv.
pub fn venv_pip_path(venv_path: &Path) -> PathBuf {
    if cfg!(windows) {
        venv_path.join("Scripts").join("pip.exe")
    } else {
        venv_path.join("bin").join("pip3")
    }
}

/// Resolve the application data directory hint (set by GUI/server bootstrap).
pub fn data_dir_hint() -> Option<PathBuf> {
    std::env::var("CLUSTER_RUNTIME_DATA_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
}

/// Base directory for all managed Python environments.
/// Prefer `{data_dir}/python/environments/`; fall back next to the executable.
pub fn environments_base_dir() -> PathBuf {
    if let Some(data) = data_dir_hint() {
        return data.join("python").join("environments");
    }
    exe_dir().join("runtime").join("python").join("environments")
}

/// Directory where a downloaded/staged Python distribution is expected.
///
/// Search order:
/// 1. `$CLUSTER_RUNTIME_PYTHON_DIR` if set
/// 2. `{data_dir}/python/` (per-machine download via setup script)
/// 3. `<exe_dir>/python/` (optional colocated install)
/// 4. Walk parents looking for `resources/python` or `src-tauri/resources/python`
pub fn bundled_python_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("CLUSTER_RUNTIME_PYTHON_DIR") {
        let p = PathBuf::from(dir);
        if p.exists() {
            return Some(p);
        }
        log::warn!(
            "CLUSTER_RUNTIME_PYTHON_DIR set but missing: {}",
            p.display()
        );
    }

    if let Some(data) = data_dir_hint() {
        let p = data.join("python");
        if p.exists() {
            return Some(p);
        }
    }

    let exe = exe_dir();

    let prod = exe.join("python");
    if prod.exists() {
        return Some(prod);
    }

    // Walk up from the binary (workspace target/, src-tauri/target/, install dir).
    let mut cur = exe.as_path();
    for _ in 0..8 {
        for candidate in [
            cur.join("resources").join("python"),
            cur.join("src-tauri").join("resources").join("python"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        cur = match cur.parent() {
            Some(p) => p,
            None => break,
        };
    }

    None
}

/// Return the directory containing the current executable.
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .expect("Cannot determine executable path")
        .parent()
        .expect("Executable has no parent directory")
        .to_path_buf()
}

// ─── Temp Scripts ─────────────────────────────────────────────────────────────

/// Generate a unique temporary `.py` file path for one-shot code execution.
pub fn temp_script_path() -> PathBuf {
    let id = uuid::Uuid::new_v4();
    std::env::temp_dir().join(format!("cluster_runtime_{}.py", id))
}

// ─── Version Parsing ──────────────────────────────────────────────────────────

/// Extract the version string from `python --version` output.
///
/// Handles both `Python 3.13.0` (stdout on modern Pythons) and
/// older versions that printed to stderr.
pub fn parse_python_version(output: &str) -> Option<String> {
    for line in output.lines() {
        let line = line.trim();
        if let Some(version) = line.strip_prefix("Python ") {
            let version = version.trim();
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
}

// ─── Process Runner ───────────────────────────────────────────────────────────

/// Run an external command, capturing stdout + stderr, returning a structured result.
///
/// This is the single, shared subprocess runner used by the execution engine,
/// pip manager, and environment manager.  No caller should shell out directly.
pub async fn run_command_captured(
    program: &Path,
    args: &[&str],
    cwd: Option<&Path>,
    env_vars: &HashMap<String, String>,
    timeout_secs: Option<u64>,
) -> PythonResult<ExecutionResult> {
    let start = Instant::now();

    let mut cmd = Command::new(program);
    cmd.args(args);

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    // Merge caller-supplied env vars on top of the inherited environment
    for (k, v) in env_vars {
        cmd.env(k, v);
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| PythonError::ExecutionError(
        format!("Failed to spawn `{}`: {}", program.display(), e)
    ))?;

    let timeout = timeout_secs.unwrap_or(60);

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| PythonError::Timeout(timeout))?
    .map_err(|e| PythonError::ExecutionError(e.to_string()))?;

    let execution_time_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    let success = output.status.success();

    // Surface tracebacks in the `exception` field when the process fails
    let exception = if !success && !stderr.is_empty() {
        Some(stderr.clone())
    } else {
        None
    };

    Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        execution_time_ms,
        return_value: None,
        exception,
        success,
    })
}
