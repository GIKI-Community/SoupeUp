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
exports.ConnectionService = void 0;
const client_1 = require("@cluster-runtime/client");
const vscode = __importStar(require("vscode"));
const notifications_1 = require("../notifications");
const settings_1 = require("../settings");
/**
 * Owns the ClusterClient lifecycle: auto-discovery, connect/reconnect,
 * the live event stream, and a cached cluster overview. Everything else in
 * the extension observes this service rather than talking to the API directly.
 */
class ConnectionService {
    constructor() {
        this.client = null;
        this.stream = null;
        this._state = "disconnected";
        this._onDidChangeState = new vscode.EventEmitter();
        this.onDidChangeState = this._onDidChangeState.event;
        this._onEvent = new vscode.EventEmitter();
        this.onEvent = this._onEvent.event;
    }
    get state() {
        return this._state;
    }
    get clusterOverview() {
        return this.overview;
    }
    isConnected() {
        return this._state === "connected" && this.client !== null;
    }
    /** Returns the connected client or throws a user-facing error. */
    requireClient() {
        if (!this.client) {
            throw new Error("Not connected to Cluster Runtime.");
        }
        return this.client;
    }
    /**
     * Attempt to discover and connect. When `silent`, failures do not raise a
     * notification (used for background auto-connect attempts).
     */
    async connect(silent = false) {
        if (this._state === "connecting")
            return false;
        this.setState("connecting");
        try {
            const client = await client_1.ClusterClient.connect();
            await client.health();
            this.client = client;
            this.setState("connected");
            await this.applyDefaultScheduler();
            await this.refreshOverview();
            this.openStream();
            this.startRefreshLoop();
            this.stopDiscoveryLoop();
            const ov = this.overview;
            (0, notifications_1.notifyConnected)(ov?.activeScheduler ?? "unknown", ov?.workerCount ?? 0);
            return true;
        }
        catch (err) {
            this.client = null;
            this.setState("disconnected");
            if (!silent) {
                (0, notifications_1.notifyRuntimeUnavailable)(err instanceof Error ? err.message : String(err));
            }
            return false;
        }
    }
    disconnect() {
        this.stream?.close();
        this.stream = null;
        this.client = null;
        this.overview = undefined;
        this.stopRefreshLoop();
        this.setState("disconnected");
    }
    /** Start polling for a runtime becoming available (auto-connect). */
    startAutoConnect() {
        if (!(0, settings_1.getSettings)().autoConnect)
            return;
        void this.connect(true);
        this.discoveryTimer = setInterval(() => {
            if (this._state === "disconnected") {
                void this.connect(true);
            }
        }, 5000);
    }
    async refreshOverview() {
        if (!this.client)
            return;
        try {
            this.overview = await this.client.cluster.overview();
            this._onDidChangeState.fire(this._state);
        }
        catch {
            // A failed refresh likely means the runtime went away; let the stream
            // close handler drive reconnection.
        }
    }
    async applyDefaultScheduler() {
        const alias = (0, settings_1.getSettings)().defaultScheduler;
        if (!alias || !this.client)
            return;
        const pluginId = (0, settings_1.schedulerAliasToPluginId)(alias);
        if (!pluginId)
            return;
        try {
            await this.client.schedulers.setActive(pluginId);
        }
        catch {
            // Non-fatal: keep the runtime's current scheduler.
        }
    }
    openStream() {
        if (!this.client)
            return;
        this.stream?.close();
        this.stream = this.client.onEvent((event) => {
            this._onEvent.fire(event);
            if (event.type === "status") {
                void this.refreshOverview();
            }
        }, {
            onClose: () => {
                // Desktop went away; drop to disconnected and let auto-connect retry.
                this.disconnect();
                this.startAutoConnect();
            },
        });
    }
    startRefreshLoop() {
        this.stopRefreshLoop();
        this.refreshTimer = setInterval(() => void this.refreshOverview(), 4000);
    }
    stopRefreshLoop() {
        if (this.refreshTimer)
            clearInterval(this.refreshTimer);
        this.refreshTimer = undefined;
    }
    stopDiscoveryLoop() {
        if (this.discoveryTimer)
            clearInterval(this.discoveryTimer);
        this.discoveryTimer = undefined;
    }
    setState(state) {
        this._state = state;
        this._onDidChangeState.fire(state);
    }
    dispose() {
        this.stopDiscoveryLoop();
        this.stopRefreshLoop();
        this.stream?.close();
        this._onDidChangeState.dispose();
        this._onEvent.dispose();
    }
}
exports.ConnectionService = ConnectionService;
//# sourceMappingURL=connection.js.map