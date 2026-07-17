import { useEffect, useState } from "react";
import {
  Activity,
  Cpu,
  ExternalLink,
  HardDrive,
  Loader2,
  Network,
  Play,
  RefreshCw,
  Server,
  Square,
  Users,
  Zap,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { StatCard } from "@/components/stat-card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useRayStore } from "@/stores";
import { RAY_EXAMPLES, exampleErrorMessage, type ComponentStatus } from "@/types";

function statusVariant(
  status: ComponentStatus | string,
): "success" | "warning" | "destructive" | "muted" {
  switch (status) {
    case "running":
    case "healthy":
    case "alive":
      return "success";
    case "starting":
    case "stopping":
    case "degraded":
      return "warning";
    case "error":
    case "unhealthy":
      return "destructive";
    default:
      return "muted";
  }
}

function formatBytes(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i += 1;
  }
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function ProcessLogPanel({
  title,
  logs,
  emptyMessage,
}: {
  title: string;
  logs?: string;
  emptyMessage: string;
}) {
  const text = logs?.trim();
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">{title}</p>
      <div className="max-h-48 overflow-y-auto rounded-md border border-border/60 bg-background/80 p-3">
        <pre className="whitespace-pre-wrap break-all font-mono text-[11px] leading-relaxed text-foreground/90">
          {text || emptyMessage}
        </pre>
      </div>
    </div>
  );
}

