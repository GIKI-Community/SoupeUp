# Cluster SDK Overview

## Python SDK

Package: `cluster-runtime/sdk/python` (`cluster-sdk` on PyPI path TBD)

```python
from cluster_sdk import Cluster, PythonJob, ResourceRequirements

cluster = Cluster(invoke_fn=tauri_invoke)

job = PythonJob(
    name="Mandelbrot",
    example_id="mandelbrot",
    inputs={"width": 4096, "height": 4096},
)

ack = cluster.submit(job)
result = cluster.result(ack["jobId"])
```

## JobSpec JSON schema

Core fields (camelCase over Tauri IPC):

```json
{
  "name": "My Job",
  "description": "optional",
  "entryPoint": {
    "type": "example",
    "exampleId": "mandelbrot"
  },
  "args": {},
  "resources": {
    "cpuCores": 4,
    "memoryBytes": 2000000000
  },
  "tags": ["example"]
}
```

Entry point types: `pythonFunction`, `pythonScript`, `pythonModule`, `example`.

## TypeScript (in-app)

See `cluster-runtime/src/api/index.ts` — `JobApi` and `SchedulerApi`.

## Future SDKs

Rust (native), Go, and JavaScript SDKs should serialize the same `JobSpec` JSON and call `job_submit`.

HTTP transport for external clients is deferred to a follow-up milestone.
