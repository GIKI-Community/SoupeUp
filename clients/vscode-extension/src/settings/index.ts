import * as vscode from "vscode";

export const DASK_PLUGIN_ID = "plugin-dask-scheduler";
export const RAY_PLUGIN_ID = "plugin-ray";

export type NotificationPreference = "all" | "failuresOnly" | "none";

export interface ExtensionSettings {
  autoConnect: boolean;
  defaultScheduler: "" | "dask" | "ray";
  watchFileChanges: boolean;
  openDashboardAfterSubmission: boolean;
  notifications: NotificationPreference;
}

export function getSettings(): ExtensionSettings {
  const cfg = vscode.workspace.getConfiguration("clusterRuntime");
  return {
    autoConnect: cfg.get<boolean>("autoConnect", true),
    defaultScheduler: cfg.get<ExtensionSettings["defaultScheduler"]>("defaultScheduler", ""),
    watchFileChanges: cfg.get<boolean>("watchFileChanges", false),
    openDashboardAfterSubmission: cfg.get<boolean>("openDashboardAfterSubmission", false),
    notifications: cfg.get<NotificationPreference>("notifications", "all"),
  };
}

/** Map a short scheduler alias (dask/ray) to its backend plugin id. */
export function schedulerAliasToPluginId(alias: string): string | undefined {
  const normalized = alias.trim().toLowerCase();
  if (normalized === "dask" || normalized === DASK_PLUGIN_ID) return DASK_PLUGIN_ID;
  if (normalized === "ray" || normalized === RAY_PLUGIN_ID) return RAY_PLUGIN_ID;
  return undefined;
}
