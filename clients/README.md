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
ApiServer (axum, inside the Tauri desktop app)
        │
        ├─ JobManager / JobApi
        ├─ SchedulerRegistry → Dask / Ray adapters
        └─ EventBus → WebSocket stream
```

The desktop app remains authoritative for cluster administration. Clients only
talk to the local HTTP API; they never touch Dask or Ray directly.

## Desktop API server

The desktop app (`cluster-runtime/src-tauri/src/api_server/`) starts an
`axum` server on `127.0.0.1:8129` (override with the `CLUSTER_RUNTIME_API_ADDR`
env var). On startup it generates a random bearer token and writes a discovery
file so clients can auto-connect:

- **Windows:** `%APPDATA%\dev.cluster-runtime.app\api\endpoint.json`
- **macOS:** `~/Library/Application Support/dev.cluster-runtime.app/api/endpoint.json`
- **Linux:** `~/.local/share/dev.cluster-runtime.app/api/endpoint.json`

```json
{ "url": "http://127.0.0.1:8129", "token": "…", "pid": 1234 }
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
| POST | `/v1/jobs` | yes | Submit a `JobSpec` (`?owner=` optional) |
| GET | `/v1/jobs` | yes | List jobs |
| GET | `/v1/jobs/:id` | yes | Job detail (progress, logs, result) |
| GET | `/v1/jobs/:id/result` | yes | Job result |
| POST | `/v1/jobs/:id/cancel` | yes | Cancel a job |
| POST | `/v1/jobs/:id/retry` | yes | Retry a job |
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

- **"No running Cluster Runtime found"** — the desktop app isn't running, or it
  hasn't written `endpoint.json` yet. Launch the desktop app and retry.
- **401 Unauthorized** — a stale token. Restart the desktop app to regenerate
  the discovery file, then reconnect.
- **Cannot reach the API** — confirm nothing else occupies port 8129, or set
  `CLUSTER_RUNTIME_API_ADDR` and reconnect.
