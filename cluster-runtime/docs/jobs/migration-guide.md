# Migration Guide: Scheduler Job Commands → Unified Job API

## Before

```typescript
await DaskApi.runExample("mandelbrot");
await RayApi.submitPythonFunction(body, args);
```

## After

```typescript
await SchedulerApi.setActive("plugin-dask-scheduler"); // or plugin-ray
await JobApi.submitExample("mandelbrot");
await JobApi.submit({ name: "fn", entryPoint: { type: "pythonFunction", body }, args });
```

## Tauri commands

| Legacy | Unified |
|--------|---------|
| `get_jobs` | `job_list` |
| `dask_run_example` / `ray_run_example` | `job_submit` with `entryPoint.type = example` |
| `dask_submit_*` / `ray_submit_*` | `job_submit` |
| `dask_cancel_job` / `ray_cancel_job` | `job_cancel` |
| — | `scheduler_get_active` / `scheduler_set_active` |

Cluster lifecycle commands (`dask_start_scheduler`, `ray_start_head`, etc.) remain per-scheduler during this milestone.

## Rust internal

Replace direct `DaskService::submit_*` calls with `JobManager::submit(JobSpec)`.

Example catalog metadata lives in `jobs/examples/`; scheduler-specific Python bodies remain in `dask/examples/` and `ray/examples/`.
