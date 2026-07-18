import {
  schedulerDisplayName,
  starterClusterConfig,
} from "@cluster-runtime/client";
import * as vscode from "vscode";

import { JobTreeItem } from "../explorer/trees";
import { runOnCluster } from "../jobs/run-on-cluster";
import type { ConnectionService } from "../services/connection";
import { DASK_PLUGIN_ID } from "../settings";

export interface TreeProviders {
  cluster: { refresh(): void };
  jobs: { refresh(): void };
  workers: { refresh(): void };
  schedulers: { refresh(): void };
  logs: { refresh(): void };
}

/** Default dashboard ports for the built-in schedulers. */
const DASHBOARD_PORTS: Record<string, number> = {
  "plugin-dask-scheduler": 8787,
  "plugin-ray": 8265,
};

export function registerCommands(
  context: vscode.ExtensionContext,
  connection: ConnectionService,
  output: vscode.OutputChannel,
  providers: TreeProviders,
): void {
  const refreshAll = () => {
    providers.cluster.refresh();
    providers.jobs.refresh();
    providers.workers.refresh();
    providers.schedulers.refresh();
    providers.logs.refresh();
  };

  const resolveJobId = async (
    arg: JobTreeItem | string | undefined,
  ): Promise<string | undefined> => {
    if (typeof arg === "string") return arg;
    if (arg instanceof JobTreeItem) return arg.job.id;
    if (!connection.isConnected()) return undefined;
    const jobs = await connection.requireClient().jobs.list();
    if (jobs.length === 0) {
      void vscode.window.showInformationMessage("No jobs available.");
      return undefined;
    }
    const pick = await vscode.window.showQuickPick(
      jobs.map((j) => ({ label: j.name || j.id, description: `${j.status}`, id: j.id })),
      { placeHolder: "Select a job" },
    );
    return pick?.id;
  };

  const register = (id: string, handler: (...args: any[]) => unknown) =>
    context.subscriptions.push(vscode.commands.registerCommand(id, handler));

  register("clusterRuntime.connect", async () => {
    await connection.connect(false);
    refreshAll();
  });

  register("clusterRuntime.disconnect", () => {
    connection.disconnect();
    refreshAll();
  });

  register("clusterRuntime.runOnCluster", () => runOnCluster(connection, output));

  register("clusterRuntime.cancelJob", async (arg?: JobTreeItem | string) => {
    const jobId = await resolveJobId(arg);
    if (!jobId) return;
    try {
      await connection.requireClient().jobs.cancel(jobId);
      void vscode.window.showInformationMessage(`Cancelled job ${jobId}.`);
      providers.jobs.refresh();
    } catch (err) {
      void vscode.window.showErrorMessage(
        `Failed to cancel job: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  });

  register("clusterRuntime.restartJob", async (arg?: JobTreeItem | string) => {
    const jobId = await resolveJobId(arg);
    if (!jobId) return;
    try {
      const ack = await connection.requireClient().jobs.retry(jobId);
      void vscode.window.showInformationMessage(`Restarted job as ${ack.jobId}.`);
      providers.jobs.refresh();
    } catch (err) {
      void vscode.window.showErrorMessage(
        `Failed to restart job: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  });

  register("clusterRuntime.viewJobLogs", async (arg?: JobTreeItem | string) => {
    const jobId = await resolveJobId(arg);
    if (!jobId) return;
    try {
      const detail = await connection.requireClient().jobs.get(jobId);
      output.show(true);
      output.appendLine(`\n─── logs for job ${jobId} (${detail.status}) ───`);
      if (detail.logs.length === 0) output.appendLine("(no logs)");
      for (const line of detail.logs) output.appendLine(line);
    } catch (err) {
      void vscode.window.showErrorMessage(
        `Failed to load job logs: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  });

  register("clusterRuntime.viewDashboard", async () => {
    const ov = connection.clusterOverview;
    const pluginId = ov?.activeScheduler ?? DASK_PLUGIN_ID;
    const port = DASHBOARD_PORTS[pluginId] ?? 8787;
    const url = `http://127.0.0.1:${port}`;
    await vscode.env.openExternal(vscode.Uri.parse(url));
  });

  register("clusterRuntime.openDesktop", async () => {
    await vscode.window.showInformationMessage(
      "Launch the Cluster Runtime desktop app to manage your cluster.",
    );
  });

  register("clusterRuntime.refresh", async () => {
    await connection.refreshOverview();
    refreshAll();
  });

  register("clusterRuntime.selectScheduler", async (pluginId?: string) => {
    if (!connection.isConnected()) {
      void vscode.window.showErrorMessage("Connect to Cluster Runtime first.");
      return;
    }
    const client = connection.requireClient();
    let target = pluginId;
    if (!target) {
      const list = await client.schedulers.list();
      const pick = await vscode.window.showQuickPick(
        list.map((s) => ({
          label: s.displayName,
          description: s.available ? "" : "unavailable",
          id: s.pluginId,
        })),
        { placeHolder: "Select the active scheduler" },
      );
      target = pick?.id;
    }
    if (!target) return;
    try {
      await client.schedulers.setActive(target);
      void vscode.window.showInformationMessage(
        `Active scheduler: ${schedulerDisplayName(target)}.`,
      );
      await connection.refreshOverview();
      refreshAll();
    } catch (err) {
      void vscode.window.showErrorMessage(
        `Failed to set scheduler: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  });

  register("clusterRuntime.initializeProject", async () => {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders?.length) {
      void vscode.window.showErrorMessage("Open a folder to initialize a project.");
      return;
    }
    const target = vscode.Uri.joinPath(folders[0].uri, ".cluster");
    try {
      await vscode.workspace.fs.stat(target);
      const overwrite = await vscode.window.showWarningMessage(
        "A .cluster file already exists. Overwrite?",
        "Overwrite",
        "Cancel",
      );
      if (overwrite !== "Overwrite") return;
    } catch {
      // Does not exist yet — good.
    }
    await vscode.workspace.fs.writeFile(
      target,
      Buffer.from(starterClusterConfig("dask", "main.py"), "utf8"),
    );
    const doc = await vscode.workspace.openTextDocument(target);
    await vscode.window.showTextDocument(doc);
  });

  register("clusterRuntime.focusView", () =>
    vscode.commands.executeCommand("workbench.view.extension.clusterRuntime"),
  );
}
