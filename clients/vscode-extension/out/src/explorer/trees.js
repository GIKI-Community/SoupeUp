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
exports.LogsTreeProvider = exports.SchedulersTreeProvider = exports.WorkersTreeProvider = exports.JobsTreeProvider = exports.JobTreeItem = exports.ClusterTreeProvider = void 0;
const client_1 = require("@cluster-runtime/client");
const vscode = __importStar(require("vscode"));
const FINISHED = ["completed", "failed", "cancelled"];
/** Base provider that refreshes whenever the connection state or events change. */
class LiveTreeProvider {
    constructor(connection) {
        this.connection = connection;
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
        connection.onDidChangeState(() => this.refresh());
        connection.onEvent(() => this.refresh());
    }
    refresh() {
        this._onDidChangeTreeData.fire();
    }
    offlineItem() {
        const item = new vscode.TreeItem("Not connected");
        item.iconPath = new vscode.ThemeIcon("circle-slash");
        item.command = { command: "clusterRuntime.connect", title: "Connect" };
        return item;
    }
}
// ─── Cluster ────────────────────────────────────────────────────────────────
class ClusterTreeProvider extends LiveTreeProvider {
    getTreeItem(element) {
        return element;
    }
    getChildren() {
        if (!this.connection.isConnected())
            return [this.offlineItem()];
        const ov = this.connection.clusterOverview;
        if (!ov)
            return [new vscode.TreeItem("Loading…")];
        const line = (label, value, icon) => {
            const item = new vscode.TreeItem(`${label}: ${value}`);
            item.iconPath = new vscode.ThemeIcon(icon);
            return item;
        };
        return [
            line("Scheduler", (0, client_1.schedulerDisplayName)(ov.activeScheduler), "server-process"),
            line("Status", ov.schedulerRunning ? "running" : "stopped", "pulse"),
            line("Health", ov.health, "heart"),
            line("Workers", String(ov.workerCount), "organization"),
            line("Cores", String(ov.totalCores), "chip"),
            line("Memory", formatBytes(ov.totalMemory), "database"),
        ];
    }
}
exports.ClusterTreeProvider = ClusterTreeProvider;
// ─── Jobs ─────────────────────────────────────────────────────────────────────
class JobTreeItem extends vscode.TreeItem {
    constructor(job) {
        super(job.name || job.id, vscode.TreeItemCollapsibleState.None);
        this.job = job;
        this.id = job.id;
        this.description = `${job.status} · ${job.schedulerId}`;
        this.tooltip = `${job.name}\nid: ${job.id}\nstatus: ${job.status}\nowner: ${job.owner}`;
        this.iconPath = new vscode.ThemeIcon(jobIcon(job.status));
        this.contextValue = FINISHED.includes(job.status)
            ? "job-finished"
            : "job-running";
    }
}
exports.JobTreeItem = JobTreeItem;
class JobsTreeProvider extends LiveTreeProvider {
    getTreeItem(element) {
        return element;
    }
    async getChildren() {
        if (!this.connection.isConnected())
            return [this.offlineItem()];
        try {
            const jobs = await this.connection.requireClient().jobs.list();
            if (jobs.length === 0) {
                return [new vscode.TreeItem("No jobs yet")];
            }
            return jobs
                .slice()
                .sort((a, b) => b.submittedAt.localeCompare(a.submittedAt))
                .map((job) => new JobTreeItem(job));
        }
        catch {
            return [new vscode.TreeItem("Failed to load jobs")];
        }
    }
}
exports.JobsTreeProvider = JobsTreeProvider;
// ─── Workers ───────────────────────────────────────────────────────────────
class WorkersTreeProvider extends LiveTreeProvider {
    getTreeItem(element) {
        return element;
    }
    async getChildren() {
        if (!this.connection.isConnected())
            return [this.offlineItem()];
        try {
            const nodes = await this.connection.requireClient().cluster.nodes();
            if (nodes.length === 0)
                return [new vscode.TreeItem("No workers")];
            return nodes.map((node) => {
                const item = new vscode.TreeItem(node.name);
                item.description = `${node.status} · cpu ${Math.round(node.cpuPercent)}% · mem ${Math.round(node.memoryPercent)}%`;
                item.tooltip = `${node.backend} · ${node.platform} · v${node.version}`;
                item.iconPath = new vscode.ThemeIcon(node.status === "online" ? "vm-active" : "vm-outline");
                return item;
            });
        }
        catch {
            return [new vscode.TreeItem("Failed to load workers")];
        }
    }
}
exports.WorkersTreeProvider = WorkersTreeProvider;
// ─── Schedulers ────────────────────────────────────────────────────────────
class SchedulersTreeProvider extends LiveTreeProvider {
    getTreeItem(element) {
        return element;
    }
    async getChildren() {
        if (!this.connection.isConnected())
            return [this.offlineItem()];
        try {
            const client = this.connection.requireClient();
            const [list, active] = await Promise.all([
                client.schedulers.list(),
                client.schedulers.getActive(),
            ]);
            return list.map((entry) => {
                const isActive = entry.pluginId === active.pluginId;
                const item = new vscode.TreeItem(entry.displayName);
                item.description = isActive
                    ? "active"
                    : entry.available
                        ? "available"
                        : "unavailable";
                item.iconPath = new vscode.ThemeIcon(isActive ? "check" : "circle-large-outline");
                item.command = {
                    command: "clusterRuntime.selectScheduler",
                    title: "Select Scheduler",
                    arguments: [entry.pluginId],
                };
                return item;
            });
        }
        catch {
            return [new vscode.TreeItem("Failed to load schedulers")];
        }
    }
}
exports.SchedulersTreeProvider = SchedulersTreeProvider;
// ─── Logs ─────────────────────────────────────────────────────────────────────
class LogsTreeProvider extends LiveTreeProvider {
    getTreeItem(element) {
        return element;
    }
    async getChildren() {
        if (!this.connection.isConnected())
            return [this.offlineItem()];
        try {
            const logs = await this.connection.requireClient().logs.list();
            return logs.slice(-100).reverse().map((log) => {
                const item = new vscode.TreeItem(`[${log.level}] ${log.message}`);
                item.description = log.module;
                item.tooltip = `${log.timestamp}\n${log.message}`;
                item.iconPath = new vscode.ThemeIcon(logIcon(log.level));
                return item;
            });
        }
        catch {
            return [new vscode.TreeItem("Failed to load logs")];
        }
    }
}
exports.LogsTreeProvider = LogsTreeProvider;
// ─── helpers ──────────────────────────────────────────────────────────────────
function jobIcon(status) {
    switch (status) {
        case "running":
            return "sync~spin";
        case "completed":
            return "pass-filled";
        case "failed":
            return "error";
        case "cancelled":
            return "circle-slash";
        default:
            return "clock";
    }
}
function logIcon(level) {
    switch (level) {
        case "error":
            return "error";
        case "warn":
            return "warning";
        case "debug":
        case "trace":
            return "debug";
        default:
            return "info";
    }
}
function formatBytes(bytes) {
    if (!bytes)
        return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}
//# sourceMappingURL=trees.js.map