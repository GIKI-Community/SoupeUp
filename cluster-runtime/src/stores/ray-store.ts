import { create } from "zustand";

import { JobApi, RayApi, SchedulerApi } from "@/api";
import {
  RAY_EXAMPLES,
  exampleErrorMessage,
  type DashboardView,
  type ExampleJobResult,
  type RayClusterSnapshot,
  type RayMetrics,
  type RaySettings,
} from "@/types";

import { useJobsStore } from "./jobs-store";

interface RayState {
  snapshot: RayClusterSnapshot | null;
  settings: RaySettings | null;
  dashboard: DashboardView | null;
  metrics: RayMetrics | null;
  lastExample: ExampleJobResult | null;
  isLoading: boolean;
  isBusy: boolean;
  headBusy: boolean;
  workerBusy: boolean;
  isRunningExample: boolean;
  error: string | null;
  joinAddress: string;
  fetchSnapshot: () => Promise<void>;
  fetchSettings: () => Promise<void>;
  fetchDashboard: () => Promise<void>;
  fetchMetrics: () => Promise<void>;
  saveSettings: (settings: RaySettings) => Promise<boolean>;
  startHead: () => Promise<boolean>;
  stopHead: () => Promise<boolean>;
  restartHead: () => Promise<boolean>;
  startWorker: (address?: string) => Promise<boolean>;
  stopWorker: () => Promise<boolean>;
  restartWorker: () => Promise<boolean>;
  ensurePackages: () => Promise<boolean>;
  runExample: (exampleId: string) => Promise<ExampleJobResult | null>;
  setJoinAddress: (address: string) => void;
}

function errMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  if (typeof error === "object" && error !== null) {
    const record = error as Record<string, unknown>;
    if (typeof record.message === "string" && record.message.trim()) {
      return record.message;
    }
    if (typeof record.error === "string" && record.error.trim()) {
      return record.error;
    }
  }
  return fallback;
}

