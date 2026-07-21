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

export interface JobSpec {
  name: string;
  description?: string;
  entryPoint:
    | { type: "pythonFunction"; body: string }
    | { type: "pythonScript"; script: string }
    | { type: "pythonModule"; module: string }
    | { type: "example"; exampleId: string; args?: unknown };
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

/** Display name for a scheduler plugin id */
export function schedulerDisplayName(pluginId: string): string {
  if (pluginId.includes("dask")) return "Dask";
  if (pluginId.includes("ray")) return "Ray";
  if (pluginId.includes("mpi")) return "MPI";
  return pluginId;
}

/** Unified example catalog for the Jobs / Cluster UI */
export const SCHEDULER_EXAMPLES = [
  { id: "mandelbrot", title: "Mandelbrot Renderer", description: "Render a Mandelbrot fractal across cluster workers" },
  { id: "monte_carlo_pi", title: "Monte Carlo π Estimation", description: "Estimate π using random sampling" },
  { id: "matrix_multiply", title: "Matrix Multiplication", description: "Distributed matrix multiply benchmark" },
  { id: "prime_search", title: "Prime Number Search", description: "Find primes in a range across workers" },
  { id: "image_blur", title: "Image Blur", description: "Apply Gaussian blur to a synthetic image" },
  { id: "word_count", title: "Word Count", description: "Count words in text lines across workers" },
] as const;

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
  /** When true, check GitHub Releases for updates on startup. */
  autoCheckUpdates: boolean;
}

export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion?: string | null;
  updateAvailable: boolean;
  releaseUrl?: string | null;
  releaseNotes?: string | null;
  checkedAt: string;
  message: string;
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

// ─── Ray Types ────────────────────────────────────────────────────────────────

export interface RayHeadInfo {
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

export interface RayWorkerInfo {
  status: ComponentStatus;
  name: string;
  headAddress: string;
  processId: string | null;
  numCpus: number;
  objectStoreMemory: string | null;
  startedAt: string | null;
  error: string | null;
  logs: string;
}

export interface RayClusterSnapshot {
  health: ClusterHealth;
  head: RayHeadInfo;
  localWorker: RayWorkerInfo;
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

export interface RaySettings {
  headHost: string;
  gcsPort: number;
  dashboardPort: number;
  headAddress: string;
  workerCpus: number;
  objectStoreMemory: string;
  workerName: string;
  loggingLevel: string;
}

export interface RayMetrics {
  headCpu: number;
  headMemory: number;
  workerCpu: number;
  workerMemory: number;
  tasksPerSec: number;
  dataTransfer: number;
  workerLoad: number;
  workerCount: number;
}

export const RAY_EXAMPLES = [
  {
    id: "mandelbrot",
    title: "Mandelbrot Renderer",
    description: "Distribute fractal row rendering across Ray workers.",
    packages: ["numpy"],
  },
  {
    id: "monte_carlo_pi",
    title: "Monte Carlo π Estimation",
    description: "Estimate π with parallel random sampling via Ray.",
    packages: [] as string[],
  },
  {
    id: "matrix_multiply",
    title: "Matrix Multiplication",
    description: "Parallel block matrix multiplications with Ray.",
    packages: ["numpy"],
  },
  {
    id: "prime_search",
    title: "Prime Number Search",
    description: "Search primes up to a limit across the Ray cluster.",
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

