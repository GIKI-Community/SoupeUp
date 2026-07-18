import {
  parseClusterConfig,
  type JobSpec,
} from "@cluster-runtime/client";
import * as vscode from "vscode";

import { notifyJobEvent } from "../notifications";
import type { ConnectionService } from "../services/connection";
import { getSettings, schedulerAliasToPluginId } from "../settings";

/**
 * Save the active Python file and submit its source as a single-file job.
 * The backend `POST /v1/jobs` currently blocks until completion, so we show
 * progress, then surface logs and the result once it resolves.
 */
export async function runOnCluster(
  connection: ConnectionService,
  output: vscode.OutputChannel,
): Promise<void> {
  if (!connection.isConnected()) {
    const connected = await connection.connect();
    if (!connected) return;
  }

  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== "python") {
    void vscode.window.showErrorMessage(
      "Open a Python file to run it on the cluster.",
    );
    return;
  }

  await editor.document.save();
  const doc = editor.document;
  const script = doc.getText();
  const fileName = doc.fileName.split(/[\\/]/).pop() ?? "script.py";

  const client = connection.requireClient();

  // Honor a workspace `.cluster` config for scheduler selection.
  await applyClusterConfigScheduler(connection);

  const spec: JobSpec = {
    name: fileName,
    description: `Submitted from VS Code (${fileName})`,
    entryPoint: { type: "pythonScript", script },
    tags: ["vscode"],
  };

  output.show(true);
  output.appendLine(`\n▶ Running ${fileName} on the cluster…`);
  notifyJobEvent("started", `Submitting ${fileName} to the cluster…`);

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: `Cluster Runtime: running ${fileName}`,
      cancellable: false,
    },
    async () => {
      try {
        const ack = await client.jobs.submit(spec, "vscode");
        const jobId = ack.jobId;
        output.appendLine(`Job ${jobId} finished with status: ${ack.status}`);

        const [detail, result] = await Promise.all([
          client.jobs.get(jobId).catch(() => undefined),
          client.jobs.result(jobId).catch(() => undefined),
        ]);

        if (detail?.logs?.length) {
          output.appendLine("─── logs ───");
          for (const line of detail.logs) output.appendLine(line);
        }
        if (result) {
          if (result.output !== undefined && result.output !== null) {
            output.appendLine("─── output ───");
            output.appendLine(
              typeof result.output === "string"
                ? result.output
                : JSON.stringify(result.output, null, 2),
            );
          }
          if (result.errors.length) {
            output.appendLine("─── errors ───");
            for (const err of result.errors) output.appendLine(err);
          }
          output.appendLine(
            `\n✔ ${result.status} in ${result.metrics.executionTimeMs}ms using ${result.metrics.workersUsed} worker(s).`,
          );
        }

        if (ack.status === "completed") {
          notifyJobEvent("completed", `${fileName} completed successfully.`);
          if (getSettings().openDashboardAfterSubmission) {
            void vscode.commands.executeCommand("clusterRuntime.viewDashboard");
          }
        } else {
          notifyJobEvent("failed", `${fileName} finished as ${ack.status}.`);
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        output.appendLine(`✖ Submission failed: ${message}`);
        notifyJobEvent("failed", `Failed to run ${fileName}: ${message}`);
      }
    },
  );
}

async function applyClusterConfigScheduler(
  connection: ConnectionService,
): Promise<void> {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders?.length) return;

  for (const folder of folders) {
    const pattern = new vscode.RelativePattern(folder, "*.cluster");
    const files = await vscode.workspace.findFiles(pattern, undefined, 1);
    if (files.length === 0) continue;

    try {
      const bytes = await vscode.workspace.fs.readFile(files[0]);
      const config = parseClusterConfig(Buffer.from(bytes).toString("utf8"));
      if (config.scheduler) {
        const pluginId = schedulerAliasToPluginId(config.scheduler);
        if (pluginId) {
          await connection.requireClient().schedulers.setActive(pluginId);
        }
      }
    } catch {
      // Ignore malformed config; fall back to the active scheduler.
    }
    return;
  }
}
