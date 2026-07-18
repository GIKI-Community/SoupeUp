import * as assert from "node:assert";
import * as vscode from "vscode";

const EXTENSION_ID = "cluster-runtime.cluster-runtime-vscode";

const EXPECTED_COMMANDS = [
  "clusterRuntime.connect",
  "clusterRuntime.disconnect",
  "clusterRuntime.runOnCluster",
  "clusterRuntime.cancelJob",
  "clusterRuntime.restartJob",
  "clusterRuntime.viewJobLogs",
  "clusterRuntime.viewDashboard",
  "clusterRuntime.openDesktop",
  "clusterRuntime.refresh",
  "clusterRuntime.selectScheduler",
  "clusterRuntime.initializeProject",
];

suite("Cluster Runtime extension", () => {
  test("activates without a running runtime", async () => {
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(ext, "extension should be present");
    await ext!.activate();
    assert.strictEqual(ext!.isActive, true);
  });

  test("registers all contributed commands", async () => {
    const ext = vscode.extensions.getExtension(EXTENSION_ID);
    await ext!.activate();
    const commands = await vscode.commands.getCommands(true);
    for (const cmd of EXPECTED_COMMANDS) {
      assert.ok(commands.includes(cmd), `missing command: ${cmd}`);
    }
  });
});
