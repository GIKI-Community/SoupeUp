import { create } from "zustand";

import { SystemApi } from "@/api";
import type { ActivityEntry, SystemInfo, SystemStatus } from "@/types";

interface SystemState {
  info: SystemInfo | null;
  status: SystemStatus | null;
  activity: ActivityEntry[];
  isLoading: boolean;
  error: string | null;
  fetchAll: () => Promise<void>;
}

export const useSystemStore = create<SystemState>((set) => ({
  info: null,
  status: null,
  activity: [],
  isLoading: false,
  error: null,
  fetchAll: async () => {
    set({ isLoading: true, error: null });
    try {
      const [info, status, activity] = await Promise.all([
        SystemApi.getInfo(),
        SystemApi.getStatus(),
        SystemApi.getActivity(),
      ]);
      set({ info, status, activity, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : "Failed to load system data",
      });
    }
  },
}));
