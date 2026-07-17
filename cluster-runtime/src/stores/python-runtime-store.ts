import { create } from "zustand";

import { PythonApi } from "@/api";
import type {
  ExecutionResult,
  PackageInfo,
  PythonRuntimeHealth,
} from "@/types";

interface PythonRuntimeState {
  health: PythonRuntimeHealth | null;
  packages: PackageInfo[];
  packageIndex: string | null;
  isExecuting: boolean;
  isInstalling: boolean;
  isLoading: boolean;
  lastResult: ExecutionResult | null;
  error: string | null;
  fetchHealth: () => Promise<void>;
  fetchPackages: () => Promise<void>;
  fetchPackageIndex: () => Promise<void>;
  executeCode: (code: string) => Promise<ExecutionResult | null>;
  installPackage: (name: string, version?: string) => Promise<boolean>;
  uninstallPackage: (name: string) => Promise<boolean>;
}

export const usePythonRuntimeStore = create<PythonRuntimeState>((set, get) => ({
  health: null,
  packages: [],
  packageIndex: null,
  isExecuting: false,
  isInstalling: false,
  isLoading: false,
  lastResult: null,
  error: null,

  fetchHealth: async () => {
    try {
      const health = await PythonApi.runtimeHealth();
      set({ health, error: null });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch Python health",
      });
    }
  },

  fetchPackages: async () => {
    set({ isLoading: true, error: null });
    try {
      const packages = await PythonApi.listPackages();
      set({ packages, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        error:
          error instanceof Error ? error.message : "Failed to list packages",
      });
    }
  },

  fetchPackageIndex: async () => {
    try {
      const packageIndex = await PythonApi.packageIndex();
      set({ packageIndex });
    } catch {
      set({ packageIndex: "https://pypi.org/simple" });
    }
  },

  executeCode: async (code: string) => {
    set({ isExecuting: true, error: null, lastResult: null });
    try {
      const lastResult = await PythonApi.executeCode(code);
      set({ lastResult, isExecuting: false });
      return lastResult;
    } catch (error) {
      set({
        isExecuting: false,
        error:
          error instanceof Error ? error.message : "Code execution failed",
      });
      return null;
    }
  },

  installPackage: async (name: string, version?: string) => {
    set({ isInstalling: true, error: null });
    try {
      await PythonApi.installPackage(name, version);
      await get().fetchPackages();
      set({ isInstalling: false });
      return true;
    } catch (error) {
      set({
        isInstalling: false,
        error:
          error instanceof Error ? error.message : "Package install failed",
      });
      return false;
    }
  },

  uninstallPackage: async (name: string) => {
    set({ isInstalling: true, error: null });
    try {
      await PythonApi.uninstallPackage(name);
      await get().fetchPackages();
      set({ isInstalling: false });
      return true;
    } catch (error) {
      set({
        isInstalling: false,
        error:
          error instanceof Error ? error.message : "Package uninstall failed",
      });
      return false;
    }
  },
}));
