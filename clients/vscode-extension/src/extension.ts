import * as vscode from "vscode";

import { registerCommands, type TreeProviders } from "./commands";
import {
  ClusterTreeProvider,
  JobsTreeProvider,
  LogsTreeProvider,
  SchedulersTreeProvider,
  WorkersTreeProvider,
} from "./explorer/trees";
import { ClusterStatusBar } from "./runtime/status-bar";
import { ConnectionService } from "./services/connection";

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel("Cluster Runtime");
  context.subscriptions.push(output);

  const connection = new ConnectionService();
  context.subscriptions.push(connection);

  const cluster = new ClusterTreeProvider(connection);
  const jobs = new JobsTreeProvider(connection);
  const workers = new WorkersTreeProvider(connection);
  const schedulers = new SchedulersTreeProvider(connection);
  const logs = new LogsTreeProvider(connection);

  context.subscriptions.push(
    vscode.window.createTreeView("clusterRuntime.cluster", { treeDataProvider: cluster }),
    vscode.window.createTreeView("clusterRuntime.jobs", { treeDataProvider: jobs }),
    vscode.window.createTreeView("clusterRuntime.workers", { treeDataProvider: workers }),
    vscode.window.createTreeView("clusterRuntime.schedulers", { treeDataProvider: schedulers }),
    vscode.window.createTreeView("clusterRuntime.logs", { treeDataProvider: logs }),
  );

  const providers: TreeProviders = { cluster, jobs, workers, schedulers, logs };
  registerCommands(context, connection, output, providers);

  context.subscriptions.push(new ClusterStatusBar(connection));

  connection.startAutoConnect();
}

export function deactivate(): void {
  // Disposables registered in `context.subscriptions` are cleaned up by VS Code.
}
