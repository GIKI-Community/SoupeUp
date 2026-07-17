import { invoke } from "@tauri-apps/api/core";

import type {
  ActivityEntry,
  ClusterSnapshot,
  DashboardView,
  DaskMetrics,
  DaskSettings,
  EnvironmentInfo,
  ExampleJobResult,
  ExecutionResult,
  Job,
  LogEntry,
  MetricsSnapshot,
  Node,
  PackageInfo,
  Plugin,
  PythonRuntimeHealth,
  SchedulerInfo,
  SystemInfo,
  SystemStatus,
  WorkerInfo,
} from "@/types";

async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return args === undefined ? invoke<T>(command) : invoke<T>(command, args);
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

export const ClusterApi = {
  getSummary: () =>
    invokeCommand<{
      total_nodes: number;
      online_nodes: number;
      total_cpus: number;
      total_ram: number;
      total_gpus: number;
      total_workers: number;
      total_available_compute: number;
    }>("get_cluster_summary"),
  getPeers: () =>
    invokeCommand<
      {
        node_id: string;
        node_name: string;
        host: string;
        port: number;
        status:
          | "Online"
          | "Offline"
          | "Connecting"
          | "Authenticating"
          | "Disconnected";
        resources: {
          cpu_cores: number;
          cpu_usage: number;
          ram_total: number;
          ram_used: number;
          ram_available: number;
          gpu_count: number;
          worker_count: number;
          active_jobs: number;
        };
        version: string;
        connected_since: string;
        last_heartbeat: string;
        latency_ms: number;
      }[]
    >("get_cluster_peers"),
};

export const PythonApi = {
  executeCode: (code: string) =>
    invokeCommand<ExecutionResult>("python_execute_code", { code }),
  executeScript: (scriptPath: string) =>
    invokeCommand<ExecutionResult>("python_execute_script", {
      scriptPath,
    }),
  executeModule: (module: string) =>
    invokeCommand<ExecutionResult>("python_execute_module", { module }),
  installPackage: (packageName: string, version?: string) =>
    invokeCommand<PackageInfo>("python_install_package", {
      package: packageName,
      version: version ?? null,
    }),
  uninstallPackage: (packageName: string) =>
    invokeCommand<void>("python_uninstall_package", { package: packageName }),
  listPackages: () => invokeCommand<PackageInfo[]>("python_list_packages"),
  createEnvironment: (name: string) =>
    invokeCommand<EnvironmentInfo>("python_create_environment", { name }),
  deleteEnvironment: (name: string) =>
    invokeCommand<void>("python_delete_environment", { name }),
  activateEnvironment: (name: string) =>
    invokeCommand<void>("python_activate_environment", { name }),
  listEnvironments: () =>
    invokeCommand<EnvironmentInfo[]>("python_list_environments"),
  runtimeHealth: () =>
    invokeCommand<PythonRuntimeHealth>("python_runtime_health"),
  version: () => invokeCommand<string>("python_version"),
  packageIndex: () => invokeCommand<string>("python_package_index"),
  setPackageIndex: (indexUrl: string) =>
    invokeCommand<void>("python_set_package_index", { indexUrl }),
};

export const DaskApi = {
  ensurePackages: () => invokeCommand<string[]>("dask_ensure_packages"),
  getSettings: () => invokeCommand<DaskSettings>("dask_get_settings"),
  updateSettings: (settings: DaskSettings) =>
    invokeCommand<DaskSettings>("dask_update_settings", { settings }),
  startScheduler: () => invokeCommand<SchedulerInfo>("dask_start_scheduler"),
  stopScheduler: () => invokeCommand<SchedulerInfo>("dask_stop_scheduler"),
  restartScheduler: () =>
    invokeCommand<SchedulerInfo>("dask_restart_scheduler"),
  schedulerStatus: () => invokeCommand<SchedulerInfo>("dask_scheduler_status"),
  startWorker: (schedulerAddress?: string) =>
    invokeCommand<WorkerInfo>("dask_start_worker", {
      schedulerAddress: schedulerAddress ?? null,
    }),
  stopWorker: () => invokeCommand<WorkerInfo>("dask_stop_worker"),
  restartWorker: () => invokeCommand<WorkerInfo>("dask_restart_worker"),
  workerStatus: () => invokeCommand<WorkerInfo>("dask_worker_status"),
  connectClient: (address?: string) =>
    invokeCommand<string>("dask_connect_client", { address: address ?? null }),
  disconnectClient: () => invokeCommand<void>("dask_disconnect_client"),
  clusterSnapshot: () =>
    invokeCommand<ClusterSnapshot>("dask_cluster_snapshot"),
  clusterInfo: () => invokeCommand<unknown>("dask_cluster_info"),
  dashboard: () => invokeCommand<DashboardView>("dask_dashboard"),
  metrics: () => invokeCommand<DaskMetrics>("dask_metrics"),
  runExample: (exampleId: string) =>
    invokeCommand<ExampleJobResult>("dask_run_example", {
      exampleId,
    }),
  submitPythonFunction: (functionBody: string, args: unknown) =>
    invokeCommand<unknown>("dask_submit_python_function", {
      functionBody,
      args,
    }),
  submitScript: (script: string) =>
    invokeCommand<unknown>("dask_submit_script", { script }),
  submitModule: (module: string) =>
    invokeCommand<unknown>("dask_submit_module", { module }),
  map: (functionBody: string, items: unknown) =>
    invokeCommand<unknown>("dask_map", { functionBody, items }),
  scatter: (data: unknown) => invokeCommand<unknown>("dask_scatter", { data }),
  gather: (keys: unknown) => invokeCommand<unknown>("dask_gather", { keys }),
  jobStatus: (jobId: string) =>
    invokeCommand<unknown>("dask_job_status", { jobId }),
  cancelJob: (jobId: string) =>
    invokeCommand<void>("dask_cancel_job", { jobId }),
};
