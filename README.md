# SoupeUp

Super Compute Cluster at GIKI.

- **Desktop GUI:** `cluster-runtime/` — Tauri app (`pnpm tauri dev`)
- **Headless server:** `pnpm server:build` / `pnpm server:dev` from `cluster-runtime/`
  - binary: `target/release/cluster-runtime-server` (no Tauri/GTK deps)
- **Clients:** `clients/` — VS Code extension + shared HTTP client

## Rust workspace (`cluster-runtime/`)

| Crate | Role |
|-------|------|
| `cluster-runtime-core` | Shared runtime (API, plugins, Dask/Ray/MPI) |
| `cluster-runtime-server` | Headless binary |
| `cluster-runtime` (`src-tauri`) | Tauri GUI only |
