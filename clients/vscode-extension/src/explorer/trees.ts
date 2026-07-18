import {
  schedulerDisplayName,
  type Job,
  type JobStatus,
} from "@cluster-runtime/client";
import * as vscode from "vscode";

import type { ConnectionService } from "../services/connection";

const FINISHED: JobStatus[] = ["completed", "failed", "cancelled"];

/** Base provider that refreshes whenever the connection state or events change. */
abstract class LiveTreeProvider<T> implements vscode.TreeDataProvider<T> {
  private readonly _onDidChangeTreeData = new vscode.EventEmitter<
    T | undefined | void
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(protected readonly connection: ConnectionService) {
    connection.onDidChangeState(() => this.refresh());
    connection.onEvent(() => this.refresh());
  }

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  abstract getTreeItem(element: T): vscode.TreeItem;
  abstract getChildren(element?: T): vscode.ProviderResult<T[]>;

  protected offlineItem(): vscode.TreeItem {
    const item = new vscode.TreeItem("Not connected");
    item.iconPath = new vscode.ThemeIcon("circle-slash");
    item.command = { command: "clusterRuntime.connect", title: "Connect" };
    return item;
  }
}

// ─── Cluster ────────────────────────────────────────────────────────────────

export class ClusterTreeProvider extends LiveTreeProvider<vscode.TreeItem> {
  getTreeItem(element: vscode.TreeItem): vscode.TreeItem {
    return element;
  }

  getChildren(): vscode.TreeItem[] {
    if (!this.connection.isConnected()) return [this.offlineItem()];
    const ov = this.connection.clusterOverview;
    if (!ov) return [new vscode.TreeItem("Loading…")];

    const line = (label: string, value: string, icon: string): vscode.TreeItem => {
      const item = new vscode.TreeItem(`${label}: ${value}`);
      item.iconPath = new vscode.ThemeIcon(icon);
      return item;
    };

    return [
      line("Scheduler", schedulerDisplayName(ov.activeScheduler), "server-process"),
      line("Status", ov.schedulerRunning ? "running" : "stopped", "pulse"),
      line("Health", ov.health, "heart"),
      line("Workers", String(ov.workerCount), "organization"),
      line("Cores", String(ov.totalCores), "chip"),
      line("Memory", formatBytes(ov.totalMemory), "database"),
    ];
  }
}

// ─── Jobs ─────────────────────────────────────────────────────────────────────

export class JobTreeItem extends vscode.TreeItem {
  constructor(readonly job: Job) {
    super(job.name || job.id, vscode.TreeItemCollapsibleState.None);
    this.id = job.id;
    this.description = `${job.status} · ${job.schedulerId}`;
    this.tooltip = `${job.name}\nid: ${job.id}\nstatus: ${job.status}\nowner: ${job.owner}`;
    this.iconPath = new vscode.ThemeIcon(jobIcon(job.status));
    this.contextValue = FINISHED.includes(job.status)
      ? "job-finished"
      : "job-running";
  }
}

export class JobsTreeProvider extends LiveTreeProvider<vscode.TreeItem> {
  getTreeItem(element: vscode.TreeItem): vscode.TreeItem {
    return element;
  }

  async getChildren(): Promise<vscode.TreeItem[]> {
    if (!this.connection.isConnected()) return [this.offlineItem()];
    try {
      const jobs = await this.connection.requireClient().jobs.list();
      if (jobs.length === 0) {
        return [new vscode.TreeItem("No jobs yet")];
      }
      return jobs
        .slice()
        .sort((a, b) => b.submittedAt.localeCompare(a.submittedAt))
        .map((job) => new JobTreeItem(job));
    } catch {
      return [new vscode.TreeItem("Failed to load jobs")];
    }
  }
}

// ─── Workers ───────────────────────────────────────────────────────────────

export class WorkersTreeProvider extends LiveTreeProvider<vscode.TreeItem> {
  getTreeItem(element: vscode.TreeItem): vscode.TreeItem {
    return element;
  }

  async getChildren(): Promise<vscode.TreeItem[]> {
    if (!this.connection.isConnected()) return [this.offlineItem()];
    try {
      const nodes = await this.connection.requireClient().cluster.nodes();
      if (nodes.length === 0) return [new vscode.TreeItem("No workers")];
      return nodes.map((node) => {
        const item = new vscode.TreeItem(node.name);
        item.description = `${node.status} · cpu ${Math.round(node.cpuPercent)}% · mem ${Math.round(node.memoryPercent)}%`;
        item.tooltip = `${node.backend} · ${node.platform} · v${node.version}`;
        item.iconPath = new vscode.ThemeIcon(
          node.status === "online" ? "vm-active" : "vm-outline",
        );
        return item;
      });
    } catch {
      return [new vscode.TreeItem("Failed to load workers")];
    }
  }
}

// ─── Schedulers ────────────────────────────────────────────────────────────

export class SchedulersTreeProvider extends LiveTreeProvider<vscode.TreeItem> {
  getTreeItem(element: vscode.TreeItem): vscode.TreeItem {
    return element;
  }

  async getChildren(): Promise<vscode.TreeItem[]> {
    if (!this.connection.isConnected()) return [this.offlineItem()];
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
    } catch {
      return [new vscode.TreeItem("Failed to load schedulers")];
    }
  }
}

// ─── Logs ─────────────────────────────────────────────────────────────────────

export class LogsTreeProvider extends LiveTreeProvider<vscode.TreeItem> {
  getTreeItem(element: vscode.TreeItem): vscode.TreeItem {
    return element;
  }

  async getChildren(): Promise<vscode.TreeItem[]> {
    if (!this.connection.isConnected()) return [this.offlineItem()];
    try {
      const logs = await this.connection.requireClient().logs.list();
      return logs.slice(-100).reverse().map((log) => {
        const item = new vscode.TreeItem(`[${log.level}] ${log.message}`);
        item.description = log.module;
        item.tooltip = `${log.timestamp}\n${log.message}`;
        item.iconPath = new vscode.ThemeIcon(logIcon(log.level));
        return item;
      });
    } catch {
      return [new vscode.TreeItem("Failed to load logs")];
    }
  }
}

// ─── helpers ──────────────────────────────────────────────────────────────────

function jobIcon(status: JobStatus): string {
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

function logIcon(level: string): string {
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

function formatBytes(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}
