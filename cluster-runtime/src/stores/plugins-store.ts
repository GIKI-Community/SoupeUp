import { create } from "zustand";

import { PluginApi } from "@/api";
import type { Plugin } from "@/types";

interface PluginsState {
  plugins: Plugin[];
  isLoading: boolean;
  error: string | null;
  fetchPlugins: () => Promise<void>;
}

export const usePluginsStore = create<PluginsState>((set) => ({
  plugins: [],
  isLoading: false,
  error: null,
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
}));
