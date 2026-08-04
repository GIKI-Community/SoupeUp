import { create } from "zustand";

import { NetworkApi } from "@/api";
import type { NetworkPeer } from "@/types";

interface NetworkState {
  localEndpointId: string | null;
  listenAddrs: string[];
  peers: NetworkPeer[];
  connectDraft: string;
  isBusy: boolean;
  error: string | null;
  lastConnected: string | null;
  setConnectDraft: (value: string) => void;
  refresh: () => Promise<void>;
  connect: (endpointId?: string) => Promise<boolean>;
}

export const useNetworkStore = create<NetworkState>((set, get) => ({
  localEndpointId: null,
  listenAddrs: [],
  peers: [],
  connectDraft: "",
  isBusy: false,
  error: null,
  lastConnected: null,

  setConnectDraft: (connectDraft) => set({ connectDraft }),

  refresh: async () => {
    try {
      const [localEndpointId, listenAddrs, peers] = await Promise.all([
        NetworkApi.localEndpointId(),
        NetworkApi.listenAddrs(),
        NetworkApi.peers(),
      ]);
      set({
        localEndpointId,
        listenAddrs,
        peers,
        error: null,
      });
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : String(e),
      });
    }
  },

  connect: async (endpointId) => {
    const id = (endpointId ?? get().connectDraft).trim();
    if (!id) {
      set({ error: "Paste an EndpointId to connect" });
      return false;
    }
    set({ isBusy: true, error: null });
    try {
      await NetworkApi.connect(id);
      set({ lastConnected: id, connectDraft: "", isBusy: false });
      await get().refresh();
      return true;
    } catch (e) {
      set({
        isBusy: false,
        error: e instanceof Error ? e.message : String(e),
      });
      return false;
    }
  },
}));
