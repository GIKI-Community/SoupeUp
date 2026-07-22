//! Discover and seed plugin manifests under `{data_dir}/plugins/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::manifest::PluginManifest;

const EMBEDDED: &[(&str, &str)] = &[
    (
        "plugin-python-runtime",
        include_str!("../../resources/plugins/plugin-python-runtime/manifest.toml"),
    ),
    (
        "plugin-dask-scheduler",
        include_str!("../../resources/plugins/plugin-dask-scheduler/manifest.toml"),
    ),
    (
        "plugin-ray",
        include_str!("../../resources/plugins/plugin-ray/manifest.toml"),
    ),
    (
        "plugin-mpi",
        include_str!("../../resources/plugins/plugin-mpi/manifest.toml"),
    ),
];

pub fn plugins_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("plugins")
}

/// Bundled catalog used for update checks (what this app build ships).
pub fn bundled_catalog() -> HashMap<String, PluginManifest> {
    let mut map = HashMap::new();
    for (id, raw) in EMBEDDED {
        match toml::from_str::<PluginManifest>(raw) {
            Ok(m) => {
                map.insert(id.to_string(), m);
            }
            Err(e) => log::error!("Bundled manifest {id} invalid: {e}"),
        }
    }
    map
}

/// Ensure each bundled plugin has a manifest under `data_dir/plugins/<id>/`.
/// Does not overwrite existing manifests (user/plugin updates stay).
pub fn seed_bundled_plugins(data_dir: &Path) -> std::io::Result<()> {
    let root = plugins_dir(data_dir);
    std::fs::create_dir_all(&root)?;
    for (id, raw) in EMBEDDED {
        let dir = root.join(id);
        let path = dir.join("manifest.toml");
        if path.exists() {
            continue;
        }
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&path, raw)?;
        log::info!("Seeded plugin manifest {}", path.display());
    }
    // Also try copying from next-to-exe resources/plugins if present (production).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let res = exe_dir.join("plugins");
            if res.is_dir() {
                copy_tree_missing(&res, &root)?;
            }
        }
    }
    Ok(())
}

fn copy_tree_missing(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            std::fs::create_dir_all(&to)?;
            copy_tree_missing(&entry.path(), &to)?;
        } else if !to.exists() {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

/// Scan `{data_dir}/plugins/*/manifest.toml`.
pub fn discover_plugins(data_dir: &Path) -> Vec<(PathBuf, PluginManifest)> {
    let root = plugins_dir(data_dir);
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return out;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let manifest_path = entry.path().join("manifest.toml");
        if !manifest_path.exists() {
            continue;
        }
        match super::PluginLoader::load_manifest(&manifest_path) {
            Ok(m) => out.push((entry.path(), m)),
            Err(e) => log::warn!(
                "Skipping invalid plugin manifest {}: {e}",
                manifest_path.display()
            ),
        }
    }
    out.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    out
}

pub fn load_manifest_at(plugin_dir: &Path) -> Result<PluginManifest, String> {
    super::PluginLoader::load_manifest(plugin_dir.join("manifest.toml"))
}
