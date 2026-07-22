import { create } from "zustand";

import { PluginApi } from "@/api";
import type { Plugin, PluginUpdateCheck } from "@/types";

interface PluginsState {
  plugins: Plugin[];
  isLoading: boolean;
  error: string | null;
  actionError: string | null;
  updateChecks: Record<string, PluginUpdateCheck>;
  fetchPlugins: () => Promise<void>;
  setEnabled: (id: string, enabled: boolean) => Promise<boolean>;
  install: (sourcePath: string) => Promise<boolean>;
  uninstall: (id: string) => Promise<boolean>;
  checkUpdate: (id: string) => Promise<PluginUpdateCheck | null>;
  clearActionError: () => void;
}

export const usePluginsStore = create<PluginsState>((set, get) => ({
  plugins: [],
  isLoading: false,
  error: null,
  actionError: null,
  updateChecks: {},
  clearActionError: () => set({ actionError: null }),
  fetchPlugins: async () => {
    set({ isLoading: true, error: null });
    try {
      const plugins = await PluginApi.list();
      set({ plugins, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : "Failed to load plugins",
      });
    }
  },
  setEnabled: async (id, enabled) => {
    set({ actionError: null });
    try {
      await PluginApi.setEnabled(id, enabled);
      await get().fetchPlugins();
      return true;
    } catch (error) {
      set({
        actionError:
          error instanceof Error ? error.message : "Failed to update plugin",
      });
      return false;
    }
  },
  install: async (sourcePath) => {
    set({ actionError: null });
    try {
      await PluginApi.install(sourcePath);
      await get().fetchPlugins();
      return true;
    } catch (error) {
      set({
        actionError:
          error instanceof Error ? error.message : "Failed to install plugin",
      });
      return false;
    }
  },
  uninstall: async (id) => {
    set({ actionError: null });
    try {
      await PluginApi.uninstall(id);
      await get().fetchPlugins();
      return true;
    } catch (error) {
      set({
        actionError:
          error instanceof Error ? error.message : "Failed to uninstall plugin",
      });
      return false;
    }
  },
  checkUpdate: async (id) => {
    set({ actionError: null });
    try {
      const result = await PluginApi.checkUpdate(id);
      set((state) => ({
        updateChecks: { ...state.updateChecks, [id]: result },
      }));
      return result;
    } catch (error) {
      set({
        actionError:
          error instanceof Error ? error.message : "Failed to check for updates",
      });
      return null;
    }
  },
}));
