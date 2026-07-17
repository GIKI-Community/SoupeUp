# Scheduler Abstraction Design

## SchedulerService trait

Located in `src-tauri/src/scheduler/abstraction/mod.rs`.

Schedulers implement:

- `capabilities()` — advertise supported features
- `submit(job_id, spec)` — execute a job
- `cancel`, `status`, `progress`, `result`, `list_jobs`
- `cluster_info()` — cluster health snapshot

## Adapters

Thin wrappers in `dask/adapter.rs` and `ray/adapter.rs`:

- Wrap existing `DaskService` / `RayService`
- Map native `JobResult` / `ExampleJobResult` → unified `jobs::models::JobResult`
- Track platform job IDs in adapter-local state

**Dask/Ray internals are not modified.** Adapters delegate to existing `jobs/JobService` and `client/` modules.

## Registry

`SchedulerRegistry` maps plugin IDs (`plugin-dask-scheduler`, `plugin-ray`) to `Arc<dyn SchedulerService>`.

Active scheduler is persisted to `{app_data}/scheduler/active_scheduler.json`.
