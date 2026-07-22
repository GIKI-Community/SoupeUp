use std::path::{Path, PathBuf};
use crate::python_runtime::utils::{bundled_python_dir, parse_python_version};
use crate::python_runtime::types::PythonError;

/// A discovered Python interpreter with its resolved filesystem path and version string.
#[derive(Debug, Clone)]
pub struct PythonInterpreter {
    /// Absolute path to the Python executable.
    pub path: PathBuf,
    /// Version string, e.g. `"3.10.11"`.
    pub version: String,
    /// Whether this interpreter came from the bundled distribution.
    pub is_bundled: bool,
}

impl PythonInterpreter {
    /// Probe a candidate path by running `python --version`.
    /// Returns `None` if the binary doesn't exist, isn't executable, or
    /// produces unrecognisable output.
    pub async fn probe(path: &Path, is_bundled: bool) -> Option<Self> {
        if !path.exists() {
            log::debug!("Python probe skip (missing): {}", path.display());
            return None;
        }

        let output = tokio::process::Command::new(path)
            .arg("--version")
            // Python ≤3.3 printed to stderr; newer versions use stdout
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await;

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                log::warn!("Python probe failed to exec {}: {e}", path.display());
                return None;
            }
        };

        let combined = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);

        let version = match parse_python_version(&combined) {
            Some(v) => v,
            None => {
                log::warn!(
                    "Python probe unrecognised version output from {}: {}",
                    path.display(),
                    combined.trim()
                );
                return None;
            }
        };

        // Require Python 3.x
        if !version.starts_with('3') {
            log::warn!("Rejected Python at {}: version {} is not 3.x", path.display(), version);
            return None;
        }

        Some(Self {
            path: path.to_path_buf(),
            version,
            is_bundled,
        })
    }
}

// ─── Discovery Strategies ─────────────────────────────────────────────────────

/// Explicit interpreter from `CLUSTER_RUNTIME_PYTHON` if set.
pub async fn env_python() -> Option<PythonInterpreter> {
    let raw = std::env::var("CLUSTER_RUNTIME_PYTHON").ok()?;
    let path = PathBuf::from(raw.trim());
    if path.as_os_str().is_empty() {
        return None;
    }
    match PythonInterpreter::probe(&path, false).await {
        Some(interp) => {
            log::info!(
                "CLUSTER_RUNTIME_PYTHON: {} ({})",
                interp.path.display(),
                interp.version
            );
            Some(interp)
        }
        None => {
            log::error!(
                "CLUSTER_RUNTIME_PYTHON points to unusable interpreter: {}",
                path.display()
            );
            None
        }
    }
}

/// Try to use the bundled Python 3.10 distribution shipped inside the app.
///
/// The bundled distribution should be placed at:
///   - Production: `<exe_dir>/python/` (copied by `tauri build` from `resources/python/`)
///   - Dev:        `src-tauri/resources/python/`
///
/// Run `scripts/Setup-PythonRuntime.ps1` to download and stage Python 3.10.
pub async fn embedded_python() -> Option<PythonInterpreter> {
    let base = bundled_python_dir()?;

    log::info!("Looking for bundled Python in {}", base.display());

    // python-build-standalone layout on Windows:
    //   python/python.exe  (install_only flavour)
    // On Linux/macOS:
    //   python/bin/python3
    let candidates: Vec<PathBuf> = if cfg!(windows) {
        vec![
            base.join("python.exe"),
            base.join("python3.exe"),
        ]
    } else {
        vec![
            base.join("bin").join("python3"),
            base.join("bin").join("python"),
            base.join("python3"),
            base.join("python"),
        ]
    };

    for candidate in candidates {
        if let Some(interp) = PythonInterpreter::probe(&candidate, true).await {
            log::info!(
                "Bundled Python {} found at {}",
                interp.version,
                interp.path.display()
            );
            return Some(interp);
        }
    }

    log::warn!("Bundled Python directory exists but no usable binary in {}", base.display());
    None
}

/// Search the system PATH (and well-known absolute paths) for Python 3.x.
pub async fn find_existing_python() -> Option<PythonInterpreter> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(windows)]
    {
        for name in [
            "python3.10.exe",
            "python3.11.exe",
            "python3.12.exe",
            "python3.13.exe",
            "python3.exe",
            "python.exe",
        ] {
            if let Some(p) = which(name) {
                candidates.push(p);
            }
        }
    }
    #[cfg(not(windows))]
    {
        for abs in [
            "/usr/bin/python3",
            "/usr/local/bin/python3",
            "/bin/python3",
            "/usr/bin/python3.12",
            "/usr/bin/python3.11",
            "/usr/bin/python3.10",
        ] {
            candidates.push(PathBuf::from(abs));
        }
        for name in [
            "python3.12",
            "python3.11",
            "python3.10",
            "python3.13",
            "python3",
            "python",
        ] {
            if let Some(p) = which(name) {
                candidates.push(p);
            }
            if let Some(p) = which_cmd(name).await {
                candidates.push(p);
            }
        }
    }

    // Dedup while preserving order.
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|p| seen.insert(p.clone()));

    log::info!(
        "System Python: probing {} candidate(s)",
        candidates.len()
    );

    for path in candidates {
        if let Some(interp) = PythonInterpreter::probe(&path, false).await {
            log::info!(
                "System Python {} found at {}",
                interp.version,
                interp.path.display()
            );
            return Some(interp);
        }
    }

    log::error!(
        "No system Python 3.x found. On Ubuntu: `sudo apt install -y python3 python3-venv python3-pip`. \
         Or set CLUSTER_RUNTIME_PYTHON=/path/to/python3"
    );
    None
}

/// Placeholder for a future automatic Python download capability.
pub async fn future_download(_version: &str) -> Result<PythonInterpreter, PythonError> {
    Err(PythonError::InterpreterNotFound(
        "Automatic Python download is not yet implemented. \
         Install system Python 3, set CLUSTER_RUNTIME_PYTHON, or stage a bundled distribution."
            .to_string(),
    ))
}

/// Discover the best available Python interpreter.
///
/// Priority order:
///   1. `CLUSTER_RUNTIME_PYTHON`
///   2. Bundled Python (python-build-standalone, inside the app)
///   3. System Python on PATH / well-known paths
pub async fn discover_python() -> Option<PythonInterpreter> {
    if let Some(interp) = env_python().await {
        return Some(interp);
    }

    if let Some(interp) = embedded_python().await {
        return Some(interp);
    }

    log::warn!(
        "Bundled Python not found. Falling back to system Python."
    );

    find_existing_python().await
}

// ─── Internal ─────────────────────────────────────────────────────────────────

/// Minimal `which`-style search across PATH entries.
fn which(name: &str) -> Option<PathBuf> {
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.is_file() || candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Ask the OS `which` binary (more reliable for symlinks / wrappers on Linux).
#[cfg(not(windows))]
async fn which_cmd(name: &str) -> Option<PathBuf> {
    let output = tokio::process::Command::new("which")
        .arg(name)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&output.stdout);
    let first = line.lines().next()?.trim();
    if first.is_empty() {
        return None;
    }
    Some(PathBuf::from(first))
}

#[cfg(windows)]
async fn which_cmd(_name: &str) -> Option<PathBuf> {
    None
}
