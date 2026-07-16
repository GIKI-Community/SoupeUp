import { create } from "zustand";

import { MetricsApi } from "@/api";
import type { MetricPoint, MetricsSnapshot } from "@/types";

interface MetricsState {
  snapshot: MetricsSnapshot | null;
  isLoading: boolean;
  error: string | null;
  fetchMetrics: () => Promise<void>;
  appendAnimatedPoint: () => void;
}

function shiftSeries(points: MetricPoint[], newValue: number): MetricPoint[] {
  const next = [
    ...points.slice(1),
    { timestamp: new Date().toISOString(), value: newValue },
  ];
  return next;
}

function jitter(base: number, variance: number): number {
  return Math.max(0, base + (Math.random() - 0.5) * variance * 2);
}

export const useMetricsStore = create<MetricsState>((set, get) => ({
  snapshot: null,
  isLoading: false,
  error: null,
  fetchMetrics: async () => {
    set({ isLoading: true, error: null });
    try {
      const snapshot = await MetricsApi.get();
      set({ snapshot, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : "Failed to load metrics",
      });
    }
  },
  appendAnimatedPoint: () => {
    const { snapshot } = get();
    if (!snapshot) return;

    const lastCpu = snapshot.cpu.points.at(-1)?.value ?? 40;
    const lastMem = snapshot.memory.points.at(-1)?.value ?? 55;
    const lastNet = snapshot.network.points.at(-1)?.value ?? 120;
    const lastDisk = snapshot.disk.points.at(-1)?.value ?? 45;

    set({
      snapshot: {
        ...snapshot,
        collectedAt: new Date().toISOString(),
        cpu: {
          ...snapshot.cpu,
          points: shiftSeries(snapshot.cpu.points, jitter(lastCpu, 6)),
        },
        memory: {
          ...snapshot.memory,
          points: shiftSeries(snapshot.memory.points, jitter(lastMem, 4)),
        },
        network: {
          ...snapshot.network,
          points: shiftSeries(snapshot.network.points, jitter(lastNet, 25)),
        },
        disk: {
          ...snapshot.disk,
          points: shiftSeries(snapshot.disk.points, jitter(lastDisk, 12)),
        },
      },
    });
  },
}));
