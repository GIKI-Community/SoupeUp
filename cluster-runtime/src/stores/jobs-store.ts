import { create } from "zustand";

import { JobApi, SchedulerApi } from "@/api";
import type { Job, JobDetail, JobSpec, JobStatus, SubmitAck } from "@/types";

interface JobsState {
  jobs: Job[];
  search: string;
  statusFilter: JobStatus | "all";
  schedulerFilter: "all" | "dask" | "ray";
  sortField: keyof Job;
  sortDirection: "asc" | "desc";
  isLoading: boolean;
  isSubmitting: boolean;
  error: string | null;
  selectedJobId: string | null;
  jobDetail: JobDetail | null;
  activeScheduler: string | null;
  setSearch: (search: string) => void;
  setStatusFilter: (status: JobStatus | "all") => void;
  setSchedulerFilter: (filter: "all" | "dask" | "ray") => void;
  setSort: (field: keyof Job, direction?: "asc" | "desc") => void;
  fetchJobs: () => Promise<void>;
  fetchActiveScheduler: () => Promise<void>;
  setActiveScheduler: (pluginId: string) => Promise<boolean>;
  submitJob: (spec: JobSpec) => Promise<SubmitAck | null>;
  runExample: (exampleId: string, name?: string) => Promise<SubmitAck | null>;
  cancelJob: (jobId: string) => Promise<boolean>;
  retryJob: (jobId: string) => Promise<SubmitAck | null>;
  fetchJobDetail: (jobId: string) => Promise<void>;
  clearJobDetail: () => void;
  tickRunningJobs: () => void;
}

function errMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Operation failed";
}

const RUNNING_STATUSES: JobStatus[] = [
  "created",
  "queued",
  "scheduling",
  "running",
];

export const useJobsStore = create<JobsState>((set, get) => ({
  jobs: [],
  search: "",
  statusFilter: "all",
  schedulerFilter: "all",
  sortField: "submittedAt",
  sortDirection: "desc",
  isLoading: false,
  isSubmitting: false,
  error: null,
  selectedJobId: null,
  jobDetail: null,
  activeScheduler: null,

  setSearch: (search) => set({ search }),
  setStatusFilter: (statusFilter) => set({ statusFilter }),
  setSchedulerFilter: (schedulerFilter) => set({ schedulerFilter }),
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
      set({ isLoading: false, error: errMessage(error) });
    }
  },

  fetchActiveScheduler: async () => {
    try {
      const activeScheduler = await SchedulerApi.getActive();
      set({ activeScheduler });
    } catch {
      // ignore
    }
  },

  setActiveScheduler: async (pluginId) => {
    try {
      await SchedulerApi.setActive(pluginId);
      set({ activeScheduler: pluginId });
      return true;
    } catch (error) {
      set({ error: errMessage(error) });
      return false;
    }
  },

  submitJob: async (spec) => {
    set({ isSubmitting: true, error: null });
    try {
      const ack = await JobApi.submit(spec);
      await get().fetchJobs();
      set({ isSubmitting: false });
      return ack;
    } catch (error) {
      set({ isSubmitting: false, error: errMessage(error) });
      return null;
    }
  },

  runExample: async (exampleId, name) => {
    set({ isSubmitting: true, error: null });
    try {
      const ack = await JobApi.submitExample(exampleId, name);
      await get().fetchJobs();
      set({ isSubmitting: false });
      return ack;
    } catch (error) {
      set({ isSubmitting: false, error: errMessage(error) });
      return null;
    }
  },

  cancelJob: async (jobId) => {
    try {
      await JobApi.cancel(jobId);
      await get().fetchJobs();
      return true;
    } catch (error) {
      set({ error: errMessage(error) });
      return false;
    }
  },

  retryJob: async (jobId) => {
    set({ isSubmitting: true, error: null });
    try {
      const ack = await JobApi.retry(jobId);
      await get().fetchJobs();
      set({ isSubmitting: false });
      return ack;
    } catch (error) {
      set({ isSubmitting: false, error: errMessage(error) });
      return null;
    }
  },

  fetchJobDetail: async (jobId) => {
    try {
      const jobDetail = await JobApi.get(jobId);
      set({ selectedJobId: jobId, jobDetail });
    } catch (error) {
      set({ error: errMessage(error) });
    }
  },

  clearJobDetail: () => set({ selectedJobId: null, jobDetail: null }),

  tickRunningJobs: () => {
    set((state) => ({
      jobs: state.jobs.map((job) =>
        RUNNING_STATUSES.includes(job.status)
          ? { ...job, durationSecs: job.durationSecs + 1 }
          : job,
      ),
    }));
  },
}));
