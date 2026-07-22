//! In-process host factories keyed by plugin manifest id.

use std::sync::Arc;

use crate::dask::adapter::DaskSchedulerAdapter;
use crate::dask::DaskService;
use crate::mpi::adapter::MpiSchedulerAdapter;
use crate::mpi::MpiService;
use crate::plugin_loader::manifest::PluginManifest;
use crate::plugin_registry::PluginStatus;
use crate::python_runtime::{self, PythonExecutionService};
use crate::ray::adapter::RaySchedulerAdapter;
use crate::ray::RayService;
use crate::AppState;

pub struct PluginStartContext<'a> {
    pub state: &'a AppState,
    pub app_version: &'a str,
}

/// Start a discovered plugin by factory id. Updates registry status and AppState slots.
pub async fn start_plugin(
    manifest: &PluginManifest,
    ctx: &PluginStartContext<'_>,
) -> Result<(), String> {
    let id = manifest.factory_id();
    match id {
        "plugin-python-runtime" => start_python(ctx).await,
        "plugin-dask-scheduler" => start_dask(ctx).await,
        "plugin-ray" => start_ray(ctx).await,
        "plugin-mpi" => start_mpi(ctx).await,
        other => Err(format!("No host factory registered for '{other}'")),
    }
}

/// Stop / unregister a plugin (optional schedulers). Mandatory plugins cannot be stopped via this.
pub async fn stop_plugin(id: &str, state: &AppState) -> Result<(), String> {
    match id {
        "plugin-python-runtime" | "plugin-dask-scheduler" => Err(format!(
            "Plugin '{id}' is required and cannot be stopped"
        )),
        "plugin-ray" => {
            if let Some(ray) = state.ray_service.write().await.take() {
                ray.shutdown().await;
            }
            state.scheduler_registry.unregister("plugin-ray").await;
            let fallback = {
                let mut reg = state.plugin_registry.write().await;
                reg.update_plugin_status("plugin-ray", PluginStatus::Disabled);
                reg.set_enabled_flag("plugin-ray", false);
                reg.default_scheduler_id()
            };
            if state.scheduler_registry.active_id().await == "plugin-ray" {
                if let Some(def) = fallback {
                    let _ = state.scheduler_registry.set_active(&def).await;
                }
            }
            Ok(())
        }
        "plugin-mpi" => {
            if let Some(mpi) = state.mpi_service.write().await.take() {
                mpi.shutdown().await;
            }
            state.scheduler_registry.unregister("plugin-mpi").await;
            let fallback = {
                let mut reg = state.plugin_registry.write().await;
                reg.update_plugin_status("plugin-mpi", PluginStatus::Disabled);
                reg.set_enabled_flag("plugin-mpi", false);
                reg.default_scheduler_id()
            };
            if state.scheduler_registry.active_id().await == "plugin-mpi" {
                if let Some(def) = fallback {
                    let _ = state.scheduler_registry.set_active(&def).await;
                }
            }
            Ok(())
        }
        other => {
            // No in-process factory — mark disabled in registry only.
            state.scheduler_registry.unregister(other).await;
            let mut reg = state.plugin_registry.write().await;
            if reg.get_plugin_info(other).map(|p| p.mandatory).unwrap_or(false) {
                return Err(format!("Plugin '{other}' is required and cannot be stopped"));
            }
            reg.update_plugin_status(other, PluginStatus::Disabled);
            reg.set_enabled_flag(other, false);
            Ok(())
        }
    }
}

