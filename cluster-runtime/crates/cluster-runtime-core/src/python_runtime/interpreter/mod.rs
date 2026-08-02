use std::path::{Path, PathBuf};
use crate::python_runtime::download::{self, is_compatible_base};
use crate::python_runtime::utils::{bundled_python_dir, parse_python_version};
use crate::python_runtime::types::PythonError;

/// A discovered Python interpreter with its resolved filesystem path and version string.
#[derive(Debug, Clone)]
pub struct PythonInterpreter {
    /// Absolute path to the Python executable.
    pub path: PathBuf,
    /// Version string, e.g. `"3.10.11"`.
    pub version: String,
    /// Whether this interpreter came from a managed/downloaded distribution
    /// (as opposed to a system install).
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

        let mut cmd = tokio::process::Command::new(path);
        cmd.arg("--version")
            // Python ≤3.3 printed to stderr; newer versions use stdout
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd.output().await;

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

/// Optional colocated / leftover `resources/python` (dev machines). Not shipped.
pub async fn embedded_python() -> Option<PythonInterpreter> {
    let base = bundled_python_dir()?;

    log::info!("Looking for colocated Python in {}", base.display());

    let candidates: Vec<PathBuf> = if cfg!(windows) {
        vec![base.join("python.exe"), base.join("python3.exe")]
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
                "Colocated Python {} found at {}",
                interp.version,
                interp.path.display()
            );
            return Some(interp);
        }
    }

    log::debug!(
        "Colocated Python directory exists but no usable binary in {}",
        base.display()
    );
    None
}

/// Microsoft Store alias stubs under WindowsApps — they open the Store / flash
/// a console and are not a usable interpreter for Cluster Runtime.
fn is_windows_apps_stub(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("WindowsApps")
    })
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
                if is_windows_apps_stub(&p) {
                    log::info!(
                        "System Python: skipping WindowsApps stub {}",
                        p.display()
                    );
                    continue;
                }
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
        if is_windows_apps_stub(&path) {
            continue;
        }
        if let Some(interp) = PythonInterpreter::probe(&path, false).await {
            if !is_compatible_base(&interp.version) {
                log::warn!(
                    "System Python {} at {} is outside the supported range; skipping \
                     (will prefer download if nothing else works)",
                    interp.version,
                    interp.path.display()
                );
                continue;
            }
            log::info!(
                "System Python {} found at {}",
                interp.version,
                interp.path.display()
            );
            return Some(interp);
        }
    }

    log::warn!("No compatible system Python 3.x found on PATH");
    None
}

/// Discover or obtain a Python interpreter for this machine.
///
/// Priority:
///   1. `CLUSTER_RUNTIME_PYTHON`
///   2. Compatible system Python (packages go in CR-managed venvs)
///   3. Previously downloaded install under `{data_dir}/python/`
///   4. Optional colocated `python/` next to the binary / resources (dev only)
///   5. Download python-build-standalone into `{data_dir}/python/`
pub async fn discover_python() -> Option<PythonInterpreter> {
    match ensure_python().await {
        Ok(interp) => Some(interp),
        Err(e) => {
            log::error!("Python discovery failed: {e}");
            None
        }
    }
}

/// Like [`discover_python`], but returns a detailed error.
pub async fn ensure_python() -> Result<PythonInterpreter, PythonError> {
    if let Some(interp) = env_python().await {
        return Ok(interp);
    }

    if let Some(interp) = find_existing_python().await {
        log::info!(
            "Using system Python {} — workloads run in isolated venvs under the data dir",
            interp.version
        );
        return Ok(interp);
    }

    if let Some(interp) = download::probe_managed().await {
        log::info!(
            "Using managed Python {} at {}",
            interp.version,
            interp.path.display()
        );
        return Ok(interp);
    }

    if let Some(interp) = embedded_python().await {
        return Ok(interp);
    }

    log::info!("No local Python found — downloading a compatible standalone interpreter…");
    download::download_standalone().await
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
