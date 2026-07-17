import { create } from "zustand";

import { NodeApi, ClusterApi } from "@/api";
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
      const nodes = await NodeApi.list();
      set({ nodes, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        error:
          typeof error === "string"
            ? error
            : error instanceof Error
              ? error.message
              : "Failed to load nodes",
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
