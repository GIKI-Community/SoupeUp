import { create } from "zustand";

import { MetricsApi } from "@/api";
import type { MetricPoint, MetricsSnapshot } from "@/types";

import { useDaskStore } from "./dask-store";

const MAX_POINTS = 30;

interface MetricsState {
  snapshot: MetricsSnapshot | null;
  initialized: boolean;
  isLoading: boolean;
  error: string | null;
  fetchMetrics: () => Promise<void>;
  appendAnimatedPoint: () => void;
}

function appendPoint(points: MetricPoint[], newValue: number): MetricPoint[] {
  const next: MetricPoint[] = [
    ...points,
    { timestamp: new Date().toISOString(), value: Math.round(newValue * 10) / 10 },
  ];
  return next.length > MAX_POINTS ? next.slice(next.length - MAX_POINTS) : next;
}

function nextValue(
  points: MetricPoint[],
  daskValue: number | undefined,
  fallbackJitterBase: number,
  variance: number,
): number {
  if (daskValue != null) return daskValue;
  if (points.length === 0) return 0;
  return Math.max(
    0,
    fallbackJitterBase + (Math.random() - 0.5) * variance * 2,
  );
}

export const useMetricsStore = create<MetricsState>((set, get) => ({
  snapshot: null,
  initialized: false,
  isLoading: false,
  error: null,
  fetchMetrics: async () => {
    if (get().initialized) return;

    set({ isLoading: true, error: null });
    try {
      const snapshot = await MetricsApi.get();
      set({ snapshot, initialized: true, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        error:
          typeof error === "string"
            ? error
            : error instanceof Error
              ? error.message
              : "Failed to load metrics",
      });
    }
  },
  appendAnimatedPoint: () => {
    const { snapshot } = get();
    if (!snapshot) return;

    const dask = useDaskStore.getState().metrics;
    const lastCpu = snapshot.cpu.points.at(-1)?.value ?? 0;
    const lastMem = snapshot.memory.points.at(-1)?.value ?? 0;
    const lastNet = snapshot.network.points.at(-1)?.value ?? 0;
    const lastDisk = snapshot.disk.points.at(-1)?.value ?? 0;

    const cpuVal = nextValue(snapshot.cpu.points, dask?.workerCpu, lastCpu, 6);
    const memVal = nextValue(snapshot.memory.points, dask?.workerMemory, lastMem, 4);
    const netVal = nextValue(
      snapshot.network.points,
      dask != null ? dask.dataTransfer / (1024 * 1024) : undefined,
      lastNet,
      25,
    );
    const diskVal = nextValue(snapshot.disk.points, dask?.tasksPerSec, lastDisk, 12);

    set({
      snapshot: {
        ...snapshot,
        collectedAt: new Date().toISOString(),
        cpu: {
          ...snapshot.cpu,
          points: appendPoint(snapshot.cpu.points, cpuVal),
        },
        memory: {
          ...snapshot.memory,
          points: appendPoint(snapshot.memory.points, memVal),
        },
        network: {
          ...snapshot.network,
          points: appendPoint(snapshot.network.points, netVal),
        },
        disk: {
          ...snapshot.disk,
          points: appendPoint(snapshot.disk.points, diskVal),
        },
      },
    });
  },
}));
