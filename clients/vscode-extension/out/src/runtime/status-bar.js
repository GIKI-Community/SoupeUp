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
exports.ClusterStatusBar = void 0;
const client_1 = require("@cluster-runtime/client");
const vscode = __importStar(require("vscode"));
/** Status bar entry: `Cluster <state> | <scheduler> | N workers`. */
class ClusterStatusBar {
    constructor(connection) {
        this.connection = connection;
        this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
        this.item.command = "clusterRuntime.focusView";
        this.connection.onDidChangeState(() => this.render());
        this.render();
        this.item.show();
    }
    render() {
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
            this.item.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
            return;
        }
        const ov = this.connection.clusterOverview;
        const scheduler = ov ? (0, client_1.schedulerDisplayName)(ov.activeScheduler) : "?";
        const workers = ov?.workerCount ?? 0;
        const health = ov?.schedulerRunning ? "$(cluster)" : "$(debug-pause)";
        this.item.text = `${health} Cluster: ${scheduler} | ${workers} worker${workers === 1 ? "" : "s"}`;
        this.item.tooltip = `Cluster Runtime connected · scheduler ${scheduler} · ${workers} workers`;
        this.item.command = "clusterRuntime.focusView";
        this.item.backgroundColor = undefined;
    }
    dispose() {
        this.item.dispose();
    }
}
exports.ClusterStatusBar = ClusterStatusBar;
//# sourceMappingURL=status-bar.js.map