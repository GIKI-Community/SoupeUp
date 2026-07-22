use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::python_runtime::interpreter::PythonInterpreter;
use crate::python_runtime::types::{EnvironmentInfo, PythonError, PythonResult};
use crate::python_runtime::utils::{
    environments_base_dir, parse_python_version, run_command_captured, venv_python_path,
};

/// Owns and manages the lifecycle of all Python virtual environments.
///
/// Every environment lives in a subdirectory of `environments_base_dir()`,
/// which is located next to the application binary.  Consumers resolve a
/// concrete environment path through this manager rather than building paths
/// themselves.
pub struct EnvironmentManager {
    /// Root directory that contains all managed environments.
    base_dir: PathBuf,
    /// The Python interpreter used to create new environments.
    interpreter: PythonInterpreter,
    /// Name of the currently active environment (defaults to `"default"`).
    active_env: Arc<RwLock<String>>,
}

impl EnvironmentManager {
    pub fn new(interpreter: PythonInterpreter) -> Self {
        Self {
            base_dir: environments_base_dir(),
            interpreter,
            active_env: Arc::new(RwLock::new("default".to_string())),
        }
    }

    // ─── Lifecycle ────────────────────────────────────────────────────────────

    /// Ensure the environments directory and the `default` environment exist.
    /// Called once during plugin initialization.
    pub async fn initialize(&self) -> PythonResult<()> {
        tokio::fs::create_dir_all(&self.base_dir).await.map_err(|e| {
            PythonError::EnvironmentError(format!(
                "Cannot create environments directory at `{}`: {}",
                self.base_dir.display(),
                e
            ))
        })?;

        let default_path = self.base_dir.join("default");
        if default_path.exists() {
            log::info!(
                "Default Python environment already exists at {}",
                default_path.display()
            );
            // Validate the venv is usable
            let python = venv_python_path(&default_path);
            if !python.exists() {
                log::warn!(
                    "Default environment appears broken (python binary missing). Recreating."
                );
                tokio::fs::remove_dir_all(&default_path).await.ok();
                self.create_environment("default").await?;
            }
        } else {
            log::info!(
                "Creating default Python environment at {}",
                default_path.display()
            );
            self.create_environment("default").await?;
        }

        Ok(())
    }

    // ─── CRUD ─────────────────────────────────────────────────────────────────

    /// Create a new isolated virtual environment with the given name.
    pub async fn create_environment(&self, name: &str) -> PythonResult<EnvironmentInfo> {
        Self::validate_name(name)?;

        let env_path = self.base_dir.join(name);

        if env_path.exists() {
            return Err(PythonError::EnvironmentError(format!(
                "Environment '{}' already exists at {}",
                name,
                env_path.display()
            )));
        }

        log::info!(
            "Creating Python venv '{}' at {} using {}",
            name,
            env_path.display(),
            self.interpreter.path.display()
        );

        let result = run_command_captured(
            &self.interpreter.path,
            &["-m", "venv", env_path.to_str().unwrap_or(name)],
            None,
            &HashMap::new(),
            Some(120),
        )
        .await?;

        if !result.success {
            // Clean up a partially-created directory
            tokio::fs::remove_dir_all(&env_path).await.ok();
            return Err(PythonError::EnvironmentError(format!(
                "Failed to create venv '{}': {}",
                name, result.stderr
            )));
        }

        // Probe the version of Python inside the new venv
        let python_version = self.probe_venv_version(&env_path).await;

        log::info!(
            "Created environment '{}' (Python {})",
            name,
            python_version.as_deref().unwrap_or("unknown")
        );

        Ok(EnvironmentInfo {
            name: name.to_string(),
            path: env_path,
            python_version,
            package_count: 0,
            active: name == *self.active_env.read().await,
        })
    }

    /// Delete a managed environment.  The `default` environment cannot be deleted.
    pub async fn delete_environment(&self, name: &str) -> PythonResult<()> {
        if name == "default" {
            return Err(PythonError::EnvironmentError(
                "The 'default' environment cannot be deleted.".to_string(),
            ));
        }

        let env_path = self.base_dir.join(name);
        if !env_path.exists() {
            return Err(PythonError::EnvironmentError(format!(
                "Environment '{}' does not exist.",
                name
            )));
        }

        tokio::fs::remove_dir_all(&env_path).await?;

        // If this was the active env, fall back to default
        let mut active = self.active_env.write().await;
        if active.as_str() == name {
            *active = "default".to_string();
            log::info!("Deleted active environment '{}'; switched to 'default'.", name);
        } else {
            log::info!("Deleted environment '{}'.", name);
        }

        Ok(())
    }

    /// Set the named environment as the active one for subsequent executions.
    pub async fn activate_environment(&self, name: &str) -> PythonResult<()> {
        let env_path = self.base_dir.join(name);
        if !env_path.exists() {
            return Err(PythonError::EnvironmentError(format!(
                "Environment '{}' does not exist.",
                name
            )));
        }

        let python = venv_python_path(&env_path);
        if !python.exists() {
            return Err(PythonError::EnvironmentError(format!(
                "Environment '{}' appears to be corrupted (no Python binary).",
                name
            )));
        }

        *self.active_env.write().await = name.to_string();
        log::info!("Activated environment '{}'.", name);
        Ok(())
    }

    /// List all environments found under `environments_base_dir()`.
    pub async fn list_environments(&self) -> PythonResult<Vec<EnvironmentInfo>> {
        let active = self.active_env.read().await.clone();
        let mut envs = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.base_dir).await.map_err(|e| {
            PythonError::EnvironmentError(format!(
                "Cannot read environments directory `{}`: {}",
                self.base_dir.display(),
                e
            ))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(PythonError::IoError)? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if name.is_empty() {
                continue;
            }

            let python_version = self.probe_venv_version(&path).await;

            envs.push(EnvironmentInfo {
                active: name == active,
                name,
                path,
                python_version,
                package_count: 0,
            });
        }

        // Always list `default` first
        envs.sort_by(|a, b| {
            if a.name == "default" {
                std::cmp::Ordering::Less
            } else if b.name == "default" {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        Ok(envs)
    }

    // ─── Accessors ────────────────────────────────────────────────────────────

    pub async fn get_active_env_name(&self) -> String {
        self.active_env.read().await.clone()
    }

    pub async fn get_active_env_path(&self) -> PathBuf {
        let name = self.active_env.read().await.clone();
        self.base_dir.join(&name)
    }

    pub fn environment_path(&self, name: &str) -> PathBuf {
        self.base_dir.join(name)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    // ─── Internal ─────────────────────────────────────────────────────────────

    async fn probe_venv_version(&self, env_path: &Path) -> Option<String> {
        let python = venv_python_path(env_path);
        if !python.exists() {
            return None;
        }
        let result = run_command_captured(
            &python,
            &["--version"],
            None,
            &HashMap::new(),
            Some(5),
        )
        .await
        .ok()?;

        parse_python_version(&(result.stdout + &result.stderr))
    }

    fn validate_name(name: &str) -> PythonResult<()> {
        if name.is_empty() {
            return Err(PythonError::EnvironmentError(
                "Environment name must not be empty.".to_string(),
            ));
        }
        // Reject path traversal characters
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err(PythonError::EnvironmentError(format!(
                "Invalid environment name: '{}'",
                name
            )));
        }
        Ok(())
    }
}
