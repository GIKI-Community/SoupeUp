"""Cluster Runtime Python SDK — scheduler-agnostic job submission."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional

__all__ = ["Cluster", "PythonJob", "ResourceRequirements", "JobStatus"]


class JobStatus(str, Enum):
    CREATED = "created"
    QUEUED = "queued"
    SCHEDULING = "scheduling"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


@dataclass
class ResourceRequirements:
    cpu_cores: Optional[float] = None
    memory_bytes: Optional[int] = None
    gpu_count: Optional[int] = None
    python_version: Optional[str] = None
    packages: List[str] = field(default_factory=list)
    arch: Optional[str] = None
    os: Optional[str] = None
    runtime_type: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {k: v for k, v in asdict(self).items() if v is not None and v != []}


@dataclass
class PythonJob:
    """Submit a Python script or function as a cluster job."""

    name: str
    script: Optional[str] = None
    function_body: Optional[str] = None
    module: Optional[str] = None
    example_id: Optional[str] = None
    inputs: Optional[Dict[str, Any]] = None
    description: Optional[str] = None
    resources: Optional[ResourceRequirements] = None
    tags: List[str] = field(default_factory=list)
    timeout_secs: Optional[int] = None

    def to_spec(self) -> Dict[str, Any]:
        if self.example_id:
            entry_point = {"type": "example", "exampleId": self.example_id}
        elif self.function_body:
            entry_point = {"type": "pythonFunction", "body": self.function_body}
        elif self.module:
            entry_point = {"type": "pythonModule", "module": self.module}
        elif self.script:
            entry_point = {"type": "pythonScript", "script": self.script}
        else:
            raise ValueError(
                "PythonJob requires script, function_body, module, or example_id"
            )

        spec: Dict[str, Any] = {
            "name": self.name,
            "entryPoint": entry_point,
            "args": self.inputs or {},
            "tags": self.tags,
        }
        if self.description:
            spec["description"] = self.description
        if self.resources:
            spec["resources"] = self.resources.to_dict()
        if self.timeout_secs:
            spec["timeoutSecs"] = self.timeout_secs
        return spec


class Cluster:
    """Client for the Cluster Runtime Job API."""

    def __init__(
        self,
        invoke_fn: Optional[Any] = None,
        local_url: Optional[str] = None,
        owner: str = "python-sdk",
    ):
        self._invoke = invoke_fn
        self._local_url = local_url
        self._owner = owner

    def submit(self, job: PythonJob) -> Dict[str, Any]:
        spec = job.to_spec()
        if self._invoke is not None:
            return self._invoke("job_submit", {"spec": spec, "owner": self._owner})
        if self._local_url:
            raise NotImplementedError(
                f"HTTP transport to {self._local_url} is not yet implemented"
            )
        raise RuntimeError(
            "Cluster SDK requires invoke_fn (in-app) or local_url (future HTTP bridge)"
        )

    def cancel(self, job_id: str) -> None:
        if self._invoke is not None:
            self._invoke("job_cancel", {"jobId": job_id})
            return
        raise RuntimeError("Cluster SDK requires invoke_fn")

    def result(self, job_id: str) -> Dict[str, Any]:
        if self._invoke is not None:
            return self._invoke("job_result", {"jobId": job_id})
        raise RuntimeError("Cluster SDK requires invoke_fn")

    def set_scheduler(self, plugin_id: str) -> None:
        if self._invoke is not None:
            self._invoke("scheduler_set_active", {"pluginId": plugin_id})
            return
        raise RuntimeError("Cluster SDK requires invoke_fn")
