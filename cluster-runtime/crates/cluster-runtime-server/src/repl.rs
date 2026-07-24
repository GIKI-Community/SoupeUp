//! Interactive REPL for headless Cluster Runtime (core ops).

use std::io::{self, BufRead, Write};

use cluster_runtime_core::logging;
use cluster_runtime_core::plugin_loader;
use cluster_runtime_core::plugin_registry::PluginStatus;
use cluster_runtime_core::AppState;

pub async fn run_repl(state: AppState) {
    println!("REPL ready. Type 'help' for commands.");
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("cr> ");
        let _ = stdout.flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_ascii_lowercase();
        match cmd.as_str() {
            "help" | "?" => print_help(),
            "quit" | "exit" | "q" => {
                println!("Shutting down…");
                break;
            }
            "status" => status(&state).await,
            "token" | "url" => show_endpoint(&state),
            "plugins" => list_plugins(&state).await,
            "plugin" => {
                if parts.len() < 3 {
                    println!("usage: plugin enable|disable <id>");
                } else {
                    plugin_cmd(&state, parts[1], parts[2]).await;
                }
            }
            "scheduler" => {
                if parts.len() == 1 {
                    let id = state.scheduler_registry.active_id().await;
                    println!("active scheduler: {id}");
                } else if parts.len() >= 3 && parts[1].eq_ignore_ascii_case("set") {
                    scheduler_set(&state, parts[2]).await;
                } else {
                    println!("usage: scheduler | scheduler set <dask|ray|mpi|plugin-id>");
                }
            }
            "dask" => {
                if parts.len() < 2 {
                    println!(
                        "usage:\n  dask status\n  dask start|stop          # local scheduler (+ local worker on start)\n  dask worker start <url>  # join remote scheduler, e.g. tcp://10.21.5.4:8786\n  dask worker stop|status"
                    );
                } else {
                    dask_cmd(&state, &parts[1..]).await;
                }
            }
            "ray" => {
                if parts.len() < 2 {
                    println!(
                        "usage:\n  ray status\n  ray start|stop\n  ray worker start <addr>  # optional remote head\n  ray worker stop|status"
                    );
                } else {
                    ray_cmd(&state, &parts[1..]).await;
                }
            }
            "peer" => {
                if parts.len() < 2 {
                    println!(
                        "usage: peer list | peer connect /ip4/<ip>/tcp/8080/ws\n  (P2P app mesh — NOT Dask. See 'help'.)"
                    );
                } else {
                    peer_cmd(&state, &parts[1..]).await;
                }
            }
            "logs" => {
                let n = parts
                    .get(1)
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(40);
                show_logs(n);
            }
            other => println!("unknown command '{other}'. Type 'help'."),
        }
    }
}

fn print_help() {
    println!(
        "Commands:
  help                         Show this help
  status                       Plugins, schedulers, python, clusters
  token / url                  Show API discovery URL + bearer token
  plugins                      List plugins
  plugin enable|disable <id>   Toggle optional plugins
  scheduler                    Show active scheduler
  scheduler set <id>           Set active (dask|ray|mpi|plugin-…)

  Dask compute cluster (scheduler/worker processes):
  dask status                  Local Dask snapshot
  dask start|stop              Start/stop LOCAL scheduler (and a local worker on start)
  dask worker start <url>      Join a REMOTE scheduler as a worker
                               example: dask worker start tcp://10.21.5.4:8786
  dask worker stop|status      Stop / show local worker process

  Ray:
  ray status|start|stop
  ray worker start [addr] | ray worker stop|status

  P2P mesh (Cluster Runtime apps talking to each other — NOT Dask):
  peer list                    Show this node's peer id + connected peers
  peer connect <multiaddr>     Dial another Cluster Runtime (port 8080)
                               example: peer connect /ip4/10.21.5.4/tcp/8080/ws
                               Wrong:  peer connect 10.21.5.4

  logs [n]                     Last n log lines (default 40)
  quit                         Stop the server

Quick pick:
  Join Windows Dask scheduler as worker  →  dask worker start tcp://WINDOWS_IP:8786
  Link two Cluster Runtime apps          →  peer connect /ip4/OTHER_IP/tcp/8080/ws"
    );
}

