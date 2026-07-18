# Cluster Runtime for VS Code

Run and monitor distributed Python jobs on your local Cluster Runtime (Dask or
Ray) without leaving the editor. The extension talks only to the local Cluster
Runtime API via [`@cluster-runtime/client`](../client); the desktop app stays
authoritative for cluster administration.

## Features (MVP)

- **Auto-connect** to a running Cluster Runtime desktop app and reconnect when
  it restarts.
- **Status bar** entry: `Cluster: <scheduler> | N workers`.
- **Activity Bar** with live tree views: Cluster, Jobs, Workers, Schedulers,
  Logs.
- **Run on Cluster** — submit the active `.py` file's source as a job, stream
  its logs to an output channel, and surface the result.
- **Scheduler selection** between Dask and Ray.
- **Job monitoring** — cancel and retry jobs, and open their logs.

## Requirements

- The Cluster Runtime desktop app must be running locally (it exposes the API
  and writes the discovery file the extension uses to connect).

## Commands

| Command | Description |
| ------- | ----------- |
| `Cluster Runtime: Connect` | Discover and connect to the runtime |
| `Cluster Runtime: Disconnect` | Drop the connection |
| `Cluster Runtime: Run on Cluster` | Submit the active Python file |
| `Cluster Runtime: Cancel Job` | Cancel a running job |
| `Cluster Runtime: Restart Job` | Retry a finished job |
| `Cluster Runtime: View Job Logs` | Print a job's logs to the output channel |
| `Cluster Runtime: View Dashboard` | Open the scheduler dashboard in a browser |
| `Cluster Runtime: Open Desktop App` | Reminder to launch the desktop app |
| `Cluster Runtime: Refresh Cluster` | Refresh all views |
| `Cluster Runtime: Select Scheduler` | Choose Dask or Ray |
| `Cluster Runtime: Initialize Project` | Write a starter `.cluster` config |

## Settings

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `clusterRuntime.autoConnect` | `true` | Auto-discover and connect on activation |
| `clusterRuntime.defaultScheduler` | `""` | Scheduler to select on connect (`dask`/`ray`) |
| `clusterRuntime.watchFileChanges` | `false` | Re-run on save (reserved) |
| `clusterRuntime.openDashboardAfterSubmission` | `false` | Open dashboard after a job |
| `clusterRuntime.notifications` | `all` | `all`, `failuresOnly`, or `none` |

## `.cluster` configuration

Place a `.cluster` YAML file at the root of your project. `Initialize Project`
scaffolds one:

```yaml
scheduler: dask        # dask | ray
entry: main.py         # entry point (reserved for multi-file runs)
working_directory: .
arguments: []
environment: default
upload_project: false  # reserved
watch_changes: false   # reserved
```

Today the extension reads `scheduler` to pick the active backend before a run.
The remaining keys are reserved for future multi-file / packaging support.

## Development

```bash
# from clients/
pnpm install
cd vscode-extension
pnpm build            # bundle with esbuild → dist/extension.js
pnpm watch            # rebuild on change
pnpm test             # activation + command-registration smoke test
```

Press <kbd>F5</kbd> in VS Code to launch an Extension Development Host.

## Troubleshooting

- **Status bar shows "offline"** — start the desktop app; the extension polls
  for it every few seconds and connects automatically.
- **401 / token mismatch** — restart the desktop app to regenerate the token,
  then run `Cluster Runtime: Connect`.
- **Run on Cluster does nothing** — make sure the active editor is a `.py` file
  and the runtime is connected.

## Notes and limitations

- Job submission currently blocks until completion on the backend, so `Run on
  Cluster` shows progress and reveals logs/result when the job finishes. Live
  per-line streaming and remote debugging are planned extension points.
- Project packaging/upload, a `.cluster` GUI editor, and a metrics dashboard are
  intentionally deferred.
