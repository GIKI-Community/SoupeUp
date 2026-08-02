//! Per-machine download of python-build-standalone into the app data dir.
//!
//! Python is **not** shipped inside the app installer. On first run we:
//! 1. Prefer an explicit / system interpreter (venvs isolate packages)
//! 2. Else reuse a previously downloaded copy under `{data_dir}/python/`
//! 3. Else fetch [`python-runtime-manifest.json`] from GitHub (or
//!    `CLUSTER_RUNTIME_PYTHON_MANIFEST_URL`) and download the platform archive
//!
//! On a typical Windows install the managed interpreter lands at:
//! `%APPDATA%\dev.cluster-runtime.app\python\python.exe`
//!
//! Update download links by editing `cluster-runtime/python-runtime-manifest.json`
//! in the repo and pushing — no app rebuild required.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::python_runtime::interpreter::PythonInterpreter;
use crate::python_runtime::types::PythonError;
use crate::python_runtime::utils::{data_dir_hint, exe_dir};

/// Default raw GitHub URL for the platform → archive manifest.
/// Override with `CLUSTER_RUNTIME_PYTHON_MANIFEST_URL` (useful for forks / testing).
const DEFAULT_MANIFEST_URL: &str = concat!(
    "https://raw.githubusercontent.com/GIKI-Community/SoupeUp/",
    "main/cluster-runtime/python-runtime-manifest.json"
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PythonRuntimeManifest {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    python_version: Option<String>,
    platforms: HashMap<String, PlatformEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct PlatformEntry {
    /// Fully-qualified download URL (already %-encoded where needed).
    url: String,
    /// Local cache filename (optional; derived from the URL if omitted).
    #[serde(default)]
    archive: Option<String>,
}

/// Directory where the managed standalone interpreter lives (`…/python/`).
pub fn managed_install_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CLUSTER_RUNTIME_PYTHON_DIR") {
        let p = PathBuf::from(dir.trim());
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    if let Some(data) = data_dir_hint() {
        return data.join("python");
    }
    exe_dir().join("python")
}

fn platform_key() -> Option<&'static str> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Some("windows-x86_64");
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Some("linux-x86_64");
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Some("macos-x86_64");
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Some("macos-aarch64");
    }
    #[allow(unreachable_code)]
    None
}

fn manifest_url() -> String {
    std::env::var("CLUSTER_RUNTIME_PYTHON_MANIFEST_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_MANIFEST_URL.to_string())
}

fn archive_name_from_url(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or("python-standalone.tar.gz")
        .replace("%2B", "+")
        .replace("%2b", "+")
        .to_string()
}

fn http_client() -> Result<reqwest::Client, PythonError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent(concat!("cluster-runtime/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| PythonError::InterpreterNotFound(e.to_string()))
}

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<bytes::Bytes, PythonError> {
    client
        .get(url)
        .send()
        .await
        .map_err(|e| PythonError::InterpreterNotFound(format!("Download failed ({url}): {e}")))?
        .error_for_status()
        .map_err(|e| PythonError::InterpreterNotFound(format!("Download HTTP error: {e}")))?
        .bytes()
        .await
        .map_err(|e| PythonError::InterpreterNotFound(format!("Download body error: {e}")))
}

async fn load_manifest(client: &reqwest::Client) -> Result<(String, PlatformEntry), PythonError> {
    let Some(key) = platform_key() else {
        return Err(PythonError::InterpreterNotFound(
            "Automatic Python download is not supported on this OS/arch. \
             Install a compatible Python 3.x or set CLUSTER_RUNTIME_PYTHON."
                .into(),
        ));
    };

    let url = manifest_url();
    log::info!("Python: fetching runtime manifest from {url}");
    let body = fetch_bytes(client, &url).await?;
    let text = String::from_utf8_lossy(&body);

    let manifest: PythonRuntimeManifest = serde_json::from_str(&text).map_err(|e| {
        PythonError::InterpreterNotFound(format!(
            "Invalid python-runtime-manifest.json from {url}: {e}"
        ))
    })?;

    if manifest.schema_version > 1 {
        log::warn!(
            "Python: manifest schemaVersion={} is newer than this app understands (1); trying anyway",
            manifest.schema_version
        );
    }

    let entry = manifest.platforms.get(key).cloned().ok_or_else(|| {
        PythonError::InterpreterNotFound(format!(
            "python-runtime-manifest.json has no entry for platform '{key}'. \
             Update the manifest in the repo or set CLUSTER_RUNTIME_PYTHON."
        ))
    })?;

    if entry.url.trim().is_empty() {
        return Err(PythonError::InterpreterNotFound(format!(
            "Manifest platform '{key}' has an empty url"
        )));
    }

    let version = manifest
        .python_version
        .unwrap_or_else(|| "unknown".into());
    log::info!(
        "Python: manifest recommends {version} for {key} → {}",
        entry.url
    );
    Ok((version, entry))
}

fn python_bin_in(install_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        install_dir.join("python.exe")
    } else {
        install_dir.join("bin").join("python3")
    }
}

/// Probe an already-downloaded managed install (no network).
pub async fn probe_managed() -> Option<PythonInterpreter> {
    let dir = managed_install_dir();
    if !dir.exists() {
        return None;
    }
    let bin = python_bin_in(&dir);
    PythonInterpreter::probe(&bin, true).await
}

/// True when this interpreter is a good default for Cluster Runtime workloads.
/// Windows Ray wheels are unreliable on 3.13+; prefer 3.10–3.12.
pub fn is_compatible_base(version: &str) -> bool {
    let mut parts = version.split('.');
    let major: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    if major != 3 {
        return false;
    }
    if cfg!(windows) {
        (10..=12).contains(&minor)
    } else {
        minor >= 10
    }
}

fn extract_tar_gz(archive_path: &Path, extract_dir: &Path) -> Result<(), PythonError> {
    let file = std::fs::File::open(archive_path).map_err(|e| {
        PythonError::InterpreterNotFound(format!(
            "Cannot open Python archive {}: {e}",
            archive_path.display()
        ))
    })?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(extract_dir).map_err(|e| {
        PythonError::InterpreterNotFound(format!(
            "Failed to extract Python archive: {e}"
        ))
    })?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

fn install_extracted(extracted: &Path, dest: &Path) -> Result<(), PythonError> {
    if dest.exists() {
        let backup = dest.with_extension("old");
        let _ = std::fs::remove_dir_all(&backup);
        let _ = std::fs::rename(dest, &backup);
        let _ = std::fs::remove_dir_all(&backup);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            PythonError::InterpreterNotFound(format!(
                "Cannot create {}: {e}",
                parent.display()
            ))
        })?;
    }

    match std::fs::rename(extracted, dest) {
        Ok(()) => Ok(()),
        Err(e) => {
            log::warn!(
                "Python: rename to {} failed ({e}); copying instead",
                dest.display()
            );
            copy_dir_recursive(extracted, dest).map_err(|copy_err| {
                PythonError::InterpreterNotFound(format!(
                    "Failed to install Python into {}: rename={e}; copy={copy_err}",
                    dest.display()
                ))
            })?;
            let _ = std::fs::remove_dir_all(extracted);
            Ok(())
        }
    }
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), PythonError> {
    let tmp = path.with_extension("partial");
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| {
            PythonError::InterpreterNotFound(format!("Cannot write {}: {e}", tmp.display()))
        })?;
        f.write_all(bytes).map_err(|e| {
            PythonError::InterpreterNotFound(format!("Cannot write {}: {e}", tmp.display()))
        })?;
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        PythonError::InterpreterNotFound(format!("Cannot finalize {}: {e}", path.display()))
    })?;
    Ok(())
}

