use std::collections::HashMap;
use std::path::Path;
use serde::Deserialize;
use crate::python_runtime::types::{PackageInfo, PythonError, PythonResult};
use crate::python_runtime::utils::{run_command_captured, venv_python_path};

/// Wraps all pip operations for a managed virtual environment.
///
/// All commands run inside the venv's own pip to guarantee isolation — no
/// system-level pip is ever invoked.
pub struct PipManager {
    /// The package index URL (e.g. `"https://pypi.org/simple"`).
    /// This is the only pip setting callers can configure at this level;
    /// advanced proxy and auth settings belong in a future configuration layer.
    package_index: String,
}

/// JSON entry from `pip list --format=json`
#[derive(Debug, Deserialize)]
struct PipListEntry {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "version")]
    version: String,
}

impl PipManager {
    /// Create a new pip manager.
    ///
    /// `package_index` defaults to the public PyPI simple index when `None`.
    pub fn new(package_index: Option<String>) -> Self {
        Self {
            package_index: package_index
                .unwrap_or_else(|| "https://pypi.org/simple".to_string()),
        }
    }

    // ─── Public API ───────────────────────────────────────────────────────────

    /// Install a package (optionally pinned to an exact version).
    ///
    /// Uses `--index-url` so every install goes through the configured index.
    pub async fn install(
        &self,
        env_path: &Path,
        package: &str,
        version: Option<&str>,
    ) -> PythonResult<PackageInfo> {
        let spec = match version {
            Some(v) => format!("{}=={}", package, v),
            None => package.to_string(),
        };

        log::info!("pip install '{}' (env: {})", spec, env_path.display());

        let python = venv_python_path(env_path);
        let result = run_command_captured(
            &python,
            &[
                "-m", "pip", "install",
                &spec,
                "--index-url", &self.package_index,
                "--quiet",
            ],
            None,
            &HashMap::new(),
            Some(300), // generous timeout for large packages
        )
        .await?;

        if !result.success {
            return Err(PythonError::PackageError(format!(
                "Failed to install '{}': {}",
                spec, result.stderr
            )));
        }

        log::info!("Installed '{}'", spec);

        // Resolve the canonical name, version, and location via `pip show`
        self.show(env_path, package).await
    }

    /// Uninstall a package from the environment.
    pub async fn uninstall(&self, env_path: &Path, package: &str) -> PythonResult<()> {
        log::info!("pip uninstall '{}' (env: {})", package, env_path.display());

        let python = venv_python_path(env_path);
        let result = run_command_captured(
            &python,
            &["-m", "pip", "uninstall", "-y", package],
            None,
            &HashMap::new(),
            Some(60),
        )
        .await?;

        if !result.success {
            return Err(PythonError::PackageError(format!(
                "Failed to uninstall '{}': {}",
                package, result.stderr
            )));
        }

        log::info!("Uninstalled '{}'", package);
        Ok(())
    }

    /// List all packages installed in the environment.
    ///
    /// Uses `pip list --format=json` for machine-readable, stable output.
    pub async fn list(&self, env_path: &Path) -> PythonResult<Vec<PackageInfo>> {
        let python = venv_python_path(env_path);
        let result = run_command_captured(
            &python,
            &["-m", "pip", "list", "--format=json"],
            None,
            &HashMap::new(),
            Some(30),
        )
        .await?;

        if !result.success {
            return Err(PythonError::PackageError(format!(
                "Failed to list packages: {}",
                result.stderr
            )));
        }

        let entries: Vec<PipListEntry> = serde_json::from_str(result.stdout.trim())
            .map_err(|e| PythonError::JsonError(format!("pip list parse error: {}", e)))?;

        Ok(entries
            .into_iter()
            .map(|e| PackageInfo {
                name: e.name,
                version: e.version,
                location: env_path.to_string_lossy().to_string(),
            })
            .collect())
    }

    /// Return the `pip freeze` output — a requirements.txt-compatible string.
    pub async fn freeze(&self, env_path: &Path) -> PythonResult<String> {
        let python = venv_python_path(env_path);
        let result = run_command_captured(
            &python,
            &["-m", "pip", "freeze"],
            None,
            &HashMap::new(),
            Some(30),
        )
        .await?;

        if !result.success {
            return Err(PythonError::PackageError(format!(
                "Failed to freeze packages: {}",
                result.stderr
            )));
        }

        Ok(result.stdout)
    }

    /// Upgrade a package to its latest version from the configured index.
    pub async fn upgrade(&self, env_path: &Path, package: &str) -> PythonResult<PackageInfo> {
        log::info!("pip upgrade '{}' (env: {})", package, env_path.display());

        let python = venv_python_path(env_path);
        let result = run_command_captured(
            &python,
            &[
                "-m", "pip", "install", "--upgrade",
                package,
                "--index-url", &self.package_index,
                "--quiet",
            ],
            None,
            &HashMap::new(),
            Some(300),
        )
        .await?;

        if !result.success {
            return Err(PythonError::PackageError(format!(
                "Failed to upgrade '{}': {}",
                package, result.stderr
            )));
        }

        log::info!("Upgraded '{}'", package);
        self.show(env_path, package).await
    }

    // ─── Configuration ────────────────────────────────────────────────────────

    pub fn set_package_index(&mut self, index: String) {
        log::info!("Package index changed to '{}'", index);
        self.package_index = index;
    }

    pub fn package_index(&self) -> &str {
        &self.package_index
    }

    // ─── Internal ─────────────────────────────────────────────────────────────

    /// Run `pip show <package>` and parse the key-value output.
    async fn show(&self, env_path: &Path, package: &str) -> PythonResult<PackageInfo> {
        let python = venv_python_path(env_path);
        let result = run_command_captured(
            &python,
            &["-m", "pip", "show", package],
            None,
            &HashMap::new(),
            Some(10),
        )
        .await
        .unwrap_or_else(|_| crate::python_runtime::types::ExecutionResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: -1,
            execution_time_ms: 0,
            return_value: None,
            exception: None,
            success: false,
        });

        let mut name = package.to_string();
        let mut version = "unknown".to_string();
        let mut location = env_path.to_string_lossy().to_string();

        for line in result.stdout.lines() {
            if let Some(val) = line.strip_prefix("Name: ") {
                name = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("Version: ") {
                version = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("Location: ") {
                location = val.trim().to_string();
            }
        }

        Ok(PackageInfo { name, version, location })
    }
}
