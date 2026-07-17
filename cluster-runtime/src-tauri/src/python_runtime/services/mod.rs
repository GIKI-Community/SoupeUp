use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::python_runtime::environment::EnvironmentManager;
use crate::python_runtime::execution::ExecutionEngine;
use crate::python_runtime::interpreter::PythonInterpreter;
use crate::python_runtime::pip::PipManager;
use crate::python_runtime::process::{BackgroundProcessInfo, BackgroundProcessManager};
use crate::python_runtime::types::{
    EnvironmentInfo, ExecutionContext, ExecutionResult, PackageInfo,
    PythonError, PythonResult, PythonRuntimeHealth, RuntimeStatus,
};

/// The single public API surface of the Python Runtime Plugin.
///
/// Other plugins resolve this type from the application's service registry and
/// call its methods — they need to know nothing about interpreters, venvs, or pip.
///
/// Internally it composes:
/// - `EnvironmentManager` — venv lifecycle
/// - `PipManager`         — package management
/// - `ExecutionEngine`    — subprocess-based code/script execution
///
/// This struct is `Send + Sync` and is wrapped in `Arc<RwLock<…>>` in `AppState`
/// so that all Tauri commands can share it safely.
pub struct PythonExecutionService {
    /// The interpreter that was discovered (or bundled).
    interpreter: PythonInterpreter,
    env_manager: Arc<EnvironmentManager>,
    /// PipManager is behind an RwLock because the package index can be changed
    /// at runtime without recreating the whole service.
    pip_manager: Arc<RwLock<PipManager>>,
    execution_engine: ExecutionEngine,
    background: BackgroundProcessManager,
    status: Arc<RwLock<RuntimeStatus>>,
}

impl PythonExecutionService {
    // ─── Constructor ──────────────────────────────────────────────────────────

    pub fn new(interpreter: PythonInterpreter, package_index: Option<String>) -> Self {
        Self {
            env_manager: Arc::new(EnvironmentManager::new(interpreter.clone())),
            pip_manager: Arc::new(RwLock::new(PipManager::new(package_index))),
            execution_engine: ExecutionEngine::new(),
            background: BackgroundProcessManager::new(),
            status: Arc::new(RwLock::new(RuntimeStatus::Initializing)),
            interpreter,
        }
    }

    /// Finish initialization: create the default environment, then mark Ready.
    pub async fn initialize(&self) -> PythonResult<()> {
        log::info!(
            "Initializing PythonExecutionService (interpreter: {} {})",
            self.interpreter.path.display(),
            self.interpreter.version
        );

        self.env_manager.initialize().await?;

        *self.status.write().await = RuntimeStatus::Ready;

        log::info!("PythonExecutionService is ready.");
        Ok(())
    }

    // ─── Execution ────────────────────────────────────────────────────────────

    pub async fn execute_code(
        &self,
        code: &str,
        context: Option<ExecutionContext>,
    ) -> PythonResult<ExecutionResult> {
        self.ensure_ready().await?;
        let ctx = context.unwrap_or_default();
        let env_path = self.env_manager.get_active_env_path().await;
        self.execution_engine
            .execute_code(code, &env_path, &ctx)
            .await
    }

    pub async fn execute_script(
        &self,
        script_path: &str,
        context: Option<ExecutionContext>,
    ) -> PythonResult<ExecutionResult> {
        self.ensure_ready().await?;
        let ctx = context.unwrap_or_default();
        let env_path = self.env_manager.get_active_env_path().await;
        self.execution_engine
            .execute_script(Path::new(script_path), &env_path, &ctx)
            .await
    }

    pub async fn execute_module(
        &self,
        module: &str,
        context: Option<ExecutionContext>,
    ) -> PythonResult<ExecutionResult> {
        self.ensure_ready().await?;
        let ctx = context.unwrap_or_default();
        let env_path = self.env_manager.get_active_env_path().await;
        self.execution_engine
            .execute_module(module, &env_path, &ctx)
            .await
    }

    // ─── Package Management ───────────────────────────────────────────────────

    pub async fn install_package(
        &self,
        package: &str,
        version: Option<&str>,
    ) -> PythonResult<PackageInfo> {
        self.ensure_ready().await?;
        let env_path = self.env_manager.get_active_env_path().await;
        let pip = self.pip_manager.read().await;
        pip.install(&env_path, package, version).await
    }

    pub async fn uninstall_package(&self, package: &str) -> PythonResult<()> {
        self.ensure_ready().await?;
        let env_path = self.env_manager.get_active_env_path().await;
        let pip = self.pip_manager.read().await;
        pip.uninstall(&env_path, package).await
    }

    pub async fn list_packages(&self) -> PythonResult<Vec<PackageInfo>> {
        self.ensure_ready().await?;
        let env_path = self.env_manager.get_active_env_path().await;
        let pip = self.pip_manager.read().await;
        pip.list(&env_path).await
    }

    pub async fn freeze_packages(&self) -> PythonResult<String> {
        self.ensure_ready().await?;
        let env_path = self.env_manager.get_active_env_path().await;
        let pip = self.pip_manager.read().await;
        pip.freeze(&env_path).await
    }