/// Download + extract python-build-standalone into [`managed_install_dir`].
pub async fn download_standalone() -> Result<PythonInterpreter, PythonError> {
    let client = http_client()?;
    let (python_version, entry) = load_manifest(&client).await?;

    let dest = managed_install_dir();
    log::info!(
        "Python: downloading standalone {python_version} into {}",
        dest.display()
    );

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            PythonError::InterpreterNotFound(format!(
                "Cannot create Python install parent {}: {e}",
                parent.display()
            ))
        })?;
    }

    let file_name = entry
        .archive
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| archive_name_from_url(&entry.url));

    let cache_dir = std::env::temp_dir().join("cluster_runtime_python_setup");
    std::fs::create_dir_all(&cache_dir)?;
    let archive_path = cache_dir.join(&file_name);

    // Cache a copy of the last successful manifest next to the install for debugging.
    if let Some(data) = data_dir_hint() {
        let _ = std::fs::create_dir_all(data.join("python"));
        // Best-effort; ignore failures.
        let _ = tokio::fs::write(
            data.join("python").join("last-manifest-url.txt"),
            format!("{}\n{}\n", manifest_url(), entry.url),
        )
        .await;
    }

    if !archive_path.exists() {
        log::info!("Python: fetching {}", entry.url);
        let bytes = fetch_bytes(&client, &entry.url).await?;
        write_bytes(&archive_path, &bytes)?;
        log::info!(
            "Python: saved archive {} ({} bytes)",
            archive_path.display(),
            bytes.len()
        );
    } else {
        log::info!("Python: using cached archive {}", archive_path.display());
    }

    let extract_dir = cache_dir.join(format!("extract-{}", std::process::id()));
    if extract_dir.exists() {
        let _ = std::fs::remove_dir_all(&extract_dir);
    }
    std::fs::create_dir_all(&extract_dir)?;

    log::info!("Python: extracting {}", archive_path.display());
    let archive_path_clone = archive_path.clone();
    let extract_dir_clone = extract_dir.clone();
    tokio::task::spawn_blocking(move || extract_tar_gz(&archive_path_clone, &extract_dir_clone))
        .await
        .map_err(|e| PythonError::InterpreterNotFound(format!("Extract task failed: {e}")))??;

    let extracted = extract_dir.join("python");
    if !extracted.exists() {
        return Err(PythonError::InterpreterNotFound(
            "Archive did not contain a top-level `python/` directory".into(),
        ));
    }

    install_extracted(&extracted, &dest)?;
    let _ = std::fs::remove_dir_all(&extract_dir);

    let bin = python_bin_in(&dest);
    PythonInterpreter::probe(&bin, true).await.ok_or_else(|| {
        PythonError::InterpreterNotFound(format!(
            "Downloaded Python but could not run {}",
            bin.display()
        ))
    })
}
