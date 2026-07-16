import { invoke } from "@tauri-apps/api/core";

import type {
  ActivityEntry,
  Job,
  LogEntry,
  MetricsSnapshot,
  Node,
  Plugin,
  SystemInfo,
  SystemStatus,
} from "@/types";

async function invokeCommand<T>(command: string): Promise<T> {
  return invoke<T>(command);
}

export const SystemApi = {
  getInfo: () => invokeCommand<SystemInfo>("get_system_info"),
  getStatus: () => invokeCommand<SystemStatus>("get_system_status"),
  getActivity: () => invokeCommand<ActivityEntry[]>("get_activity"),
};

export const NodeApi = {
  list: () => invokeCommand<Node[]>("get_nodes"),
};

export const JobApi = {
  list: () => invokeCommand<Job[]>("get_jobs"),
};

export const PluginApi = {
  list: () => invokeCommand<Plugin[]>("get_plugins"),
};

export const MetricsApi = {
  get: () => invokeCommand<MetricsSnapshot>("get_metrics"),
};

export const LogsApi = {
  list: () => invokeCommand<LogEntry[]>("get_logs"),
};