export const useRayStore = create<RayState>((set, get) => ({
  snapshot: null,
  settings: null,
  dashboard: null,
  metrics: null,
  lastExample: null,
  isLoading: false,
  isBusy: false,
  headBusy: false,
  workerBusy: false,
  isRunningExample: false,
  error: null,
  joinAddress: "127.0.0.1:6379",

  setJoinAddress: (joinAddress) => set({ joinAddress }),

  fetchSnapshot: async () => {
    try {
      const snapshot = await RayApi.clusterSnapshot();
      set({ snapshot, error: null });
    } catch (error) {
      const msg = errMessage(error, "Failed to fetch Ray cluster snapshot");
      set((state) => ({
        error: state.snapshot ? null : msg,
      }));
    }
  },

  fetchSettings: async () => {
    try {
      const settings = await RayApi.getSettings();
      set({
        settings,
        joinAddress: settings.headAddress || get().joinAddress,
        error: null,
      });
    } catch (error) {
      set({ error: errMessage(error, "Failed to fetch Ray settings") });
    }
  },

  fetchDashboard: async () => {
    try {
      const dashboard = await RayApi.dashboard();
      set({ dashboard, error: null });
    } catch (error) {
      set({ error: errMessage(error, "Failed to fetch Ray dashboard info") });
    }
  },

  fetchMetrics: async () => {
    try {
      const metrics = await RayApi.metrics();
      set({ metrics, error: null });
    } catch (error) {
      set({ error: errMessage(error, "Failed to fetch Ray metrics") });
    }
  },

  saveSettings: async (settings) => {
    set({ isBusy: true, error: null });
    try {
      const saved = await RayApi.updateSettings(settings);
      set({ settings: saved, isBusy: false, joinAddress: saved.headAddress });
      return true;
    } catch (error) {
      set({
        isBusy: false,
        error: errMessage(error, "Failed to save Ray settings"),
      });
      return false;
    }
  },

  startHead: async () => {
    set({ headBusy: true, error: null });
    try {
      await RayApi.startHead();
      await get().fetchSnapshot();
      await get().fetchDashboard();
      set({ headBusy: false });
      return true;
    } catch (error) {
      await get().fetchSnapshot();
      set({
        headBusy: false,
        error: errMessage(error, "Failed to start Ray head"),
      });
      return false;
    }
  },

  stopHead: async () => {
    set({ headBusy: true, error: null });
    try {
      await RayApi.stopHead();
      await get().fetchSnapshot();
      set({ headBusy: false });
      return true;
    } catch (error) {
      set({
        headBusy: false,
        error: errMessage(error, "Failed to stop Ray head"),
      });
      return false;
    }
  },

  restartHead: async () => {
    set({ headBusy: true, error: null });
    try {
      await RayApi.restartHead();
      await get().fetchSnapshot();
      set({ headBusy: false });
      return true;
    } catch (error) {
      set({
        headBusy: false,
        error: errMessage(error, "Failed to restart Ray head"),
      });
      return false;
    }
  },

  startWorker: async (address) => {
    set({ workerBusy: true, error: null });
    try {
      const addr = address ?? get().joinAddress;
      await RayApi.startWorker(addr);
      await get().fetchSnapshot();
      set({ workerBusy: false });
      return true;
    } catch (error) {
      await get().fetchSnapshot();
      set({
        workerBusy: false,
        error: errMessage(error, "Failed to start Ray worker"),
      });
      return false;
    }
  },

  stopWorker: async () => {
    set({ workerBusy: true, error: null });
    try {
      await RayApi.stopWorker();
      await get().fetchSnapshot();
      set({ workerBusy: false });
      return true;
    } catch (error) {
      set({
        workerBusy: false,
        error: errMessage(error, "Failed to stop Ray worker"),
      });
      return false;
    }
  },

  restartWorker: async () => {
    set({ workerBusy: true, error: null });
    try {
      await RayApi.restartWorker();
      await get().fetchSnapshot();
      set({ workerBusy: false });
      return true;
    } catch (error) {
      set({
        workerBusy: false,
        error: errMessage(error, "Failed to restart Ray worker"),
      });
      return false;
    }
  },

  ensurePackages: async () => {
    set({ isBusy: true, error: null });
    try {
      await RayApi.ensurePackages();
      set({ isBusy: false });
      return true;
    } catch (error) {
      set({
        isBusy: false,
        error: errMessage(error, "Failed to install Ray packages"),
      });
      return false;
    }
  },

  runExample: async (exampleId) => {
    const title =
      RAY_EXAMPLES.find((ex) => ex.id === exampleId)?.title ?? exampleId;
    set({ isRunningExample: true, error: null, lastExample: null });
    try {
      await SchedulerApi.setActive("plugin-ray");
      const ack = await JobApi.submitExample(exampleId, title);
      const result = await JobApi.result(ack.jobId);
      const lastExample: ExampleJobResult = {
        exampleId,
        title,
        success: result.status === "completed",
        executionTimeMs: result.metrics.executionTimeMs,
        workersUsed: result.metrics.workersUsed,
        cpuUtilization: result.metrics.cpuUtilization ?? null,
        speedup: result.metrics.speedup ?? null,
        resultSummary: result.resultSummary ?? "",
        details: result.output ?? null,
        error: result.errors[0] ?? null,
      };
      const failureMessage = lastExample.success
        ? null
        : exampleErrorMessage(lastExample);
      set({
        lastExample,
        isRunningExample: false,
        error: failureMessage,
      });
      await get().fetchSnapshot();
      useJobsStore.getState().fetchJobs();
      return lastExample;
    } catch (error) {
      const message = errMessage(error, "Example job failed");
      set({
        isRunningExample: false,
        error: message,
        lastExample: {
          exampleId,
          title,
          success: false,
          executionTimeMs: 0,
          workersUsed: 0,
          cpuUtilization: null,
          speedup: null,
          resultSummary: "",
          details: null,
          error: message,
        },
      });
      return null;
    } finally {
      void useJobsStore.getState().fetchJobs();
    }
  },
}));