async fn status(state: &AppState) {
    let plugins = state.plugin_registry.read().await.list_plugins();
    let active = state.scheduler_registry.active_id().await;
    let python_ok = state.python_service.read().await.is_some();
    println!("active_scheduler={active}");
    println!("python_ready={python_ok}");
    println!("plugins:");
    for p in plugins {
        println!(
            "  {}  status={:?} enabled={} mandatory={}",
            p.id, p.status, p.enabled, p.mandatory
        );
    }

    if let Some(dask) = state.dask_service.read().await.clone() {
        match dask.cluster_snapshot().await {
            Ok(s) => println!(
                "dask: scheduler={:?} workers={}",
                s.scheduler.status,
                s.workers.len()
            ),
            Err(e) => println!("dask: error {e}"),
        }
    } else {
        println!("dask: not loaded");
    }

    if let Some(ray) = state.ray_service.read().await.clone() {
        match ray.cluster_snapshot().await {
            Ok(s) => println!(
                "ray: head={:?} workers={}",
                s.head.status,
                s.workers.len()
            ),
            Err(e) => println!("ray: error {e}"),
        }
    } else {
        println!("ray: not loaded");
    }

    if let Some(mpi) = state.mpi_service.read().await.clone() {
        println!(
            "mpi: ready={} toolchain={:?}",
            mpi.is_ready().await,
            mpi.toolchain().await
        );
    } else {
        println!("mpi: not loaded");
    }

    if let Some(p2p) = state.p2p_service.read().await.clone() {
        println!("p2p: local={}", p2p.local_peer_id());
    } else {
        println!("p2p: not started");
    }

    show_endpoint(state);
}

fn show_endpoint(state: &AppState) {
    let path = state.data_dir.join("api").join("endpoint.json");
    match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => {
                println!("api url  : {}", v.get("url").and_then(|x| x.as_str()).unwrap_or("?"));
                println!(
                    "token    : {}",
                    v.get("token").and_then(|x| x.as_str()).unwrap_or("?")
                );
                println!("file     : {}", path.display());
            }
            Err(e) => println!("endpoint parse error: {e}"),
        },
        Err(e) => println!("endpoint not ready yet ({}): {e}", path.display()),
    }
}

async fn list_plugins(state: &AppState) {
    for p in state.plugin_registry.read().await.list_plugins() {
        println!(
            "{:<28} {:<12} enabled={}  {}",
            p.id,
            format!("{:?}", p.status),
            p.enabled,
            p.name
        );
    }
}

async fn plugin_cmd(state: &AppState, action: &str, id: &str) {
    let enabled = match action.to_ascii_lowercase().as_str() {
        "enable" => true,
        "disable" => false,
        _ => {
            println!("usage: plugin enable|disable <id>");
            return;
        }
    };
    let manifest = {
        let reg = state.plugin_registry.read().await;
        reg.get_manifest(id).cloned()
    };
    let Some(manifest) = manifest else {
        println!("plugin '{id}' not found");
        return;
    };
    if let Err(e) =
        plugin_loader::enabled::set_enabled(&state.data_dir, id, enabled, &manifest)
    {
        println!("error: {e}");
        return;
    }
    if enabled {
        match cluster_runtime_core::plugin_host::factories::enable_and_start(state, id).await {
            Ok(()) => println!("enabled and started {id}"),
            Err(e) => println!("enabled in config but start failed: {e}"),
        }
    } else {
        match cluster_runtime_core::plugin_host::factories::stop_plugin(id, state).await {
            Ok(()) => println!("disabled {id}"),
            Err(e) => println!("error: {e}"),
        }
    }
    let mut reg = state.plugin_registry.write().await;
    reg.set_enabled_flag(id, enabled);
    if !enabled {
        reg.update_plugin_status(id, PluginStatus::Disabled);
    }
}

