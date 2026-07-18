import * as vscode from "vscode";

import { getSettings } from "../settings";

/** Show job lifecycle notifications honoring the user's preference. */
export function notifyJobEvent(
  kind: "started" | "completed" | "failed",
  message: string,
): void {
  const pref = getSettings().notifications;
  if (pref === "none") return;
  if (pref === "failuresOnly" && kind !== "failed") return;

  if (kind === "failed") {
    void vscode.window.showErrorMessage(message);
  } else {
    void vscode.window.showInformationMessage(message);
  }
}

export function notifyRuntimeUnavailable(detail?: string): void {
  const base = "Cluster Runtime is not available. Start the desktop app to connect.";
  void vscode.window
    .showErrorMessage(detail ? `${base} (${detail})` : base, "Open Desktop App")
    .then((choice) => {
      if (choice === "Open Desktop App") {
        void vscode.commands.executeCommand("clusterRuntime.openDesktop");
      }
    });
}

export function notifyConnected(scheduler: string, workers: number): void {
  if (getSettings().notifications === "none") return;
  void vscode.window.showInformationMessage(
    `Connected to Cluster Runtime (${scheduler}, ${workers} worker${workers === 1 ? "" : "s"}).`,
  );
}
