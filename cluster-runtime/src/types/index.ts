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
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export interface Job {
  id: string;
  status: JobStatus;
  owner: string;
  submittedAt: string;
  runtime: string;
  durationSecs: number;
}

export type PluginStatus =
  | "discovered"
  | "validated"
  | "loaded"
  | "initializing"
  | "running"
  | "error"
  | "disabled";

export interface Plugin {
  id: string;
  name: string;
  version: string;
  status: PluginStatus;
  author: string;
  description: string;
  capabilities?: string[];
  pluginType?: string;
}

// ─── Python Runtime ───────────────────────────────────────────────────────────

export interface ExecutionResult {
  stdout: string;
  stderr: string;
  exitCode: number;
  executionTimeMs: number;
  returnValue: string | null;
  exception: string | null;
  success: boolean;
}

export interface PackageInfo {
  name: string;
  version: string;
  location: string;
}

export interface EnvironmentInfo {
  name: string;
  path: string;
  pythonVersion: string | null;
  packageCount: number;
  active: boolean;
}

export interface PythonRuntimeHealth {
  status: "ready" | "initializing" | "degraded" | "failed";
  pythonVersion: string | null;
  activeEnvironment: string | null;
  environmentPath: string | null;
  interpreterPath: string | null;
  isBundled: boolean;
}

export interface MetricPoint {
  timestamp: string;
  value: number;
}

export interface MetricSeries {
  name: string;
  unit: string;
  points: MetricPoint[];
}

export interface MetricsSnapshot {
  cpu: MetricSeries;
  memory: MetricSeries;
  network: MetricSeries;
  disk: MetricSeries;
  collectedAt: string;
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

export interface ActivityEntry {
  id: string;
  timestamp: string;
  category: string;
  message: string;
}

export interface AppSettings {
  theme: "dark" | "light" | "system";
  accentColor: string;
  language: string;
  autoStart: boolean;
  telemetryEnabled: boolean;
  listenAddress: string;
  port: number;
  enableMdns: boolean;
  enableRemote: boolean;
  authEnabled: boolean;
  tlsEnabled: boolean;
}

// ─── Dask Scheduler Plugin ────────────────────────────────────────────────────

export type ComponentStatus =
  | "stopped"
  | "starting"
  | "running"
  | "stopping"
  | "error"
  | "unknown";

export type ClusterHealth = "healthy" | "degraded" | "unhealthy" | "unknown";

export interface SchedulerInfo {
  status: ComponentStatus;
  address: string | null;
  dashboardUrl: string | null;
  processId: string | null;
  host: string;
  port: number;
  dashboardPort: number;
  startedAt: string | null;
  error: string | null;
  logs: string;
}

export interface WorkerInfo {
  status: ComponentStatus;
  name: string;
  schedulerAddress: string;
  processId: string | null;
  nthreads: number;
  memoryLimit: string | null;
  startedAt: string | null;
  error: string | null;
  logs: string;
}

export interface ConnectedWorker {
  id: string;
  name: string;
  address: string;
  nthreads: number;
  memoryLimit: number;
  memoryUsed: number;
  cpu: number;
  status: string;
}

export interface ClusterSnapshot {
  health: ClusterHealth;
  scheduler: SchedulerInfo;
  localWorker: WorkerInfo;
  workers: ConnectedWorker[];
  totalCores: number;
  totalMemory: number;
  activeTasks: number;
  completedTasks: number;
  failedTasks: number;
  bandwidthBytesPerSec: number;
  clientConnected: boolean;
  updatedAt: string | null;
}

export interface DaskSettings {
  schedulerHost: string;
  schedulerPort: number;
  dashboardPort: number;
  schedulerAddress: string;
  workerThreads: number;
  workerMemoryLimit: string;
  workerName: string;
  localDirectory: string;
  loggingLevel: string;
}

export interface DashboardTab {
  id: string;
  label: string;
  path: string;
  url: string;
}

export interface DashboardView {
  baseUrl: string;
  tabs: DashboardTab[];
}

export interface DaskMetrics {
  schedulerCpu: number;
  schedulerMemory: number;
  workerCpu: number;
  workerMemory: number;
  tasksPerSec: number;
  dataTransfer: number;
  workerLoad: number;
  workerCount: number;
}

export interface ExampleJobResult {
  exampleId: string;
  title: string;
  success: boolean;
  executionTimeMs: number;
  workersUsed: number;
  cpuUtilization: number | null;
  speedup: number | null;
  resultSummary: string;
  details: unknown;
  error: string | null;
}

export function exampleErrorMessage(result: ExampleJobResult): string {
  if (result.error?.trim()) return result.error;
  if (result.resultSummary?.trim()) return result.resultSummary;
  return "Example job failed with no error details.";
}

export const DASK_EXAMPLES = [
  {
    id: "mandelbrot",
    title: "Mandelbrot Renderer",
    description: "Distribute fractal row rendering across workers.",
    packages: ["numpy"],
  },
  {
    id: "monte_carlo_pi",
    title: "Monte Carlo π Estimation",
    description: "Estimate π with parallel random sampling.",
    packages: [] as string[],
  },
  {
    id: "matrix_multiply",
    title: "Matrix Multiplication",
    description: "Parallel block matrix multiplications.",
    packages: ["numpy"],
  },
  {
    id: "prime_search",
    title: "Prime Number Search",
    description: "Search primes up to a limit across the cluster.",
    packages: [] as string[],
  },
  {
    id: "image_blur",
    title: "Image Blur",
    description: "Synthetic image row blur distributed by row.",
    packages: ["numpy"],
  },
  {
    id: "word_count",
    title: "Word Count",
    description: "Classic map-reduce word count over text lines.",
    packages: [] as string[],
  },
] as const;

