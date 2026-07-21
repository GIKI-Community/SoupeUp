use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::jobs::models::{EntryPoint, JobSpec};
use crate::python_runtime::PythonExecutionService;

use super::launcher::{self, LaunchSpec, MpiProcess};
use super::settings::MpiSettings;
use super::types::{MpiError, MpiLaunchResult, MpiResult, MpiToolchain};

/// Façade for MPI toolchain detection and job launch.
pub struct MpiService {
    settings: RwLock<MpiSettings>,
    toolchain: RwLock<Option<MpiToolchain>>,
    /// Optional Python runtime for mpi4py jobs.
    python: RwLock<Option<Arc<PythonExecutionService>>>,
    running: RwLock<HashMap<String, Arc<MpiProcess>>>,
}

impl MpiService {
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(MpiSettings::default()),
            toolchain: RwLock::new(None),
            python: RwLock::new(None),
            running: RwLock::new(HashMap::new()),
        }
    }

    pub async fn initialize(&self) -> MpiResult<()> {
        let settings = self.settings.read().await.clone();
        match launcher::discover_toolchain(&settings).await {
            Ok(tc) => {
                *self.toolchain.write().await = Some(tc);
                Ok(())
            }
            Err(e) => {
                *self.toolchain.write().await = None;
                Err(e)
            }
        }
    }

    pub async fn set_python(&self, python: Option<Arc<PythonExecutionService>>) {
        *self.python.write().await = python;
    }

    pub async fn toolchain(&self) -> Option<MpiToolchain> {
        self.toolchain.read().await.clone()
    }

    pub async fn settings(&self) -> MpiSettings {
        self.settings.read().await.clone()
    }

    pub async fn update_settings(&self, settings: MpiSettings) -> MpiResult<()> {
        *self.settings.write().await = settings;
        self.initialize().await
    }

    pub async fn ensure_toolchain(&self) -> MpiResult<MpiToolchain> {
        if let Some(tc) = self.toolchain.read().await.clone() {
            return Ok(tc);
        }
        self.initialize().await?;
        self.toolchain
            .read()
            .await
            .clone()
            .ok_or_else(|| MpiError::NotReady("MPI toolchain unavailable".into()))
    }

    pub async fn is_ready(&self) -> bool {
        self.toolchain.read().await.is_some()
    }

    fn resolve_ranks(&self, spec: &JobSpec, explicit: Option<u32>, default_ranks: u32) -> u32 {
        if let Some(r) = explicit {
            return r.max(1);
        }
        if let Some(cores) = spec.resources.cpu_cores {
            let n = cores.floor() as u32;
            if n >= 1 {
                return n;
            }
        }
        default_ranks.max(1)
    }

    pub async fn run_job(&self, job_id: &str, spec: &JobSpec) -> MpiResult<MpiLaunchResult> {
        let toolchain = self.ensure_toolchain().await?;
        let settings = self.settings.read().await.clone();

        let launch = self.build_launch_spec(spec, &settings).await?;
        let proc = Arc::new(launcher::spawn(&toolchain, &settings, &launch).await?);
        self.running
            .write()
            .await
            .insert(job_id.to_string(), proc.clone());

        let result = launcher::wait_with_output(&proc).await;
        self.running.write().await.remove(job_id);
        result
    }

    async fn build_launch_spec(
        &self,
        spec: &JobSpec,
        settings: &MpiSettings,
    ) -> MpiResult<LaunchSpec> {
        let ctx = &spec.execution_context;
        let mut env_vars: Vec<(String, String)> = ctx.env_vars.clone().into_iter().collect();
        for (k, v) in &spec.env {
            env_vars.push((k.clone(), v.clone()));
        }

        match &spec.entry_point {
            EntryPoint::MpiExecutable {
                executable,
                ranks,
                hostfile,
            } => Ok(LaunchSpec {
                executable: executable.clone(),
                ranks: self.resolve_ranks(spec, *ranks, settings.default_ranks),
                hostfile: hostfile.clone(),
                working_dir: ctx.working_dir.clone(),
                env_vars,
                cli_args: ctx.cli_args.clone(),
            }),
            EntryPoint::PythonScript { script } => {
                let python = self.python_bin().await?;
                let path = launcher::write_temp_script(script, ".py").await?;
                Ok(LaunchSpec {
                    executable: python,
                    ranks: self.resolve_ranks(spec, None, settings.default_ranks),
                    hostfile: None,
                    working_dir: ctx.working_dir.clone(),
                    env_vars,
                    cli_args: {
                        let mut args = vec![path.to_string_lossy().into_owned()];
                        args.extend(ctx.cli_args.clone());
                        args
                    },
                })
            }
            EntryPoint::PythonFunction { body } => {
                let python = self.python_bin().await?;
                let wrapped = format!(
                    "def user_fn():\n{}\n\nif __name__ == '__main__':\n    user_fn()\n",
                    body.lines()
                        .map(|l| format!("    {l}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                let path = launcher::write_temp_script(&wrapped, ".py").await?;
                Ok(LaunchSpec {
                    executable: python,
                    ranks: self.resolve_ranks(spec, None, settings.default_ranks),
                    hostfile: None,
                    working_dir: ctx.working_dir.clone(),
                    env_vars,
                    cli_args: vec![path.to_string_lossy().into_owned()],
                })
            }
            other => Err(MpiError::Job(format!(
                "Unsupported entry point for MPI: {:?}",
                std::mem::discriminant(other)
            ))),
        }
    }

    async fn python_bin(&self) -> MpiResult<String> {
        let python = self.python.read().await.clone().ok_or_else(|| {
            MpiError::NotReady(
                "Python runtime not available for mpi4py jobs. Use MpiExecutable or wait for Python init."
                    .into(),
            )
        })?;
        let health = python
            .runtime_health()
            .await
            .map_err(|e| MpiError::Job(e.to_string()))?;
        if let Some(env_path) = health.environment_path {
            let bin = crate::python_runtime::utils::venv_python_path(&env_path);
            return Ok(bin.to_string_lossy().into_owned());
        }
        if let Some(path) = health.interpreter_path {
            return Ok(path.to_string_lossy().into_owned());
        }
        Err(MpiError::NotReady(
            "Could not resolve Python interpreter for mpi4py launch".into(),
        ))
    }

    pub async fn ensure_mpi4py(&self) -> MpiResult<()> {
        let python = self.python.read().await.clone().ok_or_else(|| {
            MpiError::NotReady("Python runtime required to install mpi4py".into())
        })?;
        let packages = python
            .list_packages()
            .await
            .map_err(|e| MpiError::Job(e.to_string()))?;
        if packages.iter().any(|p| p.name.eq_ignore_ascii_case("mpi4py")) {
            return Ok(());
        }
        log::info!("MPI: installing mpi4py into active Python environment");
        python
            .install_package("mpi4py", None)
            .await
            .map_err(|e| MpiError::Job(e.to_string()))?;
        Ok(())
    }

    pub async fn cancel_job(&self, job_id: &str) -> MpiResult<()> {
        if let Some(proc) = self.running.write().await.remove(job_id) {
            launcher::kill(&proc).await?;
        }
        Ok(())
    }

    pub async fn shutdown(&self) {
        let ids: Vec<String> = self.running.read().await.keys().cloned().collect();
        for id in ids {
            let _ = self.cancel_job(&id).await;
        }
    }
}

impl Default for MpiService {
    fn default() -> Self {
        Self::new()
    }
}
