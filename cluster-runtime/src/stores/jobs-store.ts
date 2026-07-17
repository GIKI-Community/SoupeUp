import { create } from "zustand";

import { JobApi } from "@/api";
import type { Job, JobStatus } from "@/types";

interface JobsState {
  jobs: Job[];
  search: string;
  statusFilter: JobStatus | "all";
  sortField: keyof Job;
  sortDirection: "asc" | "desc";
  isLoading: boolean;
  error: string | null;
  setSearch: (search: string) => void;
  setStatusFilter: (status: JobStatus | "all") => void;
  setSort: (field: keyof Job, direction?: "asc" | "desc") => void;
  fetchJobs: () => Promise<void>;
  tickRunningJobs: () => void;
}

export const useJobsStore = create<JobsState>((set, get) => ({
  jobs: [],
  search: "",
  statusFilter: "all",
  sortField: "submittedAt",
  sortDirection: "desc",
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
  fetchJobs: async () => {
    set({ isLoading: true, error: null });
    try {
      const jobs = await JobApi.list();
      set({ jobs, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        error:
          typeof error === "string"
            ? error
            : error instanceof Error
              ? error.message
              : "Failed to load jobs",
      });
    }
  },
  tickRunningJobs: () => {
    set((state) => ({
      jobs: state.jobs.map((job) =>
        job.status === "running"
          ? { ...job, durationSecs: job.durationSecs + 1 }
          : job,
      ),
    }));
  },
}));
