import type { StreamEvent } from "./types";

export type HttpMethod = "GET" | "POST" | "PUT" | "DELETE" | "PATCH";

/** Error thrown for any failed API interaction. */
export class ClusterError extends Error {
  constructor(
    message: string,
    readonly status?: number,
    readonly cause?: unknown,
  ) {
    super(message);
    this.name = "ClusterError";
  }
}

export interface EventStreamListeners {
  onEvent: (event: StreamEvent) => void;
  onOpen?: () => void;
  onClose?: (err?: Error) => void;
}

/** Handle to an open event stream; call `close()` to unsubscribe. */
export interface EventStreamHandle {
  close(): void;
}

/**
 * Transport abstraction so the API surface is decoupled from HTTP.
 * Mirrors the Python SDK's injectable-bridge pattern; a future in-process
 * or stdio transport can implement the same interface.
 */
export interface Transport {
  request<T>(method: HttpMethod, path: string, body?: unknown): Promise<T>;
  openEvents(listeners: EventStreamListeners): EventStreamHandle;
}