async fn scheduler_set(state: &AppState, raw: &str) {
    let id = match raw.to_ascii_lowercase().as_str() {
        "dask" => "plugin-dask-scheduler".to_string(),
        "ray" => "plugin-ray".to_string(),
        "mpi" => "plugin-mpi".to_string(),
        other => other.to_string(),
    };
    match state.scheduler_registry.set_active(&id).await {
        Ok(()) => println!("active scheduler → {id}"),
        Err(e) => println!("error: {e}"),
    }
}

async fn dask_cmd(state: &AppState, args: &[&str]) {
    let Some(dask) = state.dask_service.read().await.clone() else {
        println!("dask service not loaded (is Python/Dask plugin running?)");
        return;
    };
    let action = args[0].to_ascii_lowercase();
    match action.as_str() {
        "status" => match dask.cluster_snapshot().await {
            Ok(s) => {
                let w = dask.worker_status().await;
                println!(
                    "scheduler={:?} snapshot_workers={} cores={}",
                    s.scheduler.status,
                    s.workers.len(),
                    s.total_cores
                );
                println!(
                    "local_worker status={:?} name={} scheduler={} err={:?}",
                    w.status, w.name, w.scheduler_address, w.error
                );
            }
            Err(e) => println!("error: {e}"),
        },
        "start" => match dask.start_scheduler().await {
            Ok(info) => {
                println!("local scheduler started: {:?}", info);
                if let Err(e) = dask.start_worker(None).await {
                    println!("local worker start warning: {e}");
                } else {
                    println!("local worker started (joined local scheduler)");
                }
            }
            Err(e) => println!("error: {e}"),
        },
        "stop" => {
            let _ = dask.stop_worker().await;
            match dask.stop_scheduler().await {
                Ok(info) => println!("local scheduler/worker stopped: {:?}", info),
                Err(e) => println!("error: {e}"),
            }
        }
        "worker" => {
            if args.len() < 2 {
                println!(
                    "usage:\n  dask worker start tcp://HOST:8786\n  dask worker stop\n  dask worker status"
                );
                return;
            }
            match args[1].to_ascii_lowercase().as_str() {
                "start" => {
                    let addr = if args.len() >= 3 {
                        Some(args[2].to_string())
                    } else {
                        None
                    };
                    if addr.is_none() {
                        println!(
                            "missing scheduler address.\nexample: dask worker start tcp://10.21.5.4:8786"
                        );
                        return;
                    }
                    let addr = addr.unwrap();
                    if !addr.contains("://") {
                        println!(
                            "address looks wrong: '{addr}'\nuse full URL, e.g. tcp://10.21.5.4:8786"
                        );
                        return;
                    }
                    println!("starting Dask worker → {addr} …");
                    match dask.start_worker(Some(addr.clone())).await {
                        Ok(info) => println!(
                            "worker started: status={:?} name={} (scheduler={addr})",
                            info.status, info.name
                        ),
                        Err(e) => println!("error: {e}"),
                    }
                }
                "stop" => match dask.stop_worker().await {
                    Ok(info) => println!("worker stopped: {:?}", info.status),
                    Err(e) => println!("error: {e}"),
                },
                "status" => {
                    let w = dask.worker_status().await;
                    println!(
                        "worker status={:?} name={} scheduler={} pid={:?} err={:?}",
                        w.status, w.name, w.scheduler_address, w.process_id, w.error
                    );
                }
                _ => println!(
                    "usage: dask worker start tcp://HOST:8786 | dask worker stop | dask worker status"
                ),
            }
        }
        _ => println!(
            "usage: dask status|start|stop | dask worker start tcp://HOST:8786 | dask worker stop|status"
        ),
    }
}

