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
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const commands_1 = require("./commands");
const trees_1 = require("./explorer/trees");
const status_bar_1 = require("./runtime/status-bar");
const connection_1 = require("./services/connection");
function activate(context) {
    const output = vscode.window.createOutputChannel("Cluster Runtime");
    context.subscriptions.push(output);
    const connection = new connection_1.ConnectionService();
    context.subscriptions.push(connection);
    const cluster = new trees_1.ClusterTreeProvider(connection);
    const jobs = new trees_1.JobsTreeProvider(connection);
    const workers = new trees_1.WorkersTreeProvider(connection);
    const schedulers = new trees_1.SchedulersTreeProvider(connection);
    const logs = new trees_1.LogsTreeProvider(connection);
    context.subscriptions.push(vscode.window.createTreeView("clusterRuntime.cluster", { treeDataProvider: cluster }), vscode.window.createTreeView("clusterRuntime.jobs", { treeDataProvider: jobs }), vscode.window.createTreeView("clusterRuntime.workers", { treeDataProvider: workers }), vscode.window.createTreeView("clusterRuntime.schedulers", { treeDataProvider: schedulers }), vscode.window.createTreeView("clusterRuntime.logs", { treeDataProvider: logs }));
    const providers = { cluster, jobs, workers, schedulers, logs };
    (0, commands_1.registerCommands)(context, connection, output, providers);
    context.subscriptions.push(new status_bar_1.ClusterStatusBar(connection));
    connection.startAutoConnect();
}
function deactivate() {
    // Disposables registered in `context.subscriptions` are cleaned up by VS Code.
}
//# sourceMappingURL=extension.js.map