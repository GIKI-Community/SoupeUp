// Canonical Cluster Runtime API types, shared by all clients.
// Ported from cluster-runtime/src/types/index.ts (camelCase matches the JSON API).

export type NodeStatus = "online" | "offline" | "degraded" | "maintenance";
export type NodePlatform =
  | "windows"
  | "linux"
  | "macOS"
  | "android"
  | "raspberryPi"
  | "other";

export interface Node {
  id: string;
  name: string;
  platform: NodePlatform;
  status: NodeStatus;
  cpuPercent: number;
  memoryPercent: number;
  backend: string;
  version: string;
  lastSeen: string;
}

export type JobStatus =
  | "created"
  | "queued"
  | "scheduling"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export interface ResourceRequirements {
  cpuCores?: number;
  memoryBytes?: number;
  gpuCount?: number;
  pythonVersion?: string;
  packages?: string[];
  arch?: string;
  os?: string;
  runtimeType?: string;
}

export interface DependencyReport {
  detected: string[];
  installed: string[];
  alreadyPresent: string[];
  skippedStdlib: string[];
}

export interface JobProgress {
  percent: number;
  activeTasks: number;
  completedTasks: number;
  failedTasks: number;
  runningNodes: string[];
  etaSecs?: number;
}

export interface JobMetrics {
  executionTimeMs: number;
  workersUsed: number;
  cpuUtilization?: number;
  speedup?: number;
}

export interface JobResult {
  jobId: string;
  status: JobStatus;
  output?: unknown;
  errors: string[];
  metrics: JobMetrics;
  schedulerMetadata?: unknown;
  workers: string[];
  resultSummary?: string;
}

export type EntryPoint =
  | { type: "pythonFunction"; body: string }
  | { type: "pythonScript"; script: string }
  | { type: "pythonModule"; module: string }
  | { type: "example"; exampleId: string; args?: unknown }
  | {
      type: "mpiExecutable";
      executable: string;
      ranks?: number;
      hostfile?: string;
    };

export interface JobSpec {
  name: string;
  description?: string;
  entryPoint: EntryPoint;
  args?: unknown;
  env?: Record<string, string>;
  resources?: ResourceRequirements;
  priority?: number;
  timeoutSecs?: number;
  tags?: string[];
}

export interface Job {
  id: string;
  name: string;
  description?: string;
  status: JobStatus;
  owner: string;
  submittedAt: string;
  schedulerId: string;
  durationSecs: number;
  resources?: ResourceRequirements;
  tags?: string[];
  dependencies?: DependencyReport;
}

export interface JobDetail extends Job {
  progress: JobProgress;
  result?: JobResult;
  logs: string[];
}

export interface SchedulerCapabilities {
  supportsPython: boolean;
  supportsActors: boolean;
  supportsDags: boolean;
  supportsGpu: boolean;
  supportsFaultTolerance: boolean;
  supportsAutoscaling: boolean;
  supportsStreaming: boolean;
}

export interface SchedulerListEntry {
  pluginId: string;
  displayName: string;
  capabilities: SchedulerCapabilities;
  available: boolean;
}

export interface SubmitAck {
  jobId: string;
  status: JobStatus;
}

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";

export interface LogEntry {
  id: string;
  timestamp: string;
  module: string;
  level: LogLevel;
  message: string;
}

export interface SystemInfo {
  totalNodes: number;
  onlineNodes: number;
  activeJobs: number;
  installedPlugins: number;
  cpuUsagePercent: number;
  memoryUsagePercent: number;
  version: string;
  uptimeSecs: number;
}

export type ServiceStatus = "healthy" | "degraded" | "down";

export interface SystemStatus {
  api: ServiceStatus;
  storage: ServiceStatus;
  networking: ServiceStatus;
  pluginManager: ServiceStatus;
}

export interface SystemOverview {
  info: SystemInfo;
  status: SystemStatus;
}

/** Aggregated cluster status returned by GET /v1/cluster. */
export interface ClusterOverview {
  activeScheduler: string;
  schedulerRunning: boolean;
  health: string;
  workerCount: number;
  totalCores: number;
  totalMemory: number;
}

export interface HealthResponse {
  status: string;
  service: string;
  apiVersion: string;
}

/** A frame pushed over the /v1/events WebSocket. */
export interface StreamEvent {
  type: string;
  payload?: unknown;
}

/** Display name for a scheduler plugin id. */
export function schedulerDisplayName(pluginId: string): string {
  if (pluginId.includes("dask")) return "Dask";
  if (pluginId.includes("ray")) return "Ray";
  return pluginId;
}