async fn ray_cmd(state: &AppState, args: &[&str]) {
    let Some(ray) = state.ray_service.read().await.clone() else {
        println!("ray service not loaded");
        return;
    };
    let action = args[0].to_ascii_lowercase();
    match action.as_str() {
        "status" => match ray.cluster_snapshot().await {
            Ok(s) => println!(
                "head={:?} workers={}",
                s.head.status,
                s.workers.len()
            ),
            Err(e) => println!("error: {e}"),
        },
        "start" => match ray.start_head().await {
            Ok(info) => {
                println!("head started: {:?}", info);
                if let Err(e) = ray.start_worker(None).await {
                    println!("worker start warning: {e}");
                }
            }
            Err(e) => println!("error: {e}"),
        },
        "stop" => {
            let _ = ray.stop_worker().await;
            match ray.stop_head().await {
                Ok(info) => println!("stopped: {:?}", info),
                Err(e) => println!("error: {e}"),
            }
        }
        "worker" => {
            if args.len() < 2 {
                println!("usage: ray worker start [addr] | ray worker stop|status");
                return;
            }
            match args[1].to_ascii_lowercase().as_str() {
                "start" => {
                    let addr = args.get(2).map(|s| (*s).to_string());
                    match ray.start_worker(addr).await {
                        Ok(info) => println!("ray worker started: {:?}", info),
                        Err(e) => println!("error: {e}"),
                    }
                }
                "stop" => match ray.stop_worker().await {
                    Ok(info) => println!("ray worker stopped: {:?}", info),
                    Err(e) => println!("error: {e}"),
                },
                "status" => {
                    let w = ray.worker_status().await;
                    println!("ray worker: {:?}", w);
                }
                _ => println!("usage: ray worker start [addr] | ray worker stop|status"),
            }
        }
        _ => println!("usage: ray status|start|stop | ray worker …"),
    }
}

async fn peer_cmd(state: &AppState, args: &[&str]) {
    let Some(p2p) = state.p2p_service.read().await.clone() else {
        println!("p2p not started");
        return;
    };
    match args[0].to_ascii_lowercase().as_str() {
        "list" => match p2p.list_peers().await {
            Ok(peers) => {
                println!("local={}", p2p.local_peer_id());
                if let Ok(addrs) = p2p.listen_addrs().await {
                    println!("listen_addrs (replace 0.0.0.0 with this machine's IP when dialing from elsewhere):");
                    for a in addrs {
                        println!("  {a}");
                    }
                }
                if peers.is_empty() {
                    println!("(no connected peers)");
                }
                for p in peers {
                    println!("{:?}", p);
                }
            }
            Err(e) => println!("error: {e}"),
        },
        "connect" => {
            if args.len() < 2 {
                println!(
                    "usage: peer connect /ip4/<ip>/tcp/8080/ws\nexample: peer connect /ip4/10.21.5.4/tcp/8080/ws"
                );
                return;
            }
            let addr = args[1];
            if !addr.starts_with('/') {
                println!(
                    "invalid multiaddr '{addr}'.\n\
                     peer connect is for Cluster Runtime P2P (port 8080), not Dask.\n\
                     example: peer connect /ip4/10.21.5.4/tcp/8080/ws\n\
                     to join a Dask scheduler instead: dask worker start tcp://10.21.5.4:8786"
                );
                return;
            }
            match p2p.connect(addr).await {
                Ok(()) => println!("dialing {addr}"),
                Err(e) => println!("error: {e}"),
            }
        }
        _ => println!(
            "usage: peer list | peer connect /ip4/<ip>/tcp/8080/ws"
        ),
    }
}

fn show_logs(n: usize) {
    let logs = logging::recent_logs();
    let start = logs.len().saturating_sub(n);
    for entry in &logs[start..] {
        println!(
            "{} [{:?}] {}: {}",
            entry.timestamp.format("%H:%M:%S"),
            entry.level,
            entry.module,
            entry.message
        );
    }
}