async fn start_python(ctx: &PluginStartContext<'_>) -> Result<(), String> {
    log::info!("plugins: starting plugin-python-runtime (factory)");
    {
        let mut reg = ctx.state.plugin_registry.write().await;
        reg.update_plugin_status("plugin-python-runtime", PluginStatus::Initializing);
    }

    let interpreter = python_runtime::interpreter::discover_python()
        .await
        .ok_or_else(|| {
            "No Python interpreter found. On Ubuntu: `sudo apt install -y python3 python3-venv python3-pip`. \
             Or set CLUSTER_RUNTIME_PYTHON=/usr/bin/python3"
                .to_string()
        })?;

    log::info!(
        "plugins: using Python {} at {} (bundled={})",
        interpreter.version,
        interpreter.path.display(),
        interpreter.is_bundled
    );

    let svc = PythonExecutionService::new(interpreter, None);
    svc.initialize().await.map_err(|e| {
        log::error!("plugins: Python initialize failed: {e}");
        e.to_string()
    })?;
    let python_arc = Arc::new(svc);
    *ctx.state.python_service.write().await = Some(python_arc.clone());

    // Attach to MPI if already running.
    if let Some(mpi) = ctx.state.mpi_service.read().await.clone() {
        mpi.set_python(Some(python_arc)).await;
    }

    let mut reg = ctx.state.plugin_registry.write().await;
    reg.update_plugin_status("plugin-python-runtime", PluginStatus::Running);
    log::info!("plugins: plugin-python-runtime Running");
    Ok(())
}

async fn start_dask(ctx: &PluginStartContext<'_>) -> Result<(), String> {
    {
        let mut reg = ctx.state.plugin_registry.write().await;
        reg.update_plugin_status("plugin-dask-scheduler", PluginStatus::Initializing);
    }

    let python = ctx
        .state
        .python_service
        .read()
        .await
        .clone()
        .ok_or_else(|| "Python runtime must be ready before starting Dask".to_string())?;

    let dask = DaskService::new(python);
    let init_ok = dask.initialize().await;
    let dask_arc = Arc::new(dask);
    *ctx.state.dask_service.write().await = Some(dask_arc.clone());
    ctx.state
        .scheduler_registry
        .register(Arc::new(DaskSchedulerAdapter::new(dask_arc)))
        .await;

    let mut reg = ctx.state.plugin_registry.write().await;
    match init_ok {
        Ok(()) => {
            reg.update_plugin_status("plugin-dask-scheduler", PluginStatus::Running);
            Ok(())
        }
        Err(e) => {
            log::error!("Dask initialization failed: {e}");
            reg.update_plugin_status("plugin-dask-scheduler", PluginStatus::Error);
            Err(e.to_string())
        }
    }
}

async fn start_ray(ctx: &PluginStartContext<'_>) -> Result<(), String> {
    {
        let mut reg = ctx.state.plugin_registry.write().await;
        reg.update_plugin_status("plugin-ray", PluginStatus::Initializing);
    }

    let python = ctx
        .state
        .python_service
        .read()
        .await
        .clone()
        .ok_or_else(|| "Python runtime must be ready before starting Ray".to_string())?;

    let ray = RayService::new(python);
    let init_ok = ray.initialize().await;
    let ray_arc = Arc::new(ray);
    *ctx.state.ray_service.write().await = Some(ray_arc.clone());
    ctx.state
        .scheduler_registry
        .register(Arc::new(RaySchedulerAdapter::new(ray_arc)))
        .await;

    let mut reg = ctx.state.plugin_registry.write().await;
    match init_ok {
        Ok(()) => {
            reg.update_plugin_status("plugin-ray", PluginStatus::Running);
            Ok(())
        }
        Err(e) => {
            log::error!("Ray initialization failed: {e}");
            reg.update_plugin_status("plugin-ray", PluginStatus::Error);
            Err(e.to_string())
        }
    }
}

