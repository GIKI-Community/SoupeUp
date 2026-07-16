import { useEffect } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/layouts/app-layout";
import { useMetricsStore } from "@/stores";
import type { MetricSeries } from "@/types";

function MetricChart({
  series,
  color,
  domain,
}: {
  series: MetricSeries;
  color: string;
  domain?: [number, number];
}) {
  const data = series.points.map((point) => ({
    time: new Date(point.timestamp).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    }),
    value: Math.round(point.value * 10) / 10,
  }));

  return (
    <ResponsiveContainer width="100%" height={220}>
      <AreaChart data={data}>
        <defs>
          <linearGradient id={`gradient-${series.name}`} x1="0" y1="0" x2="0" y2="1">
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
          fill={`url(#gradient-${series.name})`}
          strokeWidth={2}
          isAnimationActive
          animationDuration={300}
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}

export function MetricsPage() {
  const { snapshot, fetchMetrics, appendAnimatedPoint } = useMetricsStore();

  useEffect(() => {
    void fetchMetrics();
    const interval = setInterval(() => {
      appendAnimatedPoint();
    }, 2000);
    return () => clearInterval(interval);
  }, [fetchMetrics, appendAnimatedPoint]);

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
