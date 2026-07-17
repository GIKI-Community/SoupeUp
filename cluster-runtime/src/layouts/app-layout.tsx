import { useEffect } from "react";
import { Outlet } from "react-router-dom";

import { Sidebar } from "@/components/sidebar";
import { useDaskStore, useMetricsStore } from "@/stores";

interface PageHeaderProps {
  title: string;
  description?: string;
  actions?: React.ReactNode;
}

export function PageHeader({ title, description, actions }: PageHeaderProps) {
  return (
    <div className="mb-8 flex items-start justify-between gap-4">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        {description && (
          <p className="mt-1 text-sm text-muted-foreground">{description}</p>
        )}
      </div>
      {actions && <div className="flex items-center gap-2">{actions}</div>}
    </div>
  );
}

export function AppLayout() {
  const fetchMetrics = useMetricsStore((s) => s.fetchMetrics);
  const appendAnimatedPoint = useMetricsStore((s) => s.appendAnimatedPoint);
  const fetchDaskMetrics = useDaskStore((s) => s.fetchMetrics);

  useEffect(() => {
    void fetchMetrics();
    void fetchDaskMetrics();
    const interval = window.setInterval(() => {
      appendAnimatedPoint();
      void fetchDaskMetrics();
    }, 2000);
    return () => window.clearInterval(interval);
  }, [fetchMetrics, appendAnimatedPoint, fetchDaskMetrics]);

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <Sidebar />
      <main className="flex-1 overflow-y-auto scrollbar-thin">
        <div className="mx-auto max-w-7xl p-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