async fn start_mpi(ctx: &PluginStartContext<'_>) -> Result<(), String> {
    log::info!("plugins: starting plugin-mpi (factory)");
    {
        let mut reg = ctx.state.plugin_registry.write().await;
        reg.update_plugin_status("plugin-mpi", PluginStatus::Initializing);
    }

    let mpi = Arc::new(MpiService::new());
    let init_ok = mpi.initialize().await;
    let has_python = ctx.state.python_service.read().await.is_some();
    if let Some(python) = ctx.state.python_service.read().await.clone() {
        mpi.set_python(Some(python)).await;
    }

    // Always keep the service slot so mpi_ensure_toolchain / status can retry.
    *ctx.state.mpi_service.write().await = Some(mpi.clone());

    let mut reg = ctx.state.plugin_registry.write().await;
    match init_ok {
        Ok(()) => {
            drop(reg);
            ctx.state
                .scheduler_registry
                .register(Arc::new(MpiSchedulerAdapter::new(mpi)))
                .await;
            let mut reg = ctx.state.plugin_registry.write().await;
            log::info!("plugins: plugin-mpi Running (has_python={has_python})");
            reg.update_plugin_status("plugin-mpi", PluginStatus::Running);
            Ok(())
        }
        Err(e) => {
            // Do NOT register as a scheduler while the toolchain is missing —
            // otherwise HashMap fallback can make broken MPI the active scheduler.
            log::error!(
                "plugins: plugin-mpi Error — toolchain init failed: {e} \
                 (not registered as scheduler; install openmpi-bin or mpich, has_python={has_python})"
            );
            reg.update_plugin_status("plugin-mpi", PluginStatus::Error);
            Err(e.to_string())
        }
    }
}

/// Discover manifests, upsert registry, start enabled compatible plugins.
pub async fn load_and_start_plugins(state: &AppState) {
    let app_version = env!("CARGO_PKG_VERSION");
    let data_dir = &state.data_dir;
    log::info!(
        "plugins: load_and_start begin app_version={app_version} data_dir={}",
        data_dir.display()
    );

    if let Err(e) = crate::plugin_loader::discover::seed_bundled_plugins(data_dir) {
        log::warn!("plugins: seed failed: {e}");
    } else {
        log::info!("plugins: seed complete (existing manifests preserved)");
    }

    let enabled_cfg = crate::plugin_loader::enabled::load(data_dir);
    let discovered = crate::plugin_loader::discover::discover_plugins(data_dir);
    log::info!("plugins: discovered {} package(s)", discovered.len());

    {
        let mut reg = state.plugin_registry.write().await;
        for (path, manifest) in &discovered {
            let enabled =
                crate::plugin_loader::enabled::is_enabled(&enabled_cfg, manifest);
            let compatible = manifest.is_compatible_with_app(app_version);
            let status = if !compatible {
                PluginStatus::Incompatible
            } else if !enabled {
                PluginStatus::Disabled
            } else {
                PluginStatus::Discovered
            };
            log::info!(
                "plugins: {} v{} type={} enabled={enabled} compatible={compatible} status={status:?} path={}",
                manifest.id,
                manifest.version,
                manifest.plugin_type,
                path.display()
            );
            if !compatible {
                log::error!(
                    "plugins: {} incompatible with app v{app_version} (app_compat={})",
                    manifest.id,
                    manifest.app_compat
                );
            }
            reg.upsert(
                crate::plugin_registry::PluginInfo::from_manifest(manifest, status, enabled),
                manifest.clone(),
            );
        }
    }

    let ctx = PluginStartContext {
        state,
        app_version,
    };

    // Order: runtimes first, then schedulers that don't need python (MPI), then python-dependent.
    let mut runtimes = Vec::new();
    let mut mpi_like = Vec::new();
    let mut python_schedulers = Vec::new();

    for (_path, manifest) in &discovered {
        let enabled = crate::plugin_loader::enabled::is_enabled(&enabled_cfg, manifest);
        if !enabled {
            log::info!("plugins: skip {} (disabled)", manifest.id);
            continue;
        }
        if !manifest.is_compatible_with_app(app_version) {
            log::warn!("plugins: skip {} (incompatible)", manifest.id);
            continue;
        }
        if manifest.is_runtime() {
            runtimes.push(manifest);
        } else if manifest.id == "plugin-mpi"
            || manifest.dependencies.is_empty() && manifest.is_scheduler()
        {
            mpi_like.push(manifest);
        } else {
            python_schedulers.push(manifest);
        }
    }

    log::info!(
        "plugins: start order runtimes=[{}] mpi_like=[{}] python_schedulers=[{}]",
        runtimes.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(","),
        mpi_like.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(","),
        python_schedulers.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(",")
    );

    for m in runtimes {
        log::info!("plugins: starting {}", m.id);
        if let Err(e) = start_plugin(m, &ctx).await {
            log::error!("plugins: Failed to start {}: {e}", m.id);
            let mut reg = state.plugin_registry.write().await;
            reg.update_plugin_status(&m.id, PluginStatus::Error);
        }
    }

    for m in mpi_like {
        log::info!("plugins: starting {}", m.id);
        if let Err(e) = start_plugin(m, &ctx).await {
            // MPI registers even on toolchain miss; status already Error.
            log::error!("plugins: Failed to start {}: {e}", m.id);
            let mut reg = state.plugin_registry.write().await;
            reg.update_plugin_status(&m.id, PluginStatus::Error);
        }
    }

    for m in python_schedulers {
        log::info!("plugins: starting {}", m.id);
        if let Err(e) = start_plugin(m, &ctx).await {
            log::error!("plugins: Failed to start {}: {e}", m.id);
            let mut reg = state.plugin_registry.write().await;
            reg.update_plugin_status(&m.id, PluginStatus::Error);
        }
    }

    // Ensure active scheduler is registered (persisted preference, else manifest default).
    let fallback = {
        let reg = state.plugin_registry.read().await;
        reg.default_scheduler_id()
            .unwrap_or_else(|| "plugin-dask-scheduler".into())
    };
    state
        .scheduler_registry
        .ensure_active_registered(&fallback)
        .await;

    let active = state.scheduler_registry.active_id().await;
    let mpi_ready = match state.mpi_service.read().await.as_ref() {
        Some(svc) => svc.is_ready().await,
        None => false,
    };
    let statuses: Vec<String> = {
        let reg = state.plugin_registry.read().await;
        reg.list_plugins()
            .into_iter()
            .map(|p| format!("{}={:?}", p.id, p.status))
            .collect()
    };
    log::info!(
        "plugins: load_and_start done active_scheduler={active} mpi_ready={mpi_ready} statuses=[{}]",
        statuses.join(", ")
    );
}

