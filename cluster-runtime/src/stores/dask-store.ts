import { create } from "zustand";

import { DaskApi } from "@/api";
import {
  DASK_EXAMPLES,
  exampleErrorMessage,
  type ClusterSnapshot,
  type DashboardView,
  type DaskMetrics,
  type DaskSettings,
  type ExampleJobResult,
} from "@/types";

import { useJobsStore } from "./jobs-store";

interface DaskState {
  snapshot: ClusterSnapshot | null;
  settings: DaskSettings | null;
  dashboard: DashboardView | null;
  metrics: DaskMetrics | null;
  lastExample: ExampleJobResult | null;
  isLoading: boolean;
  isBusy: boolean;
  schedulerBusy: boolean;
  workerBusy: boolean;
  isRunningExample: boolean;
  error: string | null;
  joinAddress: string;
  fetchSnapshot: () => Promise<void>;
  fetchSettings: () => Promise<void>;
  fetchDashboard: () => Promise<void>;
  fetchMetrics: () => Promise<void>;
  saveSettings: (settings: DaskSettings) => Promise<boolean>;
  startScheduler: () => Promise<boolean>;
  stopScheduler: () => Promise<boolean>;
  restartScheduler: () => Promise<boolean>;
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

export const useDaskStore = create<DaskState>((set, get) => ({
  snapshot: null,
  settings: null,
  dashboard: null,
  metrics: null,
  lastExample: null,
  isLoading: false,
  isBusy: false,
  schedulerBusy: false,
  workerBusy: false,
  isRunningExample: false,
  error: null,
  joinAddress: "tcp://127.0.0.1:8786",

  setJoinAddress: (joinAddress) => set({ joinAddress }),

  fetchSnapshot: async () => {
    try {
      const snapshot = await DaskApi.clusterSnapshot();
      set({ snapshot, error: null });
    } catch (error) {
      // Don't overwrite a successful worker/scheduler action with a polling error.
      const msg = errMessage(error, "Failed to fetch cluster snapshot");
      set((state) => ({
        error: state.snapshot ? null : msg,
      }));
    }
  },

  fetchSettings: async () => {
    try {
      const settings = await DaskApi.getSettings();
      set({
        settings,
        joinAddress: settings.schedulerAddress || get().joinAddress,
        error: null,
      });
    } catch (error) {
      set({ error: errMessage(error, "Failed to fetch Dask settings") });
    }
  },

  fetchDashboard: async () => {
    try {
      const dashboard = await DaskApi.dashboard();
      set({ dashboard, error: null });
    } catch (error) {
      set({ error: errMessage(error, "Failed to fetch dashboard info") });
    }
  },

  fetchMetrics: async () => {
    try {
      const metrics = await DaskApi.metrics();
      set({ metrics, error: null });
    } catch (error) {
      set({ error: errMessage(error, "Failed to fetch Dask metrics") });
    }
  },

  saveSettings: async (settings) => {
    set({ isBusy: true, error: null });
    try {
      const saved = await DaskApi.updateSettings(settings);
      set({ settings: saved, isBusy: false, joinAddress: saved.schedulerAddress });
      return true;
    } catch (error) {
      set({
        isBusy: false,
        error: errMessage(error, "Failed to save settings"),
      });
      return false;
    }
  },

  startScheduler: async () => {
    set({ schedulerBusy: true, error: null });
    try {
      await DaskApi.startScheduler();
      await get().fetchSnapshot();
      await get().fetchDashboard();
      set({ schedulerBusy: false });
      return true;
    } catch (error) {
      await get().fetchSnapshot();
      set({
        schedulerBusy: false,
        error: errMessage(error, "Failed to start scheduler"),
      });
      return false;
    }
  },

  stopScheduler: async () => {
    set({ schedulerBusy: true, error: null });
    try {
      await DaskApi.stopScheduler();
      await get().fetchSnapshot();
      set({ schedulerBusy: false });
      return true;
    } catch (error) {
      set({
        schedulerBusy: false,
        error: errMessage(error, "Failed to stop scheduler"),
      });
      return false;
    }
  },

  restartScheduler: async () => {
    set({ schedulerBusy: true, error: null });
    try {
      await DaskApi.restartScheduler();
      await get().fetchSnapshot();
      set({ schedulerBusy: false });
      return true;
    } catch (error) {
      set({
        schedulerBusy: false,
        error: errMessage(error, "Failed to restart scheduler"),
      });
      return false;
    }
  },

  startWorker: async (address) => {
    set({ workerBusy: true, error: null });
    try {
      const addr = address ?? get().joinAddress;
      await DaskApi.startWorker(addr);
      await get().fetchSnapshot();
      set({ workerBusy: false });
      return true;
    } catch (error) {
      await get().fetchSnapshot();
      set({
        workerBusy: false,
        error: errMessage(error, "Failed to start worker"),
      });
      return false;
    }
  },

  stopWorker: async () => {
    set({ workerBusy: true, error: null });
    try {
      await DaskApi.stopWorker();
      await get().fetchSnapshot();
      set({ workerBusy: false });
      return true;
    } catch (error) {
      set({
        workerBusy: false,
        error: errMessage(error, "Failed to stop worker"),
      });
      return false;
    }
  },

  restartWorker: async () => {
    set({ workerBusy: true, error: null });
    try {
      await DaskApi.restartWorker();
      await get().fetchSnapshot();
      set({ workerBusy: false });
      return true;
    } catch (error) {
      set({
        workerBusy: false,
        error: errMessage(error, "Failed to restart worker"),
      });
      return false;
    }
  },

  ensurePackages: async () => {
    set({ isBusy: true, error: null });
    try {
      await DaskApi.ensurePackages();
      set({ isBusy: false });
      return true;
    } catch (error) {
      set({
        isBusy: false,
        error: errMessage(error, "Failed to install Dask packages"),
      });
      return false;
    }
  },

  runExample: async (exampleId) => {
    const title =
      DASK_EXAMPLES.find((ex) => ex.id === exampleId)?.title ?? exampleId;
    set({ isRunningExample: true, error: null, lastExample: null });
    try {
      const lastExample = await DaskApi.runExample(exampleId);
      const failureMessage = lastExample.success
        ? null
        : exampleErrorMessage(lastExample);
      set({
        lastExample,
        isRunningExample: false,
        error: failureMessage,
      });
      await get().fetchSnapshot();
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
