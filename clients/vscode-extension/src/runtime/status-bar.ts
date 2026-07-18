import { schedulerDisplayName } from "@cluster-runtime/client";
import * as vscode from "vscode";

import type { ConnectionService } from "../services/connection";

/** Status bar entry: `Cluster <state> | <scheduler> | N workers`. */
export class ClusterStatusBar implements vscode.Disposable {
  private readonly item: vscode.StatusBarItem;

  constructor(private readonly connection: ConnectionService) {
    this.item = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Left,
      100,
    );
    this.item.command = "clusterRuntime.focusView";
    this.connection.onDidChangeState(() => this.render());
    this.render();
    this.item.show();
  }

  private render(): void {
    const state = this.connection.state;
    if (state === "connecting") {
      this.item.text = "$(sync~spin) Cluster: connecting";
      this.item.tooltip = "Connecting to Cluster Runtime…";
      this.item.backgroundColor = undefined;
      return;
    }
    if (state !== "connected") {
      this.item.text = "$(circle-slash) Cluster: offline";
      this.item.tooltip = "Cluster Runtime not connected. Click to connect.";
      this.item.command = "clusterRuntime.connect";
      this.item.backgroundColor = new vscode.ThemeColor(
        "statusBarItem.warningBackground",
      );
      return;
    }

    const ov = this.connection.clusterOverview;
    const scheduler = ov ? schedulerDisplayName(ov.activeScheduler) : "?";
    const workers = ov?.workerCount ?? 0;
    const health = ov?.schedulerRunning ? "$(cluster)" : "$(debug-pause)";
    this.item.text = `${health} Cluster: ${scheduler} | ${workers} worker${workers === 1 ? "" : "s"}`;
    this.item.tooltip = `Cluster Runtime connected · scheduler ${scheduler} · ${workers} workers`;
    this.item.command = "clusterRuntime.focusView";
    this.item.backgroundColor = undefined;
  }

  dispose(): void {
    this.item.dispose();
  }
}
