import { ArrowUpDown, Search } from "lucide-react";
import { useEffect, useMemo } from "react";

import { JobStatusBadge } from "@/components/status-badges";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { PageHeader } from "@/layouts/app-layout";
import { formatDuration } from "@/lib/utils";
import { useJobsStore } from "@/stores";
import type { Job, JobStatus } from "@/types";

export function JobsPage() {
  const {
    jobs,
    search,
    statusFilter,
    sortField,
    sortDirection,
    setSearch,
    setStatusFilter,
    setSort,
    fetchJobs,
    tickRunningJobs,
  } = useJobsStore();

  useEffect(() => {
    void fetchJobs();
    const refreshInterval = setInterval(() => void fetchJobs(), 2000);
    const tickInterval = setInterval(() => tickRunningJobs(), 1000);
    return () => {
      clearInterval(refreshInterval);
      clearInterval(tickInterval);
    };
  }, [fetchJobs, tickRunningJobs]);

  const filteredJobs = useMemo(() => {
    return jobs
      .filter((job) => {
        const matchesSearch =
          search === "" ||
          job.id.toLowerCase().includes(search.toLowerCase()) ||
          job.owner.toLowerCase().includes(search.toLowerCase());
        const matchesStatus =
          statusFilter === "all" || job.status === statusFilter;
        return matchesSearch && matchesStatus;
      })
      .sort((a, b) => {
        const aVal = a[sortField];
        const bVal = b[sortField];
        const direction = sortDirection === "asc" ? 1 : -1;

        if (typeof aVal === "string" && typeof bVal === "string") {
          return aVal.localeCompare(bVal) * direction;
        }
        if (typeof aVal === "number" && typeof bVal === "number") {
          return (aVal - bVal) * direction;
        }
        return 0;
      });
  }, [jobs, search, statusFilter, sortField, sortDirection]);

  const columns: { key: keyof Job; label: string }[] = [
    { key: "id", label: "Job ID" },
    { key: "status", label: "Status" },
    { key: "owner", label: "Owner" },
    { key: "submittedAt", label: "Submitted" },
    { key: "runtime", label: "Runtime" },
    { key: "durationSecs", label: "Duration" },
  ];

  return (
    <div>
      <PageHeader
        title="Jobs"
        description="Monitor and manage cluster workloads"
      />

      <div className="mb-6 flex flex-col gap-4 sm:flex-row">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search jobs..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select
          value={statusFilter}
          onValueChange={(value) => setStatusFilter(value as JobStatus | "all")}
        >
          <SelectTrigger className="w-full sm:w-44">
            <SelectValue placeholder="Filter status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All statuses</SelectItem>
            <SelectItem value="pending">Pending</SelectItem>
            <SelectItem value="running">Running</SelectItem>
            <SelectItem value="completed">Completed</SelectItem>
            <SelectItem value="failed">Failed</SelectItem>
            <SelectItem value="cancelled">Cancelled</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-xl border border-border/60 bg-card/80">
        <Table>
          <TableHeader>
            <TableRow>
              {columns.map((col) => (
                <TableHead key={col.key}>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="-ml-3 h-8"
                    onClick={() => setSort(col.key)}
                  >
                    {col.label}
                    <ArrowUpDown className="ml-1 h-3.5 w-3.5" />
                  </Button>
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredJobs.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={columns.length}
                  className="py-8 text-center text-sm text-muted-foreground"
                >
                  No jobs yet. Run an example from the Cluster page to see it here.
                </TableCell>
              </TableRow>
            ) : (
              filteredJobs.map((job) => (
              <TableRow key={job.id}>
                <TableCell className="font-mono text-xs">{job.id}</TableCell>
                <TableCell>
                  <JobStatusBadge status={job.status} />
                </TableCell>
                <TableCell>{job.owner}</TableCell>
                <TableCell className="text-muted-foreground">
                  {new Date(job.submittedAt).toLocaleString()}
                </TableCell>
                <TableCell className="font-mono text-xs">{job.runtime}</TableCell>
                <TableCell>{formatDuration(job.durationSecs)}</TableCell>
              </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
