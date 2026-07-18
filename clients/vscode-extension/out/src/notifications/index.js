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
exports.notifyJobEvent = notifyJobEvent;
exports.notifyRuntimeUnavailable = notifyRuntimeUnavailable;
exports.notifyConnected = notifyConnected;
const vscode = __importStar(require("vscode"));
const settings_1 = require("../settings");
/** Show job lifecycle notifications honoring the user's preference. */
function notifyJobEvent(kind, message) {
    const pref = (0, settings_1.getSettings)().notifications;
    if (pref === "none")
        return;
    if (pref === "failuresOnly" && kind !== "failed")
        return;
    if (kind === "failed") {
        void vscode.window.showErrorMessage(message);
    }
    else {
        void vscode.window.showInformationMessage(message);
    }
}
function notifyRuntimeUnavailable(detail) {
    const base = "Cluster Runtime is not available. Start the desktop app to connect.";
    void vscode.window
        .showErrorMessage(detail ? `${base} (${detail})` : base, "Open Desktop App")
        .then((choice) => {
        if (choice === "Open Desktop App") {
            void vscode.commands.executeCommand("clusterRuntime.openDesktop");
        }
    });
}
function notifyConnected(scheduler, workers) {
    if ((0, settings_1.getSettings)().notifications === "none")
        return;
    void vscode.window.showInformationMessage(`Connected to Cluster Runtime (${scheduler}, ${workers} worker${workers === 1 ? "" : "s"}).`);
}
//# sourceMappingURL=index.js.map