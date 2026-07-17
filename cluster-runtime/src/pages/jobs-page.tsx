import {
  ArrowUpDown,
  Eye,
  RefreshCw,
  RotateCcw,
  Search,
  XCircle,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { JobStatusBadge } from "@/components/status-badges";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
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
import {
  SCHEDULER_EXAMPLES,
  schedulerDisplayName,
  type Job,
  type JobStatus,
} from "@/types";

export function JobsPage() {
  const {
    jobs,
    search,
    statusFilter,
    schedulerFilter,
    sortField,
    sortDirection,
    isLoading,
    error,
    jobDetail,
    activeScheduler,
    setSearch,
    setStatusFilter,
    setSchedulerFilter,
    setSort,
    fetchJobs,
    fetchActiveScheduler,
    setActiveScheduler,
    runExample,
    cancelJob,
    retryJob,
    fetchJobDetail,
    clearJobDetail,
    tickRunningJobs,
  } = useJobsStore();

  const [schedulers, setSchedulers] = useState<
    { pluginId: string; displayName: string }[]
  >([]);

  useEffect(() => {
    void fetchJobs();
    void fetchActiveScheduler();
    void import("@/api").then(({ SchedulerApi }) =>
      SchedulerApi.list().then((list) =>
        setSchedulers(
          list.map((s) => ({
            pluginId: s.pluginId,
            displayName: s.displayName,
          })),
        ),
      ),
    );
    const refreshInterval = setInterval(() => void fetchJobs(), 2000);
    const tickInterval = setInterval(() => tickRunningJobs(), 1000);
    return () => {
      clearInterval(refreshInterval);
      clearInterval(tickInterval);
    };
  }, [fetchJobs, fetchActiveScheduler, tickRunningJobs]);

  const filteredJobs = useMemo(() => {
    return jobs
      .filter((job) => {
        const matchesSearch =
          search === "" ||
          job.id.toLowerCase().includes(search.toLowerCase()) ||
          job.name.toLowerCase().includes(search.toLowerCase()) ||
          job.owner.toLowerCase().includes(search.toLowerCase());
        const matchesStatus =
          statusFilter === "all" || job.status === statusFilter;
        const matchesScheduler =
          schedulerFilter === "all" ||
          (schedulerFilter === "dask" &&
            job.schedulerId.includes("dask")) ||
          (schedulerFilter === "ray" && job.schedulerId.includes("ray"));
        return matchesSearch && matchesStatus && matchesScheduler;
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
  }, [jobs, search, statusFilter, schedulerFilter, sortField, sortDirection]);

  const columns: { key: keyof Job; label: string }[] = [
    { key: "name", label: "Name" },
    { key: "schedulerId", label: "Scheduler" },
    { key: "status", label: "Status" },
    { key: "durationSecs", label: "Duration" },
    { key: "owner", label: "Owner" },
    { key: "submittedAt", label: "Submitted" },
  ];

  const cancellable = (status: JobStatus) =>
    ["created", "queued", "scheduling", "running"].includes(status);

  return (
    <div>
      <PageHeader
        title="Jobs"
        description="Monitor and manage cluster workloads"
        actions={
          <div className="flex items-center gap-2">
            <Select
              value={activeScheduler ?? undefined}
              onValueChange={(v) => void setActiveScheduler(v)}
            >
              <SelectTrigger className="w-36">
                <SelectValue placeholder="Scheduler" />
              </SelectTrigger>
              <SelectContent>
                {schedulers.map((s) => (
                  <SelectItem key={s.pluginId} value={s.pluginId}>
                    {s.displayName}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void fetchJobs()}
              disabled={isLoading}
            >
              <RefreshCw className="mr-1 h-4 w-4" />
              Refresh
            </Button>
          </div>
        }
      />

      {error && (
        <p className="mb-4 text-sm text-destructive">{error}</p>
      )}

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
          value={schedulerFilter}
          onValueChange={(v) =>
            setSchedulerFilter(v as "all" | "dask" | "ray")
          }
        >
          <SelectTrigger className="w-full sm:w-36">
            <SelectValue placeholder="Scheduler" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All schedulers</SelectItem>
            <SelectItem value="dask">Dask</SelectItem>
            <SelectItem value="ray">Ray</SelectItem>
          </SelectContent>
        </Select>
        <Select
          value={statusFilter}
          onValueChange={(value) => setStatusFilter(value as JobStatus | "all")}
        >
          <SelectTrigger className="w-full sm:w-44">
            <SelectValue placeholder="Filter status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All statuses</SelectItem>
            <SelectItem value="queued">Queued</SelectItem>
            <SelectItem value="running">Running</SelectItem>
            <SelectItem value="completed">Completed</SelectItem>
            <SelectItem value="failed">Failed</SelectItem>
            <SelectItem value="cancelled">Cancelled</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="mb-6 rounded-xl border border-border/60 bg-card/80 p-4">
        <h3 className="mb-3 text-sm font-medium">Run Example Job</h3>
        <div className="flex flex-wrap gap-2">
          {SCHEDULER_EXAMPLES.map((ex) => (
            <Button
              key={ex.id}
              variant="outline"
              size="sm"
              onClick={() => void runExample(ex.id, ex.title)}
            >
              {ex.title}
            </Button>
          ))}
        </div>
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
              <TableHead>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredJobs.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={columns.length + 1}
                  className="py-8 text-center text-sm text-muted-foreground"
                >
                  No jobs yet. Run an example above or from the Cluster page.
                </TableCell>
              </TableRow>
            ) : (
              filteredJobs.map((job) => (
                <TableRow key={job.id}>
                  <TableCell className="font-medium">{job.name}</TableCell>
                  <TableCell>
                    {schedulerDisplayName(job.schedulerId)}
                  </TableCell>
                  <TableCell>
                    <JobStatusBadge status={job.status} />
                  </TableCell>
                  <TableCell>{formatDuration(job.durationSecs)}</TableCell>
                  <TableCell>{job.owner}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {new Date(job.submittedAt).toLocaleString()}
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => void fetchJobDetail(job.id)}
                      >
                        <Eye className="h-4 w-4" />
                      </Button>
                      {cancellable(job.status) && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => void cancelJob(job.id)}
                        >
                          <XCircle className="h-4 w-4" />
                        </Button>
                      )}
                      {job.status === "failed" && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => void retryJob(job.id)}
                        >
                          <RotateCcw className="h-4 w-4" />
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {jobDetail && (
        <Card className="mt-6 border-border/60 bg-card/80">
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle>{jobDetail.name}</CardTitle>
            <Button variant="ghost" size="sm" onClick={() => clearJobDetail()}>
              Close
            </Button>
          </CardHeader>
          <CardContent className="space-y-4 text-sm">
            <div className="grid grid-cols-2 gap-2">
              <span className="text-muted-foreground">ID</span>
              <span className="font-mono text-xs">{jobDetail.id}</span>
              <span className="text-muted-foreground">Status</span>
              <JobStatusBadge status={jobDetail.status} />
              <span className="text-muted-foreground">Scheduler</span>
              <span>{schedulerDisplayName(jobDetail.schedulerId)}</span>
              <span className="text-muted-foreground">Progress</span>
              <span>{jobDetail.progress.percent.toFixed(0)}%</span>
            </div>
            {jobDetail.result?.resultSummary && (
              <div>
                <p className="mb-1 font-medium">Result</p>
                <p className="text-muted-foreground">
                  {jobDetail.result.resultSummary}
                </p>
              </div>
            )}
            {jobDetail.dependencies &&
              (jobDetail.dependencies.detected.length > 0 ||
                jobDetail.dependencies.installed.length > 0 ||
                jobDetail.dependencies.alreadyPresent.length > 0 ||
                jobDetail.dependencies.skippedStdlib.length > 0) && (
                <div>
                  <p className="mb-1 font-medium">Dependencies</p>
                  <div className="space-y-1 text-muted-foreground">
                    {jobDetail.dependencies.detected.length > 0 && (
                      <p>
                        <span className="text-foreground">Detected:</span>{" "}
                        {jobDetail.dependencies.detected.join(", ")}
                      </p>
                    )}
                    {jobDetail.dependencies.installed.length > 0 && (
                      <p>
                        <span className="text-foreground">Installed:</span>{" "}
                        {jobDetail.dependencies.installed.join(", ")}
                      </p>
                    )}
                    {jobDetail.dependencies.alreadyPresent.length > 0 && (
                      <p>
                        <span className="text-foreground">Already present:</span>{" "}
                        {jobDetail.dependencies.alreadyPresent.join(", ")}
                      </p>
                    )}
                    {jobDetail.dependencies.skippedStdlib.length > 0 && (
                      <p>
                        <span className="text-foreground">Stdlib (skipped):</span>{" "}
                        {jobDetail.dependencies.skippedStdlib.join(", ")}
                      </p>
                    )}
                  </div>
                </div>
              )}
            {jobDetail.logs.length > 0 && (
              <div>
                <p className="mb-1 font-medium">Logs</p>
                <pre className="max-h-48 overflow-auto rounded bg-muted p-2 text-xs">
                  {jobDetail.logs.join("\n")}
                </pre>
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
