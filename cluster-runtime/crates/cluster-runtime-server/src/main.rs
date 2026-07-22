//! Headless Cluster Runtime server — CLI flags + interactive REPL.

mod repl;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use cluster_runtime_core::bootstrap::{self, resolve_data_dir};
use cluster_runtime_core::plugin_loader;
use cluster_runtime_core::AppState;

#[derive(Debug, Parser)]
#[command(
    name = "cluster-runtime-server",
    about = "Headless Cluster Runtime (API + optional interactive REPL)",
    version
)]
struct Cli {
    /// Persistent data directory (plugins, jobs, python envs, endpoint.json)
    #[arg(long, env = "CLUSTER_RUNTIME_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// HTTP API bind address (use 0.0.0.0:8129 for LAN)
    #[arg(long, env = "CLUSTER_RUNTIME_API_ADDR")]
    api_addr: Option<String>,

    /// URL clients should use (required when binding 0.0.0.0)
    #[arg(long, env = "CLUSTER_RUNTIME_API_PUBLIC_URL")]
    public_url: Option<String>,

    /// Explicit Python interpreter path
    #[arg(long, env = "CLUSTER_RUNTIME_PYTHON")]
    python: Option<PathBuf>,

    /// Directory containing a downloaded standalone Python (bin/python3)
    #[arg(long, env = "CLUSTER_RUNTIME_PYTHON_DIR")]
    python_dir: Option<PathBuf>,

    /// Display name for this node on the P2P mesh
    #[arg(long, env = "CLUSTER_RUNTIME_NODE_NAME")]
    node_name: Option<String>,

    /// Comma-separated libp2p multiaddrs to dial on startup
    #[arg(long, env = "CLUSTER_RUNTIME_P2P_BOOTSTRAP")]
    p2p_bootstrap: Option<String>,

    /// Enable plugin by id (repeatable), applied before bootstrap
    #[arg(long = "enable-plugin", value_name = "ID")]
    enable_plugin: Vec<String>,

    /// Disable plugin by id (repeatable)
    #[arg(long = "disable-plugin", value_name = "ID")]
    disable_plugin: Vec<String>,

    /// Active scheduler after plugins load: dask | ray | mpi | plugin-id
    #[arg(long)]
    scheduler: Option<String>,

    /// Daemon mode: no interactive REPL (wait for Ctrl+C)
    #[arg(long)]
    no_repl: bool,
}

fn apply_env(cli: &Cli, data_dir: &PathBuf) {
    std::env::set_var("CLUSTER_RUNTIME_DATA_DIR", data_dir.as_os_str());
    if let Some(v) = &cli.api_addr {
        std::env::set_var("CLUSTER_RUNTIME_API_ADDR", v);
    }
    if let Some(v) = &cli.public_url {
        std::env::set_var("CLUSTER_RUNTIME_API_PUBLIC_URL", v);
    }
    if let Some(v) = &cli.python {
        std::env::set_var("CLUSTER_RUNTIME_PYTHON", v.as_os_str());
    }
    if let Some(v) = &cli.python_dir {
        std::env::set_var("CLUSTER_RUNTIME_PYTHON_DIR", v.as_os_str());
    }
    if let Some(v) = &cli.node_name {
        std::env::set_var("CLUSTER_RUNTIME_NODE_NAME", v);
    }
    if let Some(v) = &cli.p2p_bootstrap {
        std::env::set_var("CLUSTER_RUNTIME_P2P_BOOTSTRAP", v);
    }
}

