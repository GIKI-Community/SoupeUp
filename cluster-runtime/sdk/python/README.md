# Cluster SDK (Python)

Scheduler-agnostic job submission for Cluster Runtime.

```python
from cluster_sdk import Cluster, PythonJob

cluster = Cluster(invoke_fn=your_tauri_invoke_bridge)

job = PythonJob(
    name="Mandelbrot",
    script="mandelbrot.py",
    inputs={"width": 4096, "height": 4096},
    resources=ResourceRequirements(cpu_cores=4, memory_bytes=2_000_000_000),
)

ack = cluster.submit(job)
result = cluster.result(ack["jobId"])
```

See `docs/jobs/sdk-overview.md` for the full JobSpec JSON schema.
