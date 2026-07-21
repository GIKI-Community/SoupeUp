# Cluster Runtime Clients

This workspace holds the client-side ecosystem for Cluster Runtime. VS Code is
the first client, but all API logic lives in a reusable library so future
clients (CLI, other IDEs, CI bots) can reuse it.

```
clients/
├─ client/            # @cluster-runtime/client — transport-agnostic TS library
└─ vscode-extension/  # the VS Code extension, built on the client
```

## Architecture

```
VS Code Extension (Node)
        │
        ▼
@cluster-runtime/client (TS)
        │  HTTP + WebSocket (loopback + bearer token)
        ▼
ApiServer (axum)
        │  hosted by either:
        │  • Tauri desktop app (GUI), or
        │  • cluster-runtime-server (headless)
        │
        ├─ JobManager / JobApi
        ├─ SchedulerRegistry → Dask / Ray / MPI adapters
        ├─ P2P (libp2p WAN mesh on :8080/ws)
        └─ EventBus → WebSocket stream
```

Either the desktop app **or** the headless server hosts the API — do not run both
against the same port/data dir at once. Clients only talk to the local HTTP API;
they never touch Dask, Ray, or MPI directly. Cross-node traffic uses libp2p
(default WebSocket listen `/ip4/0.0.0.0/tcp/8080/ws`; optional 80/443).

WAN join: set `CLUSTER_RUNTIME_P2P_BOOTSTRAP` to peer multiaddrs, and/or
`POST /v1/peers` with `{ "multiaddr": "…" }`. Override listens with
`CLUSTER_RUNTIME_P2P_LISTEN`.

## Desktop / headless API server

Both the Tauri desktop app and the headless binary (`cluster-runtime-server`)
start the same `axum` server on `127.0.0.1:8129` (override with
`CLUSTER_RUNTIME_API_ADDR`). On startup they generate a random bearer token and
write a discovery file so clients can auto-connect:

- **Windows:** `%APPDATA%\dev.cluster-runtime.app\api\endpoint.json`
- **macOS:** `~/Library/Application Support/dev.cluster-runtime.app/api/endpoint.json`
- **Linux:** `~/.local/share/dev.cluster-runtime.app/api/endpoint.json`

Override the data directory with `CLUSTER_RUNTIME_DATA_DIR` (headless binary;
desktop still uses Tauri’s app data dir).

```json
{ "url": "http://127.0.0.1:8129", "token": "…", "pid": 1234 }
```

Headless run (from `cluster-runtime/`):

```bash
pnpm server:dev      # cargo run --bin cluster-runtime-server
pnpm server:build    # release binary under src-tauri/target/release/
```

### Endpoint reference

| Method | Path | Auth | Description |
| ------ | ---- | ---- | ----------- |
| GET | `/health` | no | Liveness probe |
| GET | `/v1/system` | yes | System info + status |
| GET | `/v1/schedulers` | yes | Available schedulers |
| GET | `/v1/schedulers/active` | yes | `{ pluginId }` |
| PUT | `/v1/schedulers/active` | yes | Body `{ pluginId }` |
| GET | `/v1/cluster` | yes | Cluster overview (scheduler, workers, cores, memory) |
| GET | `/v1/nodes` | yes | Worker nodes |
| POST | `/v1/jobs` | yes | Submit a `JobSpec` (`?owner=` optional; `?targetPeer=` forwards over P2P) |
| GET | `/v1/jobs` | yes | List jobs |
| GET | `/v1/jobs/:id` | yes | Job detail (progress, logs, result) |
| GET | `/v1/jobs/:id/result` | yes | Job result |
| POST | `/v1/jobs/:id/cancel` | yes | Cancel a job |
| POST | `/v1/jobs/:id/retry` | yes | Retry a job |
| GET | `/v1/peers` | yes | Local peer id, listen addrs, connected peers |
| POST | `/v1/peers` | yes | Dial a peer (`{ "multiaddr": "…" }`) |
| GET | `/v1/logs` | yes | Recent runtime logs |
| GET | `/v1/events` | yes | WebSocket event + status stream |

All authenticated routes require `Authorization: Bearer <token>`. Errors are
returned as `{ "error": "message" }` with an appropriate status code.

## @cluster-runtime/client

```ts
import { ClusterClient } from "@cluster-runtime/client";

const client = await ClusterClient.connect(); // auto-discovers endpoint.json
const overview = await client.cluster.overview();
const ack = await client.jobs.submit({
  name: "hello.py",
  entryPoint: { type: "pythonScript", script: "print('hi')" },
});

const stream = client.onEvent((evt) => console.log(evt.type));
// stream.close() when done
```

The library is transport-agnostic: `ClusterClient` takes a `Transport`, and the
default `HttpTransport` implements REST + WebSocket. Discovery, `.cluster`
config parsing, and all API types are exported.

## Development

```bash
# from clients/
pnpm install
pnpm -r build       # build client + extension
pnpm -r test        # run client unit tests
```

## Troubleshooting

- **"No running Cluster Runtime found"** — neither the desktop app nor
  `cluster-runtime-server` is running, or `endpoint.json` has not been written
  yet. Launch one of them and retry.
- **401 Unauthorized** — a stale token. Restart the runtime to regenerate the
  discovery file, then reconnect.
- **Cannot reach the API** — confirm nothing else occupies port 8129 (including
  both GUI and headless at once), or set `CLUSTER_RUNTIME_API_ADDR` and reconnect.