    pub async fn upgrade_package(&self, package: &str) -> PythonResult<PackageInfo> {
        self.ensure_ready().await?;
        let env_path = self.env_manager.get_active_env_path().await;
        let pip = self.pip_manager.read().await;
        pip.upgrade(&env_path, package).await
    }

    // ─── Environment Management ───────────────────────────────────────────────

    pub async fn create_environment(&self, name: &str) -> PythonResult<EnvironmentInfo> {
        self.env_manager.create_environment(name).await
    }

    pub async fn delete_environment(&self, name: &str) -> PythonResult<()> {
        self.env_manager.delete_environment(name).await
    }

    pub async fn activate_environment(&self, name: &str) -> PythonResult<()> {
        self.env_manager.activate_environment(name).await
    }

    pub async fn list_environments(&self) -> PythonResult<Vec<EnvironmentInfo>> {
        self.env_manager.list_environments().await
    }

    // ─── Runtime Information ──────────────────────────────────────────────────

    pub async fn python_version(&self) -> PythonResult<String> {
        Ok(self.interpreter.version.clone())
    }

    pub async fn runtime_health(&self) -> PythonResult<PythonRuntimeHealth> {
        let status = self.status.read().await.clone();
        let active_env = self.env_manager.get_active_env_name().await;
        let env_path = self.env_manager.get_active_env_path().await;

        Ok(PythonRuntimeHealth {
            status,
            python_version: Some(self.interpreter.version.clone()),
            active_environment: Some(active_env),
            environment_path: Some(env_path),
            interpreter_path: Some(self.interpreter.path.clone()),
            is_bundled: self.interpreter.is_bundled,
        })
    }

    pub async fn set_package_index(&self, index: String) {
        let mut pip = self.pip_manager.write().await;
        pip.set_package_index(index);
    }

    pub async fn package_index(&self) -> String {
        self.pip_manager.read().await.package_index().to_string()
    }

    // ─── Background Processes ─────────────────────────────────────────────────

    /// Spawn long-lived Python code (schedulers, workers, daemons).
    pub async fn spawn_code(
        &self,
        code: &str,
        label: &str,
        context: Option<ExecutionContext>,
    ) -> PythonResult<BackgroundProcessInfo> {
        self.ensure_ready().await?;
        let ctx = context.unwrap_or_default();
        let env_path = self.env_manager.get_active_env_path().await;
        self.background
            .spawn_code(code, &env_path, &ctx, label)
            .await
    }

    /// Stop a previously spawned background process.
    pub async fn stop_process(&self, id: &str) -> PythonResult<BackgroundProcessInfo> {
        self.background.stop(id).await
    }

    /// Poll status / captured logs for a background process.
    pub async fn process_status(&self, id: &str) -> PythonResult<BackgroundProcessInfo> {
        self.background.status(id).await
    }

    pub async fn list_processes(&self) -> Vec<BackgroundProcessInfo> {
        self.background.list().await
    }

    /// Stop all background Python processes (scheduler, worker, etc.).
    pub async fn shutdown(&self) {
        self.background.stop_all().await;
        crate::python_runtime::process::cleanup_orphaned_cluster_processes();
    }

    /// Run `ray stop --force` against the active venv (tears down GCS/raylet).
    pub async fn ray_stop_force(&self) {
        let env_path = self.env_manager.get_active_env_path().await;
        let python = crate::python_runtime::utils::venv_python_path(&env_path);

        #[cfg(windows)]
        let ray_cli = env_path.join("Scripts").join("ray.exe");
        #[cfg(not(windows))]
        let ray_cli = env_path.join("bin").join("ray");

        let mut cmd = if ray_cli.is_file() {
            let mut c = tokio::process::Command::new(&ray_cli);
            c.args(["stop", "--force"]);
            c
        } else {
            let mut c = tokio::process::Command::new(&python);
            c.args(["-m", "ray.scripts.scripts", "stop", "--force"]);
            c
        };

        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        log::info!("Running ray stop --force...");
        match tokio::time::timeout(
            std::time::Duration::from_secs(45),
            cmd.status(),
        )
        .await
        {
            Ok(Ok(status)) => {
                log::info!("ray stop --force finished (status={})", status);
            }
            Ok(Err(e)) => {
                log::warn!("ray stop --force failed to launch: {}", e);
            }
            Err(_) => {
                log::warn!("ray stop --force timed out after 45s");
            }
        }
    }

    // ─── Internal ─────────────────────────────────────────────────────────────

    /// Guard: ensure the service finished initializing before accepting calls.
    async fn ensure_ready(&self) -> PythonResult<()> {
        match *self.status.read().await {
            RuntimeStatus::Ready | RuntimeStatus::Degraded => Ok(()),
            RuntimeStatus::Initializing => Err(PythonError::NotInitialized),
            RuntimeStatus::Failed => Err(PythonError::ExecutionError(
                "Python runtime is in a failed state. Check logs for details.".to_string(),
            )),
        }
    }
}
