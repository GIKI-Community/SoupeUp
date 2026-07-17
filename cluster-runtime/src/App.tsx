import { HashRouter, Navigate, Route, Routes } from "react-router-dom";

import { AppLayout } from "./layouts/app-layout";
import { ClusterPage } from "./pages/cluster-page";
import { ComputePage } from "./pages/compute-page";
import { DashboardPage } from "./pages/dashboard-page";
import { JobsPage } from "./pages/jobs-page";
import { LogsPage } from "./pages/logs-page";
import { MetricsPage } from "./pages/metrics-page";
import { NodesPage } from "./pages/nodes-page";
import { PluginsPage } from "./pages/plugins-page";
import { SettingsPage } from "./pages/settings-page";

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/cluster" element={<ClusterPage />} />
          <Route path="/compute" element={<ComputePage />} />
          <Route path="/nodes" element={<NodesPage />} />
          <Route path="/jobs" element={<JobsPage />} />
          <Route path="/plugins" element={<PluginsPage />} />
          <Route path="/metrics" element={<MetricsPage />} />
          <Route path="/logs" element={<LogsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
