import { useEffect } from "react";
import {
  Box,
  Cpu,
  MemoryStick,
  Puzzle,
  Server,
  Zap,
} from "lucide-react";

import { ServiceStatusBadge } from "@/components/status-badges";
import { StatCard, UsageStatCard } from "@/components/stat-card";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/layouts/app-layout";
import { formatDuration, formatRelativeTime } from "@/lib/utils";
import { useSystemStore } from "@/stores";

export function DashboardPage() {
  const { info, status, activity, fetchAll } = useSystemStore();

  useEffect(() => {
    void fetchAll();
    const interval = setInterval(() => void fetchAll(), 30000);
    return () => clearInterval(interval);
  }, [fetchAll]);

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Overview of your distributed compute cluster"
        actions={
          <Button variant="outline" size="sm" onClick={() => void fetchAll()}>
            Refresh
          </Button>
        }
      />

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
        <StatCard
          title="Total Nodes"
          value={info?.totalNodes ?? "—"}
          icon={Server}
        />
        <StatCard
          title="Online Nodes"
          value={info?.onlineNodes ?? "—"}
          subtitle={`${info ? info.totalNodes - info.onlineNodes : 0} offline`}
          icon={Zap}
        />
        <StatCard
          title="Active Jobs"
          value={info?.activeJobs ?? "—"}
          icon={Box}
        />
        <StatCard
          title="Installed Plugins"
          value={info?.installedPlugins ?? "—"}
          icon={Puzzle}
        />
        <UsageStatCard
          title="CPU Usage"
          percent={info?.cpuUsagePercent ?? 0}
          icon={Cpu}
        />
        <UsageStatCard
          title="RAM Usage"
          percent={info?.memoryUsagePercent ?? 0}
          icon={MemoryStick}
        />
      </div>

      <div className="mt-8 grid gap-6 lg:grid-cols-3">
        <Card className="lg:col-span-2 border-border/60 bg-card/80">
          <CardHeader>
            <CardTitle className="text-base">Recent Activity</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {activity.map((entry) => (
                <div
                  key={entry.id}
                  className="flex items-start justify-between gap-4 border-b border-border/50 pb-4 last:border-0 last:pb-0"
                >
                  <div>
                    <p className="text-sm">{entry.message}</p>
                    <p className="mt-1 text-xs capitalize text-muted-foreground">
                      {entry.category}
                    </p>
                  </div>
                  <span className="shrink-0 text-xs text-muted-foreground">
                    {formatRelativeTime(entry.timestamp)}
                  </span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <div className="space-y-6">
          <Card className="border-border/60 bg-card/80">
            <CardHeader>
              <CardTitle className="text-base">System Status</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {status && (
                <>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">API</span>
                    <ServiceStatusBadge status={status.api} />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Storage</span>
                    <ServiceStatusBadge status={status.storage} />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">Networking</span>
                    <ServiceStatusBadge status={status.networking} />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">
                      Plugin Manager
                    </span>
                    <ServiceStatusBadge status={status.pluginManager} />
                  </div>
                </>
              )}
              {info && (
                <p className="pt-2 text-xs text-muted-foreground">
                  Uptime: {formatDuration(info.uptimeSecs)}
                </p>
              )}
            </CardContent>
          </Card>

          <Card className="border-border/60 bg-card/80">
            <CardHeader>
              <CardTitle className="text-base">Quick Actions</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-2">
              <Button variant="outline" className="justify-start" disabled>
                Add Node
              </Button>
              <Button variant="outline" className="justify-start" disabled>
                Submit Job
              </Button>
              <Button variant="outline" className="justify-start" disabled>
                Install Plugin
              </Button>
              <Button variant="outline" className="justify-start" disabled>
                View Metrics
              </Button>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
