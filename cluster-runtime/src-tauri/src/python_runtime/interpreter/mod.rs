use std::path::{Path, PathBuf};
use crate::python_runtime::utils::{bundled_python_dir, parse_python_version};
use crate::python_runtime::types::PythonError;

/// A discovered Python interpreter with its resolved filesystem path and version string.
#[derive(Debug, Clone)]
pub struct PythonInterpreter {
    /// Absolute path to the Python executable.
    pub path: PathBuf,
    /// Version string, e.g. `"3.13.0"`.
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
            return None;
        }

        let output = tokio::process::Command::new(path)
            .arg("--version")
            // Python ≤3.3 printed to stderr; newer versions use stdout
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .ok()?;

        let combined = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);

        let version = parse_python_version(&combined)?;

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

/// Try to use the bundled Python 3.13 distribution shipped inside the app.
///
/// The bundled distribution should be placed at:
///   - Production: `<exe_dir>/python/` (copied by `tauri build` from `resources/python/`)
///   - Dev:        `src-tauri/resources/python/`
///
/// Run `scripts/Setup-PythonRuntime.ps1` to download and stage Python 3.13.
pub async fn embedded_python() -> Option<PythonInterpreter> {
    let base = bundled_python_dir()?;

    log::debug!("Looking for bundled Python in {}", base.display());

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

    log::debug!("Bundled Python not found in {}", base.display());
    None
}

/// Search the system PATH for a usable Python 3.x interpreter.
/// Prefers higher versions (3.13, 3.12, …) over lower ones.
pub async fn find_existing_python() -> Option<PythonInterpreter> {
    let candidates: &[&str] = if cfg!(windows) {
        &[
            "python3.13.exe", "python3.12.exe", "python3.11.exe",
            "python3.10.exe", "python3.exe", "python.exe",
        ]
    } else {
        &["python3.13", "python3.12", "python3.11", "python3.10", "python3", "python"]
    };

    for name in candidates {
        if let Some(path) = which(name) {
            if let Some(interp) = PythonInterpreter::probe(&path, false).await {
                log::info!(
                    "System Python {} found at {}",
                    interp.version,
                    interp.path.display()
                );
                return Some(interp);
            }
        }
    }

    None
}

/// Placeholder for a future automatic Python download capability.
///
/// This function exists so that the discovery pipeline has a clear extension
/// point.  A future milestone can implement downloading python-build-standalone
/// and slot it here without touching any callers.
pub async fn future_download(_version: &str) -> Result<PythonInterpreter, PythonError> {
    Err(PythonError::InterpreterNotFound(
        "Automatic Python download is not yet implemented. \
         Run `scripts/Setup-PythonRuntime.ps1` to stage the bundled Python distribution."
            .to_string(),
    ))
}

/// Discover the best available Python interpreter.
///
/// Priority order:
///   1. Bundled Python (python-build-standalone, inside the app)
///   2. System Python on PATH
///   3. `future_download()` — currently always fails
pub async fn discover_python() -> Option<PythonInterpreter> {
    // 1. Bundled (preferred: zero external dependencies)
    if let Some(interp) = embedded_python().await {
        return Some(interp);
    }

    log::warn!(
        "Bundled Python not found. Falling back to system Python. \
         Run `scripts/Setup-PythonRuntime.ps1` to set up the bundled distribution."
    );

    // 2. System PATH
    find_existing_python().await
}

// ─── Internal ─────────────────────────────────────────────────────────────────

/// Minimal `which`-style search across PATH entries.
fn which(name: &str) -> Option<PathBuf> {
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