export function RayClusterPanel() {
  const {
    snapshot,
    dashboard,
    lastExample,
    isBusy,
    headBusy,
    workerBusy,
    isRunningExample,
    error,
    joinAddress,
    fetchSnapshot,
    fetchSettings,
    fetchDashboard,
    startHead,
    stopHead,
    restartHead,
    startWorker,
    stopWorker,
    restartWorker,
    ensurePackages,
    runExample,
    setJoinAddress,
  } = useRayStore();

  const [activeDashTab, setActiveDashTab] = useState("overview");

  useEffect(() => {
    void fetchSettings();
    void fetchSnapshot();
    void fetchDashboard();
    const timer = window.setInterval(() => {
      void fetchSnapshot();
    }, 2500);
    return () => window.clearInterval(timer);
  }, [fetchSettings, fetchSnapshot, fetchDashboard]);

  const head = snapshot?.head;
  const localWorker = snapshot?.localWorker;
  const dashTab =
    dashboard?.tabs.find((t) => t.id === activeDashTab) ?? dashboard?.tabs[0];
  const headRunning = head?.status === "running";
  const workerCount = snapshot?.workers.length ?? 0;
  const examplesReady = headRunning && workerCount > 0;
  const examplesBlockedReason = !headRunning
    ? "Start the Ray head before running examples."
    : workerCount === 0
      ? "Start at least one worker before running examples."
      : null;

  return (
    <div>
      <div className="mb-4 flex items-center justify-end gap-2">
        <Button
          variant="outline"
          size="sm"
          disabled={isBusy}
          onClick={() => void ensurePackages()}
        >
          Ensure Ray Packages
        </Button>
        <Button
          variant="outline"
          size="sm"
          disabled={isBusy}
          onClick={() => {
            void fetchSnapshot();
            void fetchDashboard();
          }}
        >
          <RefreshCw className="mr-2 h-3.5 w-3.5" />
          Refresh
        </Button>
      </div>

      {error && (
        <div className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      <div className="mb-6 grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Cluster Health"
          value={snapshot?.health ?? "unknown"}
          subtitle={
            snapshot?.clientConnected ? "Client connected" : "Client idle"
          }
          icon={Activity}
        />
        <StatCard
          title="Connected Workers"
          value={snapshot?.workers.length ?? 0}
          subtitle={`${snapshot?.totalCores ?? 0} CPUs`}
          icon={Users}
        />
        <StatCard
          title="Total Memory"
          value={formatBytes(snapshot?.totalMemory ?? 0)}
          subtitle={`${snapshot?.activeTasks ?? 0} active tasks`}
          icon={HardDrive}
        />
        <StatCard
          title="Task Stats"
          value={snapshot?.completedTasks ?? 0}
          subtitle={`${snapshot?.failedTasks ?? 0} failed`}
          icon={Zap}
        />
      </div>

      <div className="mb-6 grid gap-4 lg:grid-cols-2">
        <Card className="border-border/60 bg-card/80">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle className="flex items-center gap-2">
                  <Server className="h-4 w-4" />
                  Ray Head
                </CardTitle>
                <CardDescription>
                  Start a Ray head node on this machine for workers to join.
                </CardDescription>
              </div>
              <Badge variant={statusVariant(head?.status ?? "stopped")}>
                {head?.status ?? "stopped"}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-1 text-sm text-muted-foreground">
              <p>
                Address:{" "}
                <span className="font-mono text-foreground">
                  {head?.address ?? "—"}
                </span>
              </p>
              <p>
                Dashboard:{" "}
                <span className="font-mono text-foreground">
                  {head?.dashboardUrl ?? "—"}
                </span>
              </p>
              {head?.error && (
                <p className="text-destructive">{head.error}</p>
              )}
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                disabled={headBusy || head?.status === "running"}
                onClick={() => void startHead()}
              >
                {headBusy ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <Play className="mr-2 h-4 w-4" />
                )}
                Start Head
              </Button>
              <Button
                type="button"
                variant="outline"
                disabled={headBusy || head?.status !== "running"}
                onClick={() => void stopHead()}
              >
                <Square className="mr-2 h-4 w-4" />
                Stop
              </Button>
              <Button
                type="button"
                variant="outline"
                disabled={headBusy}
                onClick={() => void restartHead()}
              >
                <RefreshCw className="mr-2 h-4 w-4" />
                Restart
              </Button>
            </div>
            <ProcessLogPanel
              title="Head logs"
              logs={head?.logs}
              emptyMessage="No head output yet. Start the Ray head to see logs here."
            />
          </CardContent>
        </Card>

        <Card className="border-border/60 bg-card/80">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle className="flex items-center gap-2">
                  <Cpu className="h-4 w-4" />
                  Worker
                </CardTitle>
                <CardDescription>
                  Join a Ray head by address — use this on worker-only machines.
                </CardDescription>
              </div>
              <Badge variant={statusVariant(localWorker?.status ?? "stopped")}>
                {localWorker?.status ?? "stopped"}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="ray-join-address">Head address</Label>
              <Input
                id="ray-join-address"
                className="font-mono text-xs"
                value={joinAddress}
                onChange={(e) => setJoinAddress(e.target.value)}
                placeholder="192.168.1.10:6379"
              />
            </div>
            {localWorker?.error && (
              <p className="text-sm text-destructive">{localWorker.error}</p>
            )}
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                disabled={workerBusy || localWorker?.status === "running"}
                onClick={() => void startWorker(joinAddress)}
              >
                {workerBusy ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <Play className="mr-2 h-4 w-4" />
                )}
                Start Worker
              </Button>
              <Button
                type="button"
                variant="outline"
                disabled={workerBusy || localWorker?.status !== "running"}
                onClick={() => void stopWorker()}
              >
                <Square className="mr-2 h-4 w-4" />
                Stop
              </Button>
              <Button
                type="button"
                variant="outline"
                disabled={workerBusy}
                onClick={() => void restartWorker()}
              >
                <RefreshCw className="mr-2 h-4 w-4" />
                Restart
              </Button>
            </div>
            <ProcessLogPanel
              title="Worker logs"
              logs={localWorker?.logs}
              emptyMessage="No worker output yet. Start the worker to see connection logs here."
            />
          </CardContent>
        </Card>
      </div>

      <Card className="mb-6 border-border/60 bg-card/80">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Network className="h-4 w-4" />
            Connected Workers
          </CardTitle>
          <CardDescription>
            Live view from the Ray cluster. Updates every few seconds.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Address</TableHead>
                <TableHead>CPUs</TableHead>
                <TableHead>CPU</TableHead>
                <TableHead>Memory</TableHead>
                <TableHead>Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(snapshot?.workers ?? []).length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={6}
                    className="text-center text-muted-foreground"
                  >
                    No workers connected yet.
                  </TableCell>
                </TableRow>
              ) : (
                snapshot?.workers.map((w) => (
                  <TableRow key={w.id}>
                    <TableCell className="font-medium">{w.name}</TableCell>
                    <TableCell className="font-mono text-xs">{w.address}</TableCell>
                    <TableCell>{w.nthreads}</TableCell>
                    <TableCell>{w.cpu.toFixed(1)}%</TableCell>
                    <TableCell>
                      {formatBytes(w.memoryUsed)} / {formatBytes(w.memoryLimit)}
                    </TableCell>
                    <TableCell>
                      <Badge variant={statusVariant(w.status)}>{w.status}</Badge>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <div className="mb-6 grid gap-4 lg:grid-cols-2">
        <Card className="border-border/60 bg-card/80">
          <CardHeader>
            <CardTitle>Example Jobs</CardTitle>
            <CardDescription>
              Zero-code demos that prove multi-node distribution with Ray.
              {examplesBlockedReason ? (
                <span className="mt-1 block text-amber-600 dark:text-amber-400">
                  {examplesBlockedReason}
                </span>
              ) : null}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {RAY_EXAMPLES.map((ex) => (
              <div
                key={ex.id}
                className="flex items-start justify-between gap-3 rounded-lg border border-border/50 p-3"
              >
                <div>
                  <p className="text-sm font-medium">{ex.title}</p>
                  <p className="text-xs text-muted-foreground">{ex.description}</p>
                  {ex.packages.length > 0 ? (
                    <p className="mt-1 text-xs text-muted-foreground">
                      Requires: {ex.packages.join(", ")}
                    </p>
                  ) : null}
                </div>
                <Button
                  size="sm"
                  disabled={isRunningExample || !examplesReady}
                  title={examplesBlockedReason ?? undefined}
                  onClick={() => void runExample(ex.id)}
                >
                  {isRunningExample ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    "Run"
                  )}
                </Button>
              </div>
            ))}
          </CardContent>
        </Card>

        <Card className="border-border/60 bg-card/80">
          <CardHeader>
            <CardTitle>Last Example Result</CardTitle>
            <CardDescription>
              Execution time, workers, and optional single-node speedup.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {!lastExample ? (
              <p className="text-sm text-muted-foreground">
                Run an example to see results here.
              </p>
            ) : (
              <div className="space-y-3 text-sm">
                <div className="flex items-center gap-2">
                  <p className="font-medium">{lastExample.title}</p>
                  <Badge
                    variant={lastExample.success ? "success" : "destructive"}
                  >
                    {lastExample.success ? "success" : "failed"}
                  </Badge>
                </div>
                <p>Execution time: {lastExample.executionTimeMs} ms</p>
                <p>Workers used: {lastExample.workersUsed}</p>
                <p>
                  Speedup:{" "}
                  {lastExample.speedup != null
                    ? `${lastExample.speedup.toFixed(2)}×`
                    : "—"}
                </p>
                <p
                  className={
                    lastExample.success
                      ? "break-all text-muted-foreground"
                      : "break-all rounded-md border border-destructive/40 bg-destructive/10 p-3 text-destructive"
                  }
                >
                  {lastExample.success
                    ? lastExample.resultSummary
                    : exampleErrorMessage(lastExample)}
                </p>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <Card className="border-border/60 bg-card/80">
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <div>
              <CardTitle>Ray Dashboard</CardTitle>
              <CardDescription>
                Official Ray dashboard — embedded when possible, otherwise open
                in your browser.
              </CardDescription>
            </div>
            {dashTab && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => void openUrl(dashTab.url)}
              >
                <ExternalLink className="mr-2 h-3.5 w-3.5" />
                Open
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent>
          <Tabs
            value={activeDashTab}
            onValueChange={setActiveDashTab}
            className="w-full"
          >
            <TabsList className="mb-3 flex h-auto flex-wrap">
              {(dashboard?.tabs ?? []).map((tab) => (
                <TabsTrigger key={tab.id} value={tab.id}>
                  {tab.label}
                </TabsTrigger>
              ))}
            </TabsList>
            {(dashboard?.tabs ?? []).map((tab) => (
              <TabsContent key={tab.id} value={tab.id} className="mt-0">
                <div className="overflow-hidden rounded-lg border border-border/60 bg-background">
                  <iframe
                    title={tab.label}
                    src={tab.url}
                    className="h-[520px] w-full bg-background"
                    sandbox="allow-scripts allow-same-origin allow-forms"
                  />
                </div>
                <p className="mt-2 text-xs text-muted-foreground">
                  If the frame is blank (CSP / platform limits), use Open to view{" "}
                  <span className="font-mono">{tab.url}</span>
                </p>
              </TabsContent>
            ))}
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
}
