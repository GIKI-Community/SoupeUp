//! Install / uninstall plugin packages under `{data_dir}/plugins/`.

use std::path::{Path, PathBuf};

use super::discover::{load_manifest_at, plugins_dir};
use super::manifest::PluginManifest;

/// Copy a plugin package directory (must contain `manifest.toml`) into the plugins dir.
pub fn install_from_path(
    data_dir: &Path,
    source_path: &Path,
    app_version: &str,
    force: bool,
) -> Result<(PathBuf, PluginManifest), String> {
    let source = if source_path.join("manifest.toml").exists() {
        source_path.to_path_buf()
    } else if source_path.is_file()
        && source_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case("manifest.toml"))
            .unwrap_or(false)
    {
        source_path
            .parent()
            .ok_or_else(|| "manifest.toml has no parent directory".to_string())?
            .to_path_buf()
    } else {
        return Err(
            "Source must be a plugin directory containing manifest.toml".into(),
        );
    };

    let manifest = load_manifest_at(&source)?;
    if !manifest.is_compatible_with_app(app_version) {
        return Err(format!(
            "Plugin '{}' is incompatible with app v{app_version} (requires {})",
            manifest.name, manifest.app_compat
        ));
    }

    let dest = plugins_dir(data_dir).join(&manifest.id);
    if dest.exists() {
        let existing = load_manifest_at(&dest).ok();
        if existing.as_ref().map(|m| m.mandatory).unwrap_or(false) && !force {
            return Err(format!(
                "Plugin '{}' is required. Reinstalling over it requires force=true.",
                existing
                    .as_ref()
                    .map(|m| m.name.as_str())
                    .unwrap_or(manifest.id.as_str())
            ));
        }
        // Replace package contents (keep enabled.json outside).
        remove_dir_contents(&dest)?;
    } else {
        std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    }

    copy_dir_recursive(&source, &dest)?;
    Ok((dest, manifest))
}

pub fn uninstall(data_dir: &Path, id: &str, manifest: &PluginManifest) -> Result<(), String> {
    if manifest.mandatory {
        return Err(format!(
            "Plugin '{}' is required and cannot be uninstalled",
            manifest.name
        ));
    }
    let dest = plugins_dir(data_dir).join(id);
    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn remove_dir_contents(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        } else {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
