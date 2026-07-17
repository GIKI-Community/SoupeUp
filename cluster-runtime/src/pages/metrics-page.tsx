import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { StatCard } from "@/components/stat-card";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/layouts/app-layout";
import { useDaskStore, useMetricsStore } from "@/stores";
import type { MetricSeries } from "@/types";
import { Activity, Cpu, HardDrive, Network, Zap } from "lucide-react";

function MetricChart({
  series,
  color,
  domain,
}: {
  series: MetricSeries;
  color: string;
  domain?: [number, number];
}) {
  const chartId = series.name.replace(/[^a-zA-Z0-9_-]/g, "-").toLowerCase();
  const data = series.points.map((point) => ({
    time: new Date(point.timestamp).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    }),
    value: Math.round(point.value * 10) / 10,
  }));

  if (data.length === 0) {
    return (
      <div className="flex h-[220px] items-center justify-center text-sm text-muted-foreground">
        Waiting for metrics…
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height={220}>
      <AreaChart data={data}>
        <defs>
          <linearGradient id={`gradient-${chartId}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={color} stopOpacity={0.3} />
            <stop offset="95%" stopColor={color} stopOpacity={0} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.26 0.02 260)" />
        <XAxis
          dataKey="time"
          tick={{ fontSize: 11, fill: "oklch(0.65 0.02 260)" }}
          tickLine={false}
          axisLine={false}
          minTickGap={24}
        />
        <YAxis
          domain={domain ?? ["auto", "auto"]}
          tick={{ fontSize: 11, fill: "oklch(0.65 0.02 260)" }}
          tickLine={false}
          axisLine={false}
          unit={series.unit === "%" ? "%" : undefined}
        />
        <Tooltip
          contentStyle={{
            backgroundColor: "oklch(0.17 0.012 260)",
            border: "1px solid oklch(0.26 0.02 260)",
            borderRadius: "8px",
            fontSize: "12px",
          }}
          formatter={(value) => [`${value} ${series.unit}`, series.name]}
        />
        <Area
          type="monotone"
          dataKey="value"
          stroke={color}
          fill={`url(#gradient-${chartId})`}
          strokeWidth={2}
          dot={false}
          isAnimationActive={false}
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}

export function MetricsPage() {
  const { snapshot } = useMetricsStore();
  const { metrics: daskMetrics } = useDaskStore();

  if (!snapshot) {
    return (
      <div>
        <PageHeader title="Metrics" description="Real-time cluster performance" />
        <p className="text-sm text-muted-foreground">Loading metrics...</p>
      </div>
    );
  }

  const charts = [
    { series: snapshot.cpu, color: "#818cf8", domain: [0, 100] as [number, number] },
    { series: snapshot.memory, color: "#34d399", domain: [0, 100] as [number, number] },
    { series: snapshot.network, color: "#38bdf8" },
    { series: snapshot.disk, color: "#fbbf24" },
  ];

  return (
    <div>
      <PageHeader
        title="Metrics"
        description="Real-time cluster performance monitoring"
      />

      <div className="mb-6">
        <h2 className="mb-3 text-sm font-medium text-muted-foreground">
          Dask Cluster
        </h2>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <StatCard
            title="Worker CPU"
            value={`${(daskMetrics?.workerCpu ?? 0).toFixed(1)}%`}
            icon={Cpu}
          />
          <StatCard
            title="Worker Memory"
            value={`${(daskMetrics?.workerMemory ?? 0).toFixed(1)}%`}
            icon={HardDrive}
          />
          <StatCard
            title="Worker Load"
            value={`${(daskMetrics?.workerLoad ?? 0).toFixed(1)}%`}
            subtitle={`${daskMetrics?.workerCount ?? 0} workers`}
            icon={Activity}
          />
          <StatCard
            title="Data Transfer"
            value={`${(daskMetrics?.dataTransfer ?? 0).toFixed(0)} B/s`}
            subtitle={`${(daskMetrics?.tasksPerSec ?? 0).toFixed(1)} tasks/s`}
            icon={daskMetrics ? Network : Zap}
          />
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        {charts.map(({ series, color, domain }) => (
          <Card key={series.name} className="border-border/60 bg-card/80">
            <CardHeader className="pb-2">
              <CardTitle className="text-base">
                {series.name}{" "}
                <span className="text-sm font-normal text-muted-foreground">
                  ({series.unit})
                </span>
              </CardTitle>
            </CardHeader>
            <CardContent>
              <MetricChart series={series} color={color} domain={domain} />
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
