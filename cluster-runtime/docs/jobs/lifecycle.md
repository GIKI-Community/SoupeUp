# Job Lifecycle

## States

```
Created → Queued → Scheduling → Running → Completed
                                      ↘ Failed
                                      ↘ Cancelled
```

Failed jobs may be retried via `job_retry` (re-submits the stored `JobSpec`).

## Events

`JobManager` publishes on the internal event bus:

- `JobStarted { job_id }`
- `JobFinished { job_id, success }`

## Persistence

History records are appended to `{app_data}/jobs/history.jsonl` with metadata, logs, and original spec for retry.

Results are stored in `{app_data}/jobs/results.json`.
