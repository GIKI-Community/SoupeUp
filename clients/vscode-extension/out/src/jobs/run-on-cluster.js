"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.runOnCluster = runOnCluster;
const client_1 = require("@cluster-runtime/client");
const vscode = __importStar(require("vscode"));
const notifications_1 = require("../notifications");
const settings_1 = require("../settings");
/**
 * Save the active Python file and submit its source as a single-file job.
 * The backend `POST /v1/jobs` currently blocks until completion, so we show
 * progress, then surface logs and the result once it resolves.
 */
async function runOnCluster(connection, output) {
    if (!connection.isConnected()) {
        const connected = await connection.connect();
        if (!connected)
            return;
    }
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "python") {
        void vscode.window.showErrorMessage("Open a Python file to run it on the cluster.");
        return;
    }
    await editor.document.save();
    const doc = editor.document;
    const script = doc.getText();
    const fileName = doc.fileName.split(/[\\/]/).pop() ?? "script.py";
    const client = connection.requireClient();
    // Honor a workspace `.cluster` config for scheduler selection.
    await applyClusterConfigScheduler(connection);
    const spec = {
        name: fileName,
        description: `Submitted from VS Code (${fileName})`,
        entryPoint: { type: "pythonScript", script },
        tags: ["vscode"],
    };
    output.show(true);
    output.appendLine(`\n▶ Running ${fileName} on the cluster…`);
    (0, notifications_1.notifyJobEvent)("started", `Submitting ${fileName} to the cluster…`);
    await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: `Cluster Runtime: running ${fileName}`,
        cancellable: false,
    }, async () => {
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
                for (const line of detail.logs)
                    output.appendLine(line);
            }
            if (result) {
                if (result.output !== undefined && result.output !== null) {
                    output.appendLine("─── output ───");
                    output.appendLine(typeof result.output === "string"
                        ? result.output
                        : JSON.stringify(result.output, null, 2));
                }
                if (result.errors.length) {
                    output.appendLine("─── errors ───");
                    for (const err of result.errors)
                        output.appendLine(err);
                }
                output.appendLine(`\n✔ ${result.status} in ${result.metrics.executionTimeMs}ms using ${result.metrics.workersUsed} worker(s).`);
            }
            if (ack.status === "completed") {
                (0, notifications_1.notifyJobEvent)("completed", `${fileName} completed successfully.`);
                if ((0, settings_1.getSettings)().openDashboardAfterSubmission) {
                    void vscode.commands.executeCommand("clusterRuntime.viewDashboard");
                }
            }
            else {
                (0, notifications_1.notifyJobEvent)("failed", `${fileName} finished as ${ack.status}.`);
            }
        }
        catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            output.appendLine(`✖ Submission failed: ${message}`);
            (0, notifications_1.notifyJobEvent)("failed", `Failed to run ${fileName}: ${message}`);
        }
    });
}
async function applyClusterConfigScheduler(connection) {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders?.length)
        return;
    for (const folder of folders) {
        const pattern = new vscode.RelativePattern(folder, "*.cluster");
        const files = await vscode.workspace.findFiles(pattern, undefined, 1);
        if (files.length === 0)
            continue;
        try {
            const bytes = await vscode.workspace.fs.readFile(files[0]);
            const config = (0, client_1.parseClusterConfig)(Buffer.from(bytes).toString("utf8"));
            if (config.scheduler) {
                const pluginId = (0, settings_1.schedulerAliasToPluginId)(config.scheduler);
                if (pluginId) {
                    await connection.requireClient().schedulers.setActive(pluginId);
                }
            }
        }
        catch {
            // Ignore malformed config; fall back to the active scheduler.
        }
        return;
    }
}
//# sourceMappingURL=run-on-cluster.js.map