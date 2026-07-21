//! Detect and invoke `mpirun` / `mpiexec`.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::settings::MpiSettings;
use super::types::{MpiError, MpiFlavour, MpiLaunchResult, MpiResult, MpiToolchain};

/// Resolve the first usable MPI launcher on PATH.
pub async fn discover_toolchain(settings: &MpiSettings) -> MpiResult<MpiToolchain> {
    let mut candidates: Vec<&str> = Vec::new();
    if let Some(pref) = settings.preferred_launcher.as_deref() {
        candidates.push(pref);
    }
    for name in ["mpirun", "mpiexec"] {
        if !candidates.iter().any(|c| *c == name) {
            candidates.push(name);
        }
    }

    for name in candidates {
        if let Some(path) = which(name).await {
            let flavour = detect_flavour(&path).await;
            log::info!("MPI: found launcher {} ({:?})", path.display(), flavour);
            return Ok(MpiToolchain {
                launcher: path.to_string_lossy().into_owned(),
                flavour,
            });
        }
    }

    Err(MpiError::ToolchainNotFound(
        "Neither mpirun nor mpiexec found on PATH. Install OpenMPI, MPICH, or Microsoft MPI."
            .into(),
    ))
}

async fn which(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    let output = Command::new("where").arg(name).output().await.ok()?;
    #[cfg(not(windows))]
    let output = Command::new("which").arg(name).output().await.ok()?;

    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = stdout.lines().next()?.trim();
    if first.is_empty() {
        return None;
    }
    Some(PathBuf::from(first))
}

async fn detect_flavour(path: &std::path::Path) -> MpiFlavour {
    let output = Command::new(path).arg("--version").output().await;
    let text = match output {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            s.to_lowercase()
        }
        Err(_) => return MpiFlavour::Unknown,
    };
    if text.contains("open mpi") || text.contains("openmpi") {
        MpiFlavour::OpenMpi
    } else if text.contains("microsoft") || text.contains("ms-mpi") {
        MpiFlavour::MsMpi
    } else if text.contains("mpich") || text.contains("hydra") {
        MpiFlavour::Mpich
    } else {
        MpiFlavour::Unknown
    }
}

pub struct LaunchSpec {
    pub executable: String,
    pub ranks: u32,
    pub hostfile: Option<String>,
    pub working_dir: Option<String>,
    pub env_vars: Vec<(String, String)>,
    pub cli_args: Vec<String>,
}

/// Spawned MPI process that can be cancelled.
pub struct MpiProcess {
    pub child: Mutex<Child>,
    pub ranks: u32,
    pub started: Instant,
}

pub async fn spawn(
    toolchain: &MpiToolchain,
    settings: &MpiSettings,
    spec: &LaunchSpec,
) -> MpiResult<MpiProcess> {
    let mut cmd = Command::new(&toolchain.launcher);
    for arg in &settings.extra_launcher_args {
        cmd.arg(arg);
    }
    cmd.arg("-np").arg(spec.ranks.to_string());
    if let Some(hf) = &spec.hostfile {
        match toolchain.flavour {
            MpiFlavour::MsMpi => {
                cmd.arg("-hostfile").arg(hf);
            }
            _ => {
                cmd.arg("--hostfile").arg(hf);
            }
        }
    }
    cmd.arg(&spec.executable);
    for a in &spec.cli_args {
        cmd.arg(a);
    }

    if let Some(dir) = &spec.working_dir {
        cmd.current_dir(dir);
    }
    for (k, v) in &spec.env_vars {
        cmd.env(k, v);
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(windows)]
    {
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    let child = cmd.spawn().map_err(MpiError::Io)?;

    Ok(MpiProcess {
        child: Mutex::new(child),
        ranks: spec.ranks,
        started: Instant::now(),
    })
}

pub async fn wait_with_output(proc: &MpiProcess) -> MpiResult<MpiLaunchResult> {
    let mut child = proc.child.lock().await;
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let stdout_task = async {
        let mut buf = Vec::new();
        if let Some(ref mut s) = stdout_pipe {
            let _ = s.read_to_end(&mut buf).await;
        }
        buf
    };
    let stderr_task = async {
        let mut buf = Vec::new();
        if let Some(ref mut s) = stderr_pipe {
            let _ = s.read_to_end(&mut buf).await;
        }
        buf
    };

    let (out_bytes, err_bytes) = tokio::join!(stdout_task, stderr_task);
    let status = child.wait().await.map_err(MpiError::Io)?;
    let elapsed = proc.started.elapsed().as_millis() as u64;

    Ok(MpiLaunchResult {
        success: status.success(),
        exit_code: status.code(),
        stdout: String::from_utf8_lossy(&out_bytes).into_owned(),
        stderr: String::from_utf8_lossy(&err_bytes).into_owned(),
        execution_time_ms: elapsed,
        ranks: proc.ranks,
    })
}

pub async fn kill(proc: &MpiProcess) -> MpiResult<()> {
    let mut child = proc.child.lock().await;
    let _ = child.kill().await;
    Ok(())
}

/// Write script contents to a temp `.py` file and return its path.
pub async fn write_temp_script(contents: &str, suffix: &str) -> MpiResult<PathBuf> {
    let id = uuid::Uuid::new_v4();
    let path = std::env::temp_dir().join(format!("cluster_runtime_mpi_{id}{suffix}"));
    let mut file = tokio::fs::File::create(&path).await?;
    file.write_all(contents.as_bytes()).await?;
    file.flush().await?;
    Ok(path)
}
