import WebSocket from "ws";

import {
  ClusterError,
  type EventStreamHandle,
  type EventStreamListeners,
  type HttpMethod,
  type Transport,
} from "./transport";
import type { StreamEvent } from "./types";

export interface HttpTransportOptions {
  /** Base URL, e.g. http://127.0.0.1:8129 */
  url: string;
  /** Bearer token required for all /v1 routes. */
  token: string;
  /** Per-request timeout in ms (default 30000). */
  timeoutMs?: number;
  /** Reconnect the event stream automatically (default true). */
  autoReconnect?: boolean;
}

/** REST + WebSocket transport over the local Cluster Runtime HTTP API. */
export class HttpTransport implements Transport {
  private readonly url: string;
  private readonly token: string;
  private readonly timeoutMs: number;
  private readonly autoReconnect: boolean;

  constructor(opts: HttpTransportOptions) {
    this.url = opts.url.replace(/\/+$/, "");
    this.token = opts.token;
    this.timeoutMs = opts.timeoutMs ?? 30_000;
    this.autoReconnect = opts.autoReconnect ?? true;
  }

  async request<T>(method: HttpMethod, path: string, body?: unknown): Promise<T> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);

    let res: Response;
    try {
      res = await fetch(`${this.url}${path}`, {
        method,
        headers: {
          Authorization: `Bearer ${this.token}`,
          ...(body !== undefined ? { "Content-Type": "application/json" } : {}),
        },
        body: body !== undefined ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });
    } catch (err) {
      clearTimeout(timer);
      if (err instanceof Error && err.name === "AbortError") {
        throw new ClusterError(`Request timed out after ${this.timeoutMs}ms`, undefined, err);
      }
      throw new ClusterError(
        `Cannot reach Cluster Runtime at ${this.url}. Is the desktop app running?`,
        undefined,
        err,
      );
    } finally {
      clearTimeout(timer);
    }

    const text = await res.text();
    const parsed = text ? safeJson(text) : undefined;

    if (!res.ok) {
      const message =
        (parsed && typeof parsed === "object" && "error" in parsed
          ? String((parsed as { error: unknown }).error)
          : undefined) ?? `HTTP ${res.status} ${res.statusText}`;
      throw new ClusterError(message, res.status);
    }

    return parsed as T;
  }

  openEvents(listeners: EventStreamListeners): EventStreamHandle {
    const wsUrl = `${this.url.replace(/^http/, "ws")}/v1/events`;
    let closedByUser = false;
    let socket: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

    const connect = () => {
      socket = new WebSocket(wsUrl, {
        headers: { Authorization: `Bearer ${this.token}` },
      });

      socket.on("open", () => listeners.onOpen?.());

      socket.on("message", (data: WebSocket.RawData) => {
        const frame = safeJson(data.toString());
        if (frame && typeof frame === "object" && "type" in frame) {
          listeners.onEvent(frame as StreamEvent);
        }
      });

      socket.on("error", () => {
        // Surfaced via close.
      });

      socket.on("close", () => {
        if (closedByUser) {
          listeners.onClose?.();
          return;
        }
        if (this.autoReconnect) {
          reconnectTimer = setTimeout(connect, 2000);
        } else {
          listeners.onClose?.();
        }
      });
    };

    connect();

    return {
      close: () => {
        closedByUser = true;
        if (reconnectTimer) clearTimeout(reconnectTimer);
        socket?.close();
      },
    };
  }
}

function safeJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}
