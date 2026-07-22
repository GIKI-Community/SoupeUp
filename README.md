# SoupeUp

Super Compute Cluster at GIKI.

- **Desktop GUI:** from `cluster-runtime/`, run `pnpm run dev` (Tauri + UI)
  - UI-only: `pnpm dev:ui`
- **Headless server:** `pnpm server:build` / `pnpm server:dev`
  - binary: `target/release/cluster-runtime-server` (no Tauri/GTK deps)
- **Clients:** `clients/` — VS Code extension + shared HTTP client

## Rust workspace (`cluster-runtime/`)

| Crate | Role |
|-------|------|
| `cluster-runtime-core` | Shared runtime (API, plugins, Dask/Ray/MPI) |
| `cluster-runtime-server` | Headless binary + CLI/REPL |
| `cluster-runtime` (`src-tauri`) | Tauri GUI only |

### Python (downloaded per machine — not in the binary)

```bash
# Linux
./scripts/Setup-PythonRuntime.sh --dest /var/lib/cluster-runtime/python
export CLUSTER_RUNTIME_PYTHON_DIR=/var/lib/cluster-runtime/python

# Windows (dev / GUI resources)
./scripts/Setup-PythonRuntime.ps1
```

Or install system Python with venv support (`sudo apt install python3 python3-venv python3-pip`).

### Headless on Ubuntu (LAN + REPL)

```bash
pnpm server:build   # from cluster-runtime/
./scripts/Setup-PythonRuntime.sh --dest ./data/python

./target/release/cluster-runtime-server \
  --data-dir ./data \
  --api-addr 0.0.0.0:8129 \
  --public-url http://<server-ip>:8129 \
  --python-dir ./data/python

# Interactive prompt (default):
#   cr> status
#   cr> dask start
#   cr> scheduler set dask
#   cr> peer list
#   cr> help
# Daemon (no REPL): add --no-repl
```

Flags also include `--enable-plugin` / `--disable-plugin`, `--scheduler dask|ray|mpi`, `--python`, `--node-name`, `--p2p-bootstrap`.