fn apply_plugin_toggles(data_dir: &PathBuf, cli: &Cli) -> Result<(), String> {
    if cli.enable_plugin.is_empty() && cli.disable_plugin.is_empty() {
        return Ok(());
    }
    let discovered = plugin_loader::discover::discover_plugins(data_dir);
    let manifests: std::collections::HashMap<String, _> = discovered
        .into_iter()
        .map(|(_p, m)| (m.id.clone(), m))
        .collect();

    for id in &cli.enable_plugin {
        let m = manifests
            .get(id)
            .ok_or_else(|| format!("Unknown plugin id '{id}' (seed manifests first by starting once)"))?;
        plugin_loader::enabled::set_enabled(data_dir, id, true, m)?;
        log::info!("cli: enabled plugin {id}");
    }
    for id in &cli.disable_plugin {
        let m = manifests
            .get(id)
            .ok_or_else(|| format!("Unknown plugin id '{id}'"))?;
        plugin_loader::enabled::set_enabled(data_dir, id, false, m)?;
        log::info!("cli: disabled plugin {id}");
    }
    Ok(())
}

fn resolve_scheduler_id(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "dask" => "plugin-dask-scheduler".into(),
        "ray" => "plugin-ray".into(),
        "mpi" => "plugin-mpi".into(),
        other => other.to_string(),
    }
}

#[tokio::main]
async fn main() {
    cluster_runtime_core::logging::init();

    let cli = Cli::parse();
    let data_dir = cli.data_dir.clone().unwrap_or_else(resolve_data_dir);
    apply_env(&cli, &data_dir);

    // Ensure plugins dir exists and manifests are seeded before enable/disable flags.
    if let Err(e) = plugin_loader::discover::seed_bundled_plugins(&data_dir) {
        log::warn!("plugin seed before flags failed: {e}");
    }
    if let Err(e) = apply_plugin_toggles(&data_dir, &cli) {
        log::error!("cli plugin flags: {e}");
        std::process::exit(1);
    }

    log::info!("cluster-runtime-server: data dir {}", data_dir.display());

    let state = AppState::new(data_dir.clone());
    bootstrap::start(&state).await;

    // Give plugin spawn a moment so status/REPL see something useful.
    tokio::time::sleep(Duration::from_millis(800)).await;

    if let Some(sched) = &cli.scheduler {
        let id = resolve_scheduler_id(sched);
        // Retry a few times while factories finish.
        let mut ok = false;
        for _ in 0..20 {
            match state.scheduler_registry.set_active(&id).await {
                Ok(()) => {
                    log::info!("cli: active scheduler set to {id}");
                    ok = true;
                    break;
                }
                Err(e) => {
                    log::debug!("cli: waiting for scheduler {id}: {e}");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
        if !ok {
            log::warn!("cli: could not set scheduler '{id}' (is the plugin running?)");
        }
    }

    let use_repl = !cli.no_repl;
    print_banner(&data_dir, use_repl);

    if use_repl {
        repl::run_repl(state.clone()).await;
    } else {
        log::info!("cluster-runtime-server: daemon mode. Press Ctrl+C to stop.");
        match tokio::signal::ctrl_c().await {
            Ok(()) => log::info!("cluster-runtime-server: Ctrl+C received"),
            Err(e) => log::error!("cluster-runtime-server: failed to listen for Ctrl+C: {e}"),
        }
    }

    bootstrap::shutdown_services(&state).await;
    log::info!("cluster-runtime-server: exited");
}

fn print_banner(data_dir: &PathBuf, repl: bool) {
    let endpoint = data_dir.join("api").join("endpoint.json");
    let (url, token) = read_endpoint(&endpoint);
    println!();
    println!("Cluster Runtime (headless)");
    println!("  data dir : {}", data_dir.display());
    println!("  endpoint : {}", endpoint.display());
    if let Some(u) = &url {
        println!("  api url  : {u}");
    }
    if let Some(t) = &token {
        println!("  token    : {t}");
    }
    if repl {
        println!("  type 'help' for commands, 'quit' to stop");
    }
    println!();
}

fn read_endpoint(path: &std::path::Path) -> (Option<String>, Option<String>) {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return (None, None);
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return (None, None);
    };
    (
        v.get("url").and_then(|x| x.as_str()).map(str::to_string),
        v.get("token").and_then(|x| x.as_str()).map(str::to_string),
    )
}
