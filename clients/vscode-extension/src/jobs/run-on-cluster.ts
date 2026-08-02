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
 * POST /v1/jobs returns immediately with a job id; we poll until the job
 * finishes (or the user cancels the progress notification).
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
    // 0 = no hard cap; job finishes when the script/cluster work completes.
    timeoutSecs: 0,
  };

  output.show(true);
  output.appendLine(`\n▶ Running ${fileName} on the cluster…`);
  notifyJobEvent("started", `Submitting ${fileName} to the cluster…`);

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: `Cluster Runtime: running ${fileName}`,
      cancellable: true,
    },
    async (progress, cancelToken) => {
      let jobId: string | undefined;
      const abort = new AbortController();
      const cancelSub = cancelToken.onCancellationRequested(() => {
        abort.abort();
        if (jobId) {
          void client.jobs.cancel(jobId).catch(() => undefined);
        }
      });

      try {
        progress.report({ message: "Submitting…" });
        const ack = await client.jobs.submit(spec, "vscode");
        jobId = ack.jobId;
        output.appendLine(`Job ${jobId} accepted (status: ${ack.status}). Waiting…`);

        let lastLogCount = 0;
        const detail = await client.jobs.wait(jobId, {
          pollIntervalMs: 1500,
          signal: abort.signal,
          onUpdate: (d) => {
            progress.report({
              message: `${d.status}${d.progress?.percent ? ` (${Math.round(d.progress.percent)}%)` : ""}`,
            });
            if (d.logs?.length > lastLogCount) {
              for (const line of d.logs.slice(lastLogCount)) {
                output.appendLine(line);
              }
              lastLogCount = d.logs.length;
            }
          },
        });

        output.appendLine(`Job ${jobId} finished with status: ${detail.status}`);

        const result =
          detail.result ??
          (await client.jobs.result(jobId).catch(() => undefined));

        if (detail.logs?.length && lastLogCount === 0) {
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
          if (result.errors?.length) {
            output.appendLine("─── errors ───");
            for (const err of result.errors) output.appendLine(err);
          }
          const ms = result.metrics?.executionTimeMs ?? 0;
          const workers = result.metrics?.workersUsed ?? 0;
          output.appendLine(
            `\n✔ ${result.status} in ${ms}ms using ${workers} worker(s).`,
          );
        }

        if (detail.status === "completed") {
          notifyJobEvent("completed", `${fileName} completed successfully.`);
          if (getSettings().openDashboardAfterSubmission) {
            void vscode.commands.executeCommand("clusterRuntime.viewDashboard");
          }
        } else if (detail.status === "cancelled" || cancelToken.isCancellationRequested) {
          output.appendLine("✖ Job cancelled.");
          notifyJobEvent("failed", `${fileName} was cancelled.`);
        } else {
          notifyJobEvent("failed", `${fileName} finished as ${detail.status}.`);
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        if (cancelToken.isCancellationRequested || /aborted/i.test(message)) {
          output.appendLine("✖ Cancelled.");
          notifyJobEvent("failed", `Cancelled ${fileName}.`);
        } else {
          output.appendLine(`✖ Submission failed: ${message}`);
          notifyJobEvent("failed", `Failed to run ${fileName}: ${message}`);
        }
      } finally {
        cancelSub.dispose();
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
