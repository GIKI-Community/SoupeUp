# Scheduler Plugin Development Guide

To add a new scheduler (e.g. HTCondor, Slurm, Celery):

1. Implement your scheduler plugin using the existing plugin pattern (`services/`, `client/`, etc.).
2. Create `your_scheduler/adapter.rs` implementing `SchedulerService`.
3. Register the adapter in `lib.rs` after service initialization:
   ```rust
   scheduler_registry.register(Arc::new(YourSchedulerAdapter::new(svc))).await;
   ```
4. Advertise honest `SchedulerCapabilities`.
5. Map native job states to unified `JobStatus`.
6. Do **not** modify `jobs/manager` or the UI — only the adapter.

## Capability negotiation

`JobManager` calls `validate_job()` before dispatch. Unsupported resource fields produce warnings (GPU without `supports_gpu`, etc.) but submission proceeds best-effort.

## Testing checklist

- Submit Python function, script, module, and example jobs
- Cancel in-flight job
- Status/progress/result mapping
- Capability detection
