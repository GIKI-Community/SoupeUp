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

export type PluginStatus = "enabled" | "disabled" | "error" | "updating";

export interface Plugin {
  id: string;
  name: string;
  version: string;
  status: PluginStatus;
  author: string;
  description: string;
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
