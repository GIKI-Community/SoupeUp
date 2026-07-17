import type { JobStatus, LogLevel, NodeStatus, PluginStatus, ServiceStatus } from "@/types";

import { Badge } from "@/components/ui/badge";

const nodeStatusVariant: Record<
  NodeStatus,
  "success" | "destructive" | "warning" | "muted"
> = {
  online: "success",
  offline: "destructive",
  degraded: "warning",
  maintenance: "muted",
};

const jobStatusVariant: Record<
  JobStatus,
  "success" | "destructive" | "warning" | "muted" | "default"
> = {
  pending: "muted",
  running: "default",
  completed: "success",
  failed: "destructive",
  cancelled: "warning",
};

const pluginStatusVariant: Record<
  PluginStatus,
  "success" | "destructive" | "warning" | "muted" | "default"
> = {
  discovered: "muted",
  validated: "muted",
  loaded: "default",
  initializing: "warning",
  running: "success",
  error: "destructive",
  disabled: "muted",
};

const serviceStatusVariant: Record<
  ServiceStatus,
  "success" | "destructive" | "warning"
> = {
  healthy: "success",
  degraded: "warning",
  down: "destructive",
};

const logLevelColor: Record<LogLevel, string> = {
  trace: "text-muted-foreground",
  debug: "text-sky-400",
  info: "text-emerald-400",
  warn: "text-amber-400",
  error: "text-red-400",
};

export function NodeStatusBadge({ status }: { status: NodeStatus }) {
  return <Badge variant={nodeStatusVariant[status]}>{status}</Badge>;
}

export function JobStatusBadge({ status }: { status: JobStatus }) {
  return <Badge variant={jobStatusVariant[status]}>{status}</Badge>;
}

export function PluginStatusBadge({ status }: { status: PluginStatus }) {
  return <Badge variant={pluginStatusVariant[status]}>{status}</Badge>;
}

export function ServiceStatusBadge({ status }: { status: ServiceStatus }) {
  return <Badge variant={serviceStatusVariant[status]}>{status}</Badge>;
}

export function LogLevelText({ level }: { level: LogLevel }) {
  return (
    <span className={`font-mono text-xs uppercase ${logLevelColor[level]}`}>
      {level}
    </span>
  );
}
