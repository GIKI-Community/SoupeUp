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
| `cluster-runtime-server` | Headless binary |
| `cluster-runtime` (`src-tauri`) | Tauri GUI only |

### Headless on Ubuntu (LAN)

```bash
sudo apt install -y python3 python3-venv python3-pip   # required for Dask/Ray
# optional: sudo apt install -y openmpi-bin            # only if you want MPI

export CLUSTER_RUNTIME_API_ADDR=0.0.0.0:8129
export CLUSTER_RUNTIME_API_PUBLIC_URL=http://<server-ip>:8129
export RUST_LOG=info
./target/release/cluster-runtime-server
# Logs print the bearer token + curl examples.
# Token also in $CLUSTER_RUNTIME_DATA_DIR/api/endpoint.json
```
