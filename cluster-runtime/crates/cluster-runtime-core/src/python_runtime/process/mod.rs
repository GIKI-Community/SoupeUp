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

#[cfg(windows)]
mod job_object;

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
    /// Windows Job Object — closing it kills the process and descendants that
    /// did not break away from the job (Ray CLI children sometimes do).
    #[cfg(windows)]
    job: Option<job_object::JobObject>,
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
            // CREATE_NO_WINDOW | CREATE_SUSPENDED is not available via tokio easily;
            // CREATE_NO_WINDOW alone is fine — we assign to the job after spawn.
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

        #[cfg(windows)]
        let job = pid.and_then(job_object::JobObject::for_pid);

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
            #[cfg(windows)]
            job,
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

        terminate_managed(&mut proc).await;

        let info = BackgroundProcessInfo {
            id: proc.id.clone(),
            label: proc.label.clone(),
            status: proc.status.clone(),
            pid: proc.pid,
            started_at: proc.started_at,
            exit_code: proc.exit_code,
            stdout_tail: proc.stdout.read().await.clone(),
            stderr_tail: proc.stderr.read().await.clone(),
        };

        if !proc.script_path.as_os_str().is_empty() {
            let _ = tokio::fs::remove_file(&proc.script_path).await;
        }

        log::info!("Background process stopped: {} (pid={:?})", id, proc.pid);
        Ok(info)
    }

    /// Kill every tracked background process (used on app exit).
    pub async fn stop_all(&self) {
        let ids: Vec<String> = {
            let processes = self.processes.read().await;
            processes.keys().cloned().collect()
        };

        if ids.is_empty() {
            log::info!("No background Python processes to stop.");
            return;
        }

        log::info!(
            "Stopping {} background Python process(es)...",
            ids.len()
        );

        for id in ids {
            let _ = self.stop(&id).await;
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

async fn terminate_managed(proc: &mut ManagedProcess) {
    let pid = proc.pid.or_else(|| proc.child.id());

    // 1. Close the Windows Job Object first — kills the process and any
    //    descendants that remained in the job.
    #[cfg(windows)]
    {
        if let Some(job) = proc.job.take() {
            job.terminate();
        }
    }

    // 2. Tree-kill by PID (covers processes that broke away from the job,
    //    e.g. Ray CLI grandchildren). Must use /T /F — never /F alone first.
    kill_process_tree(pid);

    // 3. Ensure the tokio Child handle is cleaned up.
    let _ = proc.child.kill().await;
    let status = proc.child.try_wait().ok().flatten();
    proc.status = ProcessStatus::Stopped;
    proc.exit_code = status.and_then(|s| s.code());
}

/// Force-kill a process and its entire tree by PID.
fn kill_process_tree(pid: Option<u32>) {
    let Some(pid) = pid else {
        return;
    };

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Stdio;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        // Tree-kill FIRST. Killing the root without /T orphans children on Windows.
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    #[cfg(not(windows))]
    {
        // Kill the process group when possible.
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &format!("-{}", pid)])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let _ = std::process::Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// Last-resort cleanup for Ray daemons that escaped the process tree.
pub fn cleanup_orphaned_cluster_processes() {
    #[cfg(windows)]
    job_object::kill_orphaned_ray_processes();
}

impl Default for BackgroundProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
