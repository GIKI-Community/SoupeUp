# Job System Architecture

Cluster Runtime exposes a **scheduler-agnostic Job API** as its primary public interface.

## Layers

```
Application / UI / SDK
        ↓
   jobs/api (JobApi)
        ↓
   jobs/manager (JobManager)
        ↓
   scheduler/selection (SchedulerRegistry)
        ↓
   Dask / Ray adapters (SchedulerService)
        ↓
   Existing plugin services (unchanged internals)
```

## Modules

| Module | Responsibility |
|--------|----------------|
| `jobs/models` | Unified Job, Task, JobResult, JobStatus types |
| `jobs/manager` | Lifecycle orchestration, validation, dispatch |
| `jobs/history` | Persistent job history (JSONL) |
| `jobs/results` | Result store keyed by platform job ID |
| `jobs/progress` | Unified progress aggregation |
| `jobs/examples` | Shared example catalog metadata |
| `scheduler/abstraction` | `SchedulerService` trait |
| `scheduler/selection` | Registry + active scheduler persistence |

## Key invariant

Applications never call Dask or Ray APIs directly for job submission. They submit `JobSpec` values; the active scheduler adapter translates them.
