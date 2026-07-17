import { create } from "zustand";

import { ClusterApi, NodeApi } from "@/api";
import type { Node, NodeStatus } from "@/types";

interface ClusterSummary {
  total_nodes: number;
  online_nodes: number;
  total_cpus: number;
  total_ram: number;
  total_gpus: number;
  total_workers: number;
  total_available_compute: number;
}

interface NodesState {
  nodes: Node[];
  summary: ClusterSummary | null;
  search: string;
  statusFilter: NodeStatus | "all";
  sortField: keyof Node;
  sortDirection: "asc" | "desc";
  isLoading: boolean;
  error: string | null;
  setSearch: (search: string) => void;
  setStatusFilter: (status: NodeStatus | "all") => void;
  setSort: (field: keyof Node, direction?: "asc" | "desc") => void;
  fetchNodes: () => Promise<void>;
  fetchSummary: () => Promise<void>;
}

export const useNodesStore = create<NodesState>((set, get) => ({
  nodes: [],
  summary: null,
  search: "",
  statusFilter: "all",
  sortField: "name",
  sortDirection: "asc",
  isLoading: false,
  error: null,
  setSearch: (search) => set({ search }),
  setStatusFilter: (statusFilter) => set({ statusFilter }),
  setSort: (field, direction) => {
    const current = get();
    const nextDirection =
      direction ??
      (current.sortField === field && current.sortDirection === "asc"
        ? "desc"
        : "asc");
    set({ sortField: field, sortDirection: nextDirection });
  },
  fetchNodes: async () => {
    set({ isLoading: true, error: null });
    try {
      // Try to get real cluster peers first
      try {
        const peers = await ClusterApi.getPeers();
        // Convert peers to Node format
        const nodes: Node[] = peers.map((peer) => ({
          id: peer.node_id,
          name: peer.node_name,
          platform: "other" as Node["platform"],
          status: (peer.status.toLowerCase() as NodeStatus) || "offline",
          cpuPercent: peer.resources.cpu_usage,
          memoryPercent: (peer.resources.ram_used / peer.resources.ram_total) * 100,
          backend: `${peer.host}:${peer.port}`,
          version: peer.version,
          lastSeen: new Date(peer.last_heartbeat),
        }));
        set({ nodes, isLoading: false });
      } catch {
        // Fall back to mock nodes
        const nodes = await NodeApi.list();
        set({ nodes, isLoading: false });
      }
    } catch (error) {
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : "Failed to load nodes",
      });
    }
  },
  fetchSummary: async () => {
    try {
      const summary = await ClusterApi.getSummary();
      set({ summary });
    } catch (error) {
      console.error("Failed to fetch cluster summary:", error);
    }
  },
}));
