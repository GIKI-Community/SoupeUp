import {
  ClusterClient,
  type ClusterOverview,
  type EventStreamHandle,
  type StreamEvent,
} from "@cluster-runtime/client";
import * as vscode from "vscode";

import { notifyConnected, notifyRuntimeUnavailable } from "../notifications";
import { getSettings, schedulerAliasToPluginId } from "../settings";

export type ConnectionState = "disconnected" | "connecting" | "connected";

/**
 * Owns the ClusterClient lifecycle: auto-discovery, connect/reconnect,
 * the live event stream, and a cached cluster overview. Everything else in
 * the extension observes this service rather than talking to the API directly.
 */
export class ConnectionService implements vscode.Disposable {
  private client: ClusterClient | null = null;
  private stream: EventStreamHandle | null = null;
  private _state: ConnectionState = "disconnected";
  private overview: ClusterOverview | undefined;
  private discoveryTimer: ReturnType<typeof setInterval> | undefined;
  private refreshTimer: ReturnType<typeof setInterval> | undefined;

  private readonly _onDidChangeState = new vscode.EventEmitter<ConnectionState>();
  readonly onDidChangeState = this._onDidChangeState.event;
  private readonly _onEvent = new vscode.EventEmitter<StreamEvent>();
  readonly onEvent = this._onEvent.event;

  get state(): ConnectionState {
    return this._state;
  }

  get clusterOverview(): ClusterOverview | undefined {
    return this.overview;
  }

  isConnected(): boolean {
    return this._state === "connected" && this.client !== null;
  }

  /** Returns the connected client or throws a user-facing error. */
  requireClient(): ClusterClient {
    if (!this.client) {
      throw new Error("Not connected to Cluster Runtime.");
    }
    return this.client;
  }

  /**
   * Attempt to discover and connect. When `silent`, failures do not raise a
   * notification (used for background auto-connect attempts).
   */
  async connect(silent = false): Promise<boolean> {
    if (this._state === "connecting") return false;
    this.setState("connecting");

    try {
      const client = await ClusterClient.connect();
      await client.health();
      this.client = client;
      this.setState("connected");

      await this.applyDefaultScheduler();
      await this.refreshOverview();
      this.openStream();
      this.startRefreshLoop();
      this.stopDiscoveryLoop();

      const ov = this.overview;
      notifyConnected(ov?.activeScheduler ?? "unknown", ov?.workerCount ?? 0);
      return true;
    } catch (err) {
      this.client = null;
      this.setState("disconnected");
      if (!silent) {
        notifyRuntimeUnavailable(err instanceof Error ? err.message : String(err));
      }
      return false;
    }
  }

  disconnect(): void {
    this.stream?.close();
    this.stream = null;
    this.client = null;
    this.overview = undefined;
    this.stopRefreshLoop();
    this.setState("disconnected");
  }

  /** Start polling for a runtime becoming available (auto-connect). */
  startAutoConnect(): void {
    if (!getSettings().autoConnect) return;
    void this.connect(true);
    this.discoveryTimer = setInterval(() => {
      if (this._state === "disconnected") {
        void this.connect(true);
      }
    }, 5000);
  }

  async refreshOverview(): Promise<void> {
    if (!this.client) return;
    try {
      this.overview = await this.client.cluster.overview();
      this._onDidChangeState.fire(this._state);
    } catch {
      // A failed refresh likely means the runtime went away; let the stream
      // close handler drive reconnection.
    }
  }

  private async applyDefaultScheduler(): Promise<void> {
    const alias = getSettings().defaultScheduler;
    if (!alias || !this.client) return;
    const pluginId = schedulerAliasToPluginId(alias);
    if (!pluginId) return;
    try {
      await this.client.schedulers.setActive(pluginId);
    } catch {
      // Non-fatal: keep the runtime's current scheduler.
    }
  }

  private openStream(): void {
    if (!this.client) return;
    this.stream?.close();
    this.stream = this.client.onEvent(
      (event) => {
        this._onEvent.fire(event);
        if (event.type === "status") {
          void this.refreshOverview();
        }
      },
      {
        onClose: () => {
          // Desktop went away; drop to disconnected and let auto-connect retry.
          this.disconnect();
          this.startAutoConnect();
        },
      },
    );
  }

  private startRefreshLoop(): void {
    this.stopRefreshLoop();
    this.refreshTimer = setInterval(() => void this.refreshOverview(), 4000);
  }

  private stopRefreshLoop(): void {
    if (this.refreshTimer) clearInterval(this.refreshTimer);
    this.refreshTimer = undefined;
  }

  private stopDiscoveryLoop(): void {
    if (this.discoveryTimer) clearInterval(this.discoveryTimer);
    this.discoveryTimer = undefined;
  }

  private setState(state: ConnectionState): void {
    this._state = state;
    this._onDidChangeState.fire(state);
  }

  dispose(): void {
    this.stopDiscoveryLoop();
    this.stopRefreshLoop();
    this.stream?.close();
    this._onDidChangeState.dispose();
    this._onEvent.dispose();
  }
}
