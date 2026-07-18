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
exports.RAY_PLUGIN_ID = exports.DASK_PLUGIN_ID = void 0;
exports.getSettings = getSettings;
exports.schedulerAliasToPluginId = schedulerAliasToPluginId;
const vscode = __importStar(require("vscode"));
exports.DASK_PLUGIN_ID = "plugin-dask-scheduler";
exports.RAY_PLUGIN_ID = "plugin-ray";
function getSettings() {
    const cfg = vscode.workspace.getConfiguration("clusterRuntime");
    return {
        autoConnect: cfg.get("autoConnect", true),
        defaultScheduler: cfg.get("defaultScheduler", ""),
        watchFileChanges: cfg.get("watchFileChanges", false),
        openDashboardAfterSubmission: cfg.get("openDashboardAfterSubmission", false),
        notifications: cfg.get("notifications", "all"),
    };
}
/** Map a short scheduler alias (dask/ray) to its backend plugin id. */
function schedulerAliasToPluginId(alias) {
    const normalized = alias.trim().toLowerCase();
    if (normalized === "dask" || normalized === exports.DASK_PLUGIN_ID)
        return exports.DASK_PLUGIN_ID;
    if (normalized === "ray" || normalized === exports.RAY_PLUGIN_ID)
        return exports.RAY_PLUGIN_ID;
    return undefined;
}
//# sourceMappingURL=index.js.map