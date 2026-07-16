import { create } from "zustand";
import { persist } from "zustand/middleware";

import type { AppSettings } from "@/types";

const defaultSettings: AppSettings = {
  theme: "dark",
  accentColor: "#6366f1",
  language: "en",
  autoStart: false,
  telemetryEnabled: false,
  listenAddress: "127.0.0.1",
  port: 9470,
  enableMdns: true,
  enableRemote: false,
  authEnabled: false,
  tlsEnabled: false,
};

interface SettingsState {
  settings: AppSettings;
  updateSettings: (partial: Partial<AppSettings>) => void;
  resetSettings: () => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      settings: defaultSettings,
      updateSettings: (partial) =>
        set((state) => ({
          settings: { ...state.settings, ...partial },
        })),
      resetSettings: () => set({ settings: defaultSettings }),
    }),
    { name: "cluster-runtime-settings" },
  ),
);
