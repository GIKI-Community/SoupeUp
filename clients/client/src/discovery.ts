import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";

/** Tauri bundle identifier (from cluster-runtime/src-tauri/tauri.conf.json). */
export const APP_IDENTIFIER = "dev.cluster-runtime.app";

export interface DiscoveredEndpoint {
  url: string;
  token: string;
  pid?: number;
}

/** Per-OS base data directory that mirrors Tauri's `app_data_dir()`. */
export function appDataDir(identifier: string = APP_IDENTIFIER): string {
  const home = os.homedir();
  switch (process.platform) {
    case "win32":
      return path.join(
        process.env.APPDATA ?? path.join(home, "AppData", "Roaming"),
        identifier,
      );
    case "darwin":
      return path.join(home, "Library", "Application Support", identifier);
    default:
      return path.join(
        process.env.XDG_DATA_HOME ?? path.join(home, ".local", "share"),
        identifier,
      );
  }
}

/** Absolute path to the endpoint discovery file written by the desktop app. */
export function endpointFilePath(identifier: string = APP_IDENTIFIER): string {
  return path.join(appDataDir(identifier), "api", "endpoint.json");
}

/**
 * Read the endpoint discovery file. Returns null if the runtime is not
 * running (file missing) or the file is unreadable/invalid.
 */
export async function discoverEndpoint(
  identifier: string = APP_IDENTIFIER,
): Promise<DiscoveredEndpoint | null> {
  const file = endpointFilePath(identifier);
  try {
    const raw = await fs.readFile(file, "utf8");
    const parsed = JSON.parse(raw) as Partial<DiscoveredEndpoint>;
    if (typeof parsed.url === "string" && typeof parsed.token === "string") {
      return { url: parsed.url, token: parsed.token, pid: parsed.pid };
    }
    return null;
  } catch {
    return null;
  }
}
