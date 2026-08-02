import { discoverEndpoint } from "./discovery";
import { HttpTransport } from "./http-transport";
import {
  ClusterError,
  type EventStreamHandle,
  type EventStreamListeners,
  type Transport,
} from "./transport";
import type {
  ClusterOverview,
  HealthResponse,
  Job,
  JobDetail,
  JobResult,
  JobSpec,
  LogEntry,
  Node,
  SchedulerListEntry,
  StreamEvent,
  SubmitAck,
  SystemOverview,
  DepsProbeResult,
  InstallPackagesResult,
  PackageInfo,
} from "./types";

export interface ConnectOptions {
  /** Provide an explicit endpoint instead of auto-discovering one. */
  url?: string;
  token?: string;
  timeoutMs?: number;
  autoReconnect?: boolean;
  /** Override the Tauri bundle identifier used for discovery. */
  identifier?: string;
}

/**
 * Typed client for the Cluster Runtime API. Namespaces mirror the desktop
 * frontend's api layer so behavior stays consistent across clients.
 */
export class ClusterClient {
  constructor(private readonly transport: Transport) {}

  /** Discover a running runtime and build a client, or throw if none found. */
  static async connect(options: ConnectOptions = {}): Promise<ClusterClient> {
    let url = options.url;
    let token = options.token;

    if (!url || !token) {
      const found = await discoverEndpoint(options.identifier);
      if (!found) {
        throw new ClusterError(
          "No running Cluster Runtime found. Start the desktop app and try again.",
        );
      }
      url = url ?? found.url;
      token = token ?? found.token;
    }

    const transport = new HttpTransport({
      url,
      token,
      timeoutMs: options.timeoutMs,
      autoReconnect: options.autoReconnect,
    });
    return new ClusterClient(transport);
  }

  /** Unauthenticated liveness probe. */
  health(): Promise<HealthResponse> {
    return this.transport.request<HealthResponse>("GET", "/health");
  }

  readonly system = {
    get: (): Promise<SystemOverview> =>
      this.transport.request("GET", "/v1/system"),
  };

  readonly schedulers = {
    list: (): Promise<SchedulerListEntry[]> =>
      this.transport.request("GET", "/v1/schedulers"),
    getActive: (): Promise<{ pluginId: string }> =>
      this.transport.request("GET", "/v1/schedulers/active"),
    setActive: (pluginId: string): Promise<{ pluginId: string }> =>
      this.transport.request("PUT", "/v1/schedulers/active", { pluginId }),
  };

  readonly cluster = {
    overview: (): Promise<ClusterOverview> =>
      this.transport.request("GET", "/v1/cluster"),
    nodes: (): Promise<Node[]> => this.transport.request("GET", "/v1/nodes"),
  };

  readonly jobs = {
    submit: (spec: JobSpec, owner?: string): Promise<SubmitAck> =>
      this.transport.request(
        "POST",
        `/v1/jobs${owner ? `?owner=${encodeURIComponent(owner)}` : ""}`,
        spec,
      ),
    list: (): Promise<Job[]> => this.transport.request("GET", "/v1/jobs"),
    get: (id: string): Promise<JobDetail> =>
      this.transport.request("GET", `/v1/jobs/${encodeURIComponent(id)}`),
    result: (id: string): Promise<JobResult> =>
      this.transport.request("GET", `/v1/jobs/${encodeURIComponent(id)}/result`),
    cancel: (id: string): Promise<{ cancelled: string }> =>
      this.transport.request("POST", `/v1/jobs/${encodeURIComponent(id)}/cancel`),
    retry: (id: string): Promise<SubmitAck> =>
      this.transport.request("POST", `/v1/jobs/${encodeURIComponent(id)}/retry`),
    /**
     * Poll until the job reaches a terminal status (completed/failed/cancelled).
     * Submit itself returns immediately; use this to wait for long-running work.
     */
    wait: async (
      id: string,
      opts: {
        pollIntervalMs?: number;
        /** Overall wait deadline (default: no limit). */
        timeoutMs?: number;
        signal?: AbortSignal;
        onUpdate?: (detail: JobDetail) => void;
      } = {},
    ): Promise<JobDetail> => {
      const pollIntervalMs = opts.pollIntervalMs ?? 1500;
      const deadline =
        opts.timeoutMs !== undefined ? Date.now() + opts.timeoutMs : Number.POSITIVE_INFINITY;
      const terminal = new Set(["completed", "failed", "cancelled"]);

      for (;;) {
        if (opts.signal?.aborted) {
          throw new ClusterError("Wait for job aborted");
        }
        if (Date.now() > deadline) {
          throw new ClusterError(
            `Timed out waiting for job ${id} after ${opts.timeoutMs}ms`,
          );
        }

        const detail = await this.jobs.get(id);
        opts.onUpdate?.(detail);
        if (terminal.has(detail.status)) {
          return detail;
        }

        await new Promise<void>((resolve, reject) => {
          const timer = setTimeout(resolve, pollIntervalMs);
          const onAbort = () => {
            clearTimeout(timer);
            reject(new ClusterError("Wait for job aborted"));
          };
          if (opts.signal) {
            if (opts.signal.aborted) {
              onAbort();
              return;
            }
            opts.signal.addEventListener("abort", onAbort, { once: true });
          }
        });
      }
    },
  };

  readonly logs = {
    list: (): Promise<LogEntry[]> => this.transport.request("GET", "/v1/logs"),
  };

  readonly python = {
    listPackages: (): Promise<{ packages: PackageInfo[] }> =>
      this.transport.request("GET", "/v1/python/packages"),
    probe: (body: {
      source?: string;
      modules?: string[];
      checkWorkers?: boolean;
    }): Promise<DepsProbeResult> =>
      this.transport.request("POST", "/v1/python/probe", body),
    installPackages: (body: {
      packages: string[];
      installOnWorkers?: boolean;
    }): Promise<InstallPackagesResult> =>
      this.transport.request("POST", "/v1/python/packages/install", body),
  };

  /** Subscribe to the live event stream. Returns a handle to unsubscribe. */
  onEvent(
    listener: (event: StreamEvent) => void,
    extra?: Omit<EventStreamListeners, "onEvent">,
  ): EventStreamHandle {
    return this.transport.openEvents({ onEvent: listener, ...extra });
  }
}