/// Re-evaluate enabled plugins after install / enable (start one plugin).
pub async fn enable_and_start(state: &AppState, id: &str) -> Result<(), String> {
    let app_version = env!("CARGO_PKG_VERSION");
    let manifest = {
        let reg = state.plugin_registry.read().await;
        if let Some(info) = reg.get_plugin_info(id) {
            if info.status == PluginStatus::Running && info.enabled {
                return Ok(());
            }
        }
        reg.get_manifest(id)
            .cloned()
            .ok_or_else(|| format!("Plugin '{id}' not found"))?
    };
    if !manifest.is_compatible_with_app(app_version) {
        return Err(format!(
            "Plugin '{}' is incompatible with app v{app_version} (requires {})",
            manifest.name, manifest.app_compat
        ));
    }
    crate::plugin_loader::enabled::set_enabled(
        &state.data_dir,
        id,
        true,
        &manifest,
    )?;
    {
        let mut reg = state.plugin_registry.write().await;
        reg.set_enabled_flag(id, true);
        reg.update_plugin_status(id, PluginStatus::Initializing);
    }

    // Ensure Python if needed.
    if manifest.dependencies.iter().any(|d| d == "plugin-python-runtime")
        && state.python_service.read().await.is_none()
    {
        let py = {
            let reg = state.plugin_registry.read().await;
            reg.get_manifest("plugin-python-runtime").cloned()
        };
        if let Some(py) = py {
            let ctx = PluginStartContext {
                state,
                app_version,
            };
            start_plugin(&py, &ctx).await?;
        }
    }

    let ctx = PluginStartContext {
        state,
        app_version,
    };
    start_plugin(&manifest, &ctx).await
}
