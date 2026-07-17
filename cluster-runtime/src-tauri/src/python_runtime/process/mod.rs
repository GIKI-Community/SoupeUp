//! Background process management for long-lived Python services
//! (schedulers, workers, daemons). One-shot execution stays in `execution/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::python_runtime::types::{ExecutionContext, PythonError, PythonResult};
use crate::python_runtime::utils::{temp_script_path, venv_python_path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Starting,
    Running,
    Exited,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundProcessInfo {
    pub id: String,
    pub label: String,
    pub status: ProcessStatus,
    pub pid: Option<u32>,
    pub started_at: DateTime<Utc>,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

struct ManagedProcess {
    id: String,
    label: String,
    pid: Option<u32>,
    child: Child,
    script_path: PathBuf,
    started_at: DateTime<Utc>,
    status: ProcessStatus,
    exit_code: Option<i32>,
    stdout: Arc<RwLock<String>>,
    stderr: Arc<RwLock<String>>,
}

/// Tracks long-running Python child processes started through the runtime.
pub struct BackgroundProcessManager {
    processes: RwLock<HashMap<String, ManagedProcess>>,
}

impl BackgroundProcessManager {
    pub fn new() -> Self {
        Self {
            processes: RwLock::new(HashMap::new()),
        }
    }

    /// Write `code` to a temp script and spawn it as a background process.
    pub async fn spawn_code(
        &self,
        code: &str,
        env_path: &Path,
        context: &ExecutionContext,
        label: &str,
    ) -> PythonResult<BackgroundProcessInfo> {
        let script_path = temp_script_path();
        tokio::fs::write(&script_path, code).await.map_err(|e| {
            PythonError::ExecutionError(format!("Cannot write background script: {}", e))
        })?;

        self.spawn_script(&script_path, env_path, context, label, true)
            .await
    }

    /// Spawn an existing `.py` file as a background process.
    pub async fn spawn_script(
        &self,
        script_path: &Path,
        env_path: &Path,
        context: &ExecutionContext,
        label: &str,
        owns_script: bool,
    ) -> PythonResult<BackgroundProcessInfo> {
        if !script_path.exists() {
            return Err(PythonError::ExecutionError(format!(
                "Script not found: {}",
                script_path.display()
            )));
        }

        let python = venv_python_path(env_path);
        let script_str = script_path.to_str().ok_or_else(|| {
            PythonError::ExecutionError("Script path contains non-UTF8 characters".to_string())
        })?;

        let mut all_args = vec![script_str.to_string()];
        all_args.extend(context.args.clone());

        let mut cmd = Command::new(&python);
        cmd.args(&all_args);
        if let Some(cwd) = context.working_directory.as_deref() {
            cmd.current_dir(cwd);
        }
        for (k, v) in &context.env_vars {
            cmd.env(k, v);
        }
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.stdin(std::process::Stdio::null());
        cmd.kill_on_drop(true);

        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().map_err(|e| {
            PythonError::ExecutionError(format!(
                "Failed to spawn background process `{}`: {}",
                python.display(),
                e
            ))
        })?;

        let id = Uuid::new_v4().to_string();
        let pid = child.id();
        let stdout_buf = Arc::new(RwLock::new(String::new()));
        let stderr_buf = Arc::new(RwLock::new(String::new()));

        if let Some(stdout) = child.stdout.take() {
            let buf = stdout_buf.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let mut guard = buf.write().await;
                    if !guard.is_empty() {
                        guard.push('\n');
                    }
                    guard.push_str(&line);
                    if guard.len() > 64 * 1024 {
                        let trim = guard.len() - 32 * 1024;
                        *guard = guard[trim..].to_string();
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let buf = stderr_buf.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let mut guard = buf.write().await;
                    if !guard.is_empty() {
                        guard.push('\n');
                    }
                    guard.push_str(&line);
                    if guard.len() > 64 * 1024 {
                        let trim = guard.len() - 32 * 1024;
                        *guard = guard[trim..].to_string();
                    }
                }
            });
        }

        let info = BackgroundProcessInfo {
            id: id.clone(),
            label: label.to_string(),
            status: ProcessStatus::Running,
            pid,
            started_at: Utc::now(),
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: String::new(),
        };

        let managed = ManagedProcess {
            id: id.clone(),
            label: label.to_string(),
            pid,
            child,
            script_path: if owns_script {
                script_path.to_path_buf()
            } else {
                PathBuf::new()
            },
            started_at: info.started_at,
            status: ProcessStatus::Running,
            exit_code: None,
            stdout: stdout_buf,
            stderr: stderr_buf,
        };

        self.processes.write().await.insert(id, managed);

        log::info!(
            "Background process started: {} (label={}, pid={:?})",
            info.id,
            label,
            pid
        );

        Ok(info)
    }

    pub async fn stop(&self, id: &str) -> PythonResult<BackgroundProcessInfo> {
        let mut proc = self
            .processes
            .write()
            .await
            .remove(id)
            .ok_or_else(|| {
                PythonError::ExecutionError(format!("Background process not found: {}", id))
            })?;

        let pid = proc.pid;
        kill_process_tree(pid);
        let _ = proc.child.kill().await;
        let status = proc.child.try_wait().ok().flatten();
        proc.status = ProcessStatus::Stopped;
        proc.exit_code = status.and_then(|s| s.code());

        let info = BackgroundProcessInfo {
            id: proc.id.clone(),
            label: proc.label.clone(),
            status: proc.status.clone(),
            pid,
            started_at: proc.started_at,
            exit_code: proc.exit_code,
            stdout_tail: proc.stdout.read().await.clone(),
            stderr_tail: proc.stderr.read().await.clone(),
        };

        if !proc.script_path.as_os_str().is_empty() {
            let _ = tokio::fs::remove_file(&proc.script_path).await;
        }

        log::info!("Background process stopped: {} (pid={:?})", id, pid);
        Ok(info)
    }

    /// Kill every tracked background process by PID (used on app exit).
    pub async fn stop_all(&self) {
        let targets: Vec<(String, Option<u32>, PathBuf)> = {
            let processes = self.processes.read().await;
            processes
                .iter()
                .map(|(id, proc)| (id.clone(), proc.pid, proc.script_path.clone()))
                .collect()
        };

        if targets.is_empty() {
            return;
        }

        log::info!(
            "Stopping {} background Python process(es)...",
            targets.len()
        );

        for (id, pid, _) in &targets {
            kill_process_tree(*pid);
            log::info!("Killed background process {} (pid={:?})", id, pid);
        }

        let mut processes = self.processes.write().await;
        for (id, _, script_path) in targets {
            if let Some(mut proc) = processes.remove(&id) {
                proc.status = ProcessStatus::Stopped;
                let _ = proc.child.kill().await;
            }
            if !script_path.as_os_str().is_empty() {
                let _ = tokio::fs::remove_file(&script_path).await;
            }
        }
    }

    pub async fn status(&self, id: &str) -> PythonResult<BackgroundProcessInfo> {
        let mut processes = self.processes.write().await;
        let proc = processes.get_mut(id).ok_or_else(|| {
            PythonError::ExecutionError(format!("Background process not found: {}", id))
        })?;

        self.refresh_status(proc).await;
        Ok(self.to_info(proc).await)
    }

    pub async fn list(&self) -> Vec<BackgroundProcessInfo> {
        let mut processes = self.processes.write().await;
        let mut out = Vec::new();
        for proc in processes.values_mut() {
            self.refresh_status(proc).await;
            out.push(self.to_info(proc).await);
        }
        out
    }

    async fn refresh_status(&self, proc: &mut ManagedProcess) {
        if matches!(
            proc.status,
            ProcessStatus::Running | ProcessStatus::Starting
        ) {
            match proc.child.try_wait() {
                Ok(Some(status)) => {
                    proc.exit_code = status.code();
                    proc.status = if status.success() {
                        ProcessStatus::Exited
                    } else {
                        ProcessStatus::Failed
                    };
                }
                Ok(None) => {
                    proc.status = ProcessStatus::Running;
                }
                Err(_) => {
                    proc.status = ProcessStatus::Failed;
                }
            }
        }
    }

    async fn to_info(&self, proc: &ManagedProcess) -> BackgroundProcessInfo {
        BackgroundProcessInfo {
            id: proc.id.clone(),
            label: proc.label.clone(),
            status: proc.status.clone(),
            pid: proc.pid.or_else(|| proc.child.id()),
            started_at: proc.started_at,
            exit_code: proc.exit_code,
            stdout_tail: proc.stdout.read().await.clone(),
            stderr_tail: proc.stderr.read().await.clone(),
        }
    }
}

/// Force-kill a process and its children by PID.
fn kill_process_tree(pid: Option<u32>) {
    let Some(pid) = pid else {
        return;
    };

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }

    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}

impl Default for BackgroundProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
