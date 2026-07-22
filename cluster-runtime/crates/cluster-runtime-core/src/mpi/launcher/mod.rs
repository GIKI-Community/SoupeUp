//! Detect and invoke `mpirun` / `mpiexec`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::settings::MpiSettings;
use super::types::{MpiError, MpiFlavour, MpiLaunchResult, MpiResult, MpiToolchain};

/// Resolve the first usable MPI launcher on PATH (and well-known install dirs).
pub async fn discover_toolchain(settings: &MpiSettings) -> MpiResult<MpiToolchain> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(pref) = settings.preferred_launcher.as_deref() {
        candidates.push(pref.to_string());
    }
    for name in ["mpirun", "mpiexec"] {
        if !candidates.iter().any(|c| c == name) {
            candidates.push(name.to_string());
        }
    }
    for path in well_known_launcher_paths() {
        let s = path.to_string_lossy().into_owned();
        if !candidates.iter().any(|c| c == &s) {
            candidates.push(s);
        }
    }

    let path_hint = path_env_snippet();
    log::info!(
        "MPI: discovering toolchain ({} candidates); PATH snippet: {path_hint}",
        candidates.len()
    );

    for name in &candidates {
        log::debug!("MPI: probing launcher candidate '{name}'");
        match resolve_launcher(name).await {
            Some(path) => {
                let flavour = detect_flavour(&path).await;
                log::info!(
                    "MPI: found launcher {} ({flavour:?})",
                    path.display()
                );
                return Ok(MpiToolchain {
                    launcher: path.to_string_lossy().into_owned(),
                    flavour,
                });
            }
            None => {
                log::debug!("MPI: candidate '{name}' not found");
            }
        }
    }

    let msg = format!(
        "Neither mpirun nor mpiexec found on PATH or well-known install dirs. \
         Install OpenMPI, MPICH, or Microsoft MPI. PATH snippet: {path_hint}"
    );
    log::error!("MPI: toolchain discovery failed — {msg}");
    Err(MpiError::ToolchainNotFound(msg))
}

fn path_env_snippet() -> String {
    let path = std::env::var_os("PATH")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    if path.len() <= 240 {
        path
    } else {
        format!("{}…", &path[..240])
    }
}

fn well_known_launcher_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();

    #[cfg(windows)]
    {
        if let Ok(msmpi_bin) = std::env::var("MSMPI_BIN") {
            out.push(PathBuf::from(msmpi_bin).join("mpiexec.exe"));
        }
        for base in [
            r"C:\Program Files\Microsoft MPI\Bin",
            r"C:\Program Files (x86)\Microsoft MPI\Bin",
        ] {
            out.push(PathBuf::from(base).join("mpiexec.exe"));
        }
    }

    #[cfg(not(windows))]
    {
        for p in [
            "/usr/bin/mpirun",
            "/usr/bin/mpiexec",
            "/usr/local/bin/mpirun",
            "/usr/local/bin/mpiexec",
            "/opt/homebrew/bin/mpirun",
            "/opt/homebrew/bin/mpiexec",
        ] {
            out.push(PathBuf::from(p));
        }
    }

    out
}

async fn resolve_launcher(name_or_path: &str) -> Option<PathBuf> {
    let as_path = PathBuf::from(name_or_path);
    if as_path.is_absolute() || name_or_path.contains(['/', '\\']) {
        if file_exists(&as_path).await {
            return Some(as_path);
        }
        return None;
    }
    which(name_or_path).await
}

async fn file_exists(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.is_file())
        .unwrap_or(false)
}

async fn which(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    let output = match Command::new("where").arg(name).output().await {
        Ok(o) => o,
        Err(e) => {
            log::warn!("MPI: failed to run `where {name}`: {e}");
            return None;
        }
    };
    #[cfg(not(windows))]
    let output = match Command::new("which").arg(name).output().await {
        Ok(o) => o,
        Err(e) => {
            log::warn!("MPI: failed to run `which {name}`: {e}");
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::debug!(
            "MPI: `where`/`which` {name} exit={:?} stderr={}",
            output.status.code(),
            stderr.trim()
        );
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
            log::debug!(
                "MPI: flavour probe for {}: {}",
                path.display(),
                s.chars().take(200).collect::<String>()
            );
            s.to_lowercase()
        }
        Err(e) => {
            log::warn!(
                "MPI: could not run `{} --version`: {e}",
                path.display()
            );
            return MpiFlavour::Unknown;
        }
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
    log::info!(
        "MPI: spawning {} ranks={} executable={} flavour={:?}",
        toolchain.launcher,
        spec.ranks,
        spec.executable,
        toolchain.flavour
    );

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

    let child = cmd.spawn().map_err(|e| {
        log::error!(
            "MPI: failed to spawn launcher {}: {e}",
            toolchain.launcher
        );
        MpiError::Io(e)
    })?;

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
    let status = child.wait().await.map_err(|e| {
        log::error!("MPI: wait failed: {e}");
        MpiError::Io(e)
    })?;
    let elapsed = proc.started.elapsed().as_millis() as u64;
    let result = MpiLaunchResult {
        success: status.success(),
        exit_code: status.code(),
        stdout: String::from_utf8_lossy(&out_bytes).into_owned(),
        stderr: String::from_utf8_lossy(&err_bytes).into_owned(),
        execution_time_ms: elapsed,
        ranks: proc.ranks,
    };

    if result.success {
        log::info!(
            "MPI: job finished ok ranks={} exit={:?} {}ms",
            result.ranks,
            result.exit_code,
            result.execution_time_ms
        );
    } else {
        log::error!(
            "MPI: job failed ranks={} exit={:?} {}ms stderr={}",
            result.ranks,
            result.exit_code,
            result.execution_time_ms,
            result.stderr.chars().take(500).collect::<String>()
        );
    }

    Ok(result)
}

pub async fn kill(proc: &MpiProcess) -> MpiResult<()> {
    log::warn!("MPI: killing process group (ranks={})", proc.ranks);
    let mut child = proc.child.lock().await;
    if let Err(e) = child.kill().await {
        log::warn!("MPI: kill error (may already have exited): {e}");
    }
    Ok(())
}

/// Write script contents to a temp `.py` file and return its path.
pub async fn write_temp_script(contents: &str, suffix: &str) -> MpiResult<PathBuf> {
    let id = uuid::Uuid::new_v4();
    let path = std::env::temp_dir().join(format!("cluster_runtime_mpi_{id}{suffix}"));
    let mut file = tokio::fs::File::create(&path).await?;
    file.write_all(contents.as_bytes()).await?;
    file.flush().await?;
    log::debug!("MPI: wrote temp script {}", path.display());
    Ok(path)
}
