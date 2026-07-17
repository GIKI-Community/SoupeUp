import { Loader2, Play, Settings2 } from "lucide-react";
import { useEffect, useState } from "react";

import { PluginStatusBadge } from "@/components/status-badges";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { PageHeader } from "@/layouts/app-layout";
import { usePluginsStore, usePythonRuntimeStore } from "@/stores";
import type { Plugin, PythonRuntimeHealth } from "@/types";

const PYTHON_RUNTIME_ID = "plugin-python-runtime";

function healthVariant(
  status: PythonRuntimeHealth["status"],
): "success" | "warning" | "destructive" | "muted" {
  switch (status) {
    case "ready":
      return "success";
    case "initializing":
    case "degraded":
      return "warning";
    case "failed":
      return "destructive";
    default:
      return "muted";
  }
}

function PythonRuntimeDetail() {
  const {
    health,
    packages,
    isExecuting,
    isInstalling,
    isLoading,
    lastResult,
    error,
    fetchHealth,
    fetchPackages,
    executeCode,
    installPackage,
  } = usePythonRuntimeStore();

  const [code, setCode] = useState('print("Hello World")');
  const [packageName, setPackageName] = useState("");

  useEffect(() => {
    void fetchHealth();
    void fetchPackages();
  }, [fetchHealth, fetchPackages]);

  const ready = health?.status === "ready" || health?.status === "degraded";

  return (
    <div className="mt-4 space-y-4 border-t border-border/60 pt-4">
      <div className="flex flex-wrap items-center gap-2">
        {health?.pythonVersion && (
          <Badge variant="secondary">Python {health.pythonVersion}</Badge>
        )}
        {health && (
          <Badge variant={healthVariant(health.status)}>
            {health.status}
          </Badge>
        )}
        {health?.isBundled && <Badge variant="outline">bundled</Badge>}
      </div>

      <div className="grid gap-2 text-sm sm:grid-cols-2">
        <div>
          <p className="text-xs text-muted-foreground">Active environment</p>
          <p className="font-medium">{health?.activeEnvironment ?? "—"}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Environment path</p>
          <p className="break-all font-mono text-xs text-muted-foreground">
            {health?.environmentPath ?? "—"}
          </p>
        </div>
      </div>

      {error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      <div>
        <div className="mb-2 flex items-center justify-between gap-2">
          <h4 className="text-sm font-medium">Installed packages</h4>
          <Button
            variant="ghost"
            size="sm"
            disabled={!ready || isLoading}
            onClick={() => void fetchPackages()}
          >
            {isLoading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : "Refresh"}
          </Button>
        </div>
        <div className="mb-3 flex gap-2">
          <Input
            placeholder="Package name (e.g. requests)"
            value={packageName}
            onChange={(e) => setPackageName(e.target.value)}
            className="max-w-xs bg-background"
            disabled={!ready || isInstalling}
          />
          <Button
            size="sm"
            disabled={!ready || isInstalling || !packageName.trim()}
            onClick={() => {
              void installPackage(packageName.trim()).then((ok) => {
                if (ok) setPackageName("");
              });
            }}
          >
            {isInstalling ? (
              <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
            ) : null}
            Install
          </Button>
        </div>
        <div className="max-h-48 overflow-auto rounded-md border border-border/60">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Version</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {packages.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={2} className="text-muted-foreground">
                    {isLoading ? "Loading…" : "No packages installed"}
                  </TableCell>
                </TableRow>
              ) : (
                packages.map((pkg) => (
                  <TableRow key={pkg.name}>
                    <TableCell className="font-mono text-xs">{pkg.name}</TableCell>
                    <TableCell className="font-mono text-xs">{pkg.version}</TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </div>
      </div>

      <div>
        <h4 className="mb-2 text-sm font-medium">Quick Execute</h4>
        <textarea
          value={code}
          onChange={(e) => setCode(e.target.value)}
          rows={5}
          spellCheck={false}
          disabled={!ready || isExecuting}
          className="w-full resize-y rounded-md border border-input bg-background px-3 py-2 font-mono text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
        />
        <div className="mt-2 flex justify-end">
          <Button
            size="sm"
            disabled={!ready || isExecuting || !code.trim()}
            onClick={() => void executeCode(code)}
          >
            {isExecuting ? (
              <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
            ) : (
              <Play className="mr-1.5 h-3.5 w-3.5" />
            )}
            Run
          </Button>
        </div>
        {lastResult && (
          <div className="mt-3 space-y-2">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Badge variant={lastResult.success ? "success" : "destructive"}>
                exit {lastResult.exitCode}
              </Badge>
              <span>{lastResult.executionTimeMs} ms</span>
            </div>
            {lastResult.stdout && (
              <pre className="overflow-auto rounded-md bg-muted/50 p-3 font-mono text-xs whitespace-pre-wrap">
                {lastResult.stdout}
              </pre>
            )}
            {(lastResult.stderr || lastResult.exception) && (
              <pre className="overflow-auto rounded-md border border-destructive/30 bg-destructive/10 p-3 font-mono text-xs text-destructive whitespace-pre-wrap">
                {lastResult.exception ?? lastResult.stderr}
              </pre>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function PluginCard({
  plugin,
  expanded,
  onToggle,
}: {
  plugin: Plugin;
  expanded: boolean;
  onToggle: () => void;
}) {
  const isPython = plugin.id === PYTHON_RUNTIME_ID;

  return (
    <Card className="border-border/60 bg-card/80 transition-colors hover:border-border">
      <CardHeader className="flex flex-row items-start justify-between space-y-0">
        <div className="space-y-1">
          <div className="flex items-center gap-3">
            <CardTitle className="text-base">{plugin.name}</CardTitle>
            <PluginStatusBadge status={plugin.status} />
          </div>
          <p className="text-xs text-muted-foreground">
            v{plugin.version} · {plugin.author}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {isPython && (
            <Button variant="outline" size="sm" onClick={onToggle}>
              {expanded ? "Hide" : "Manage"}
            </Button>
          )}
          {plugin.status === "disabled" ? (
            <Button variant="outline" size="sm">
              Enable
            </Button>
          ) : (
            <Button variant="outline" size="sm">
              Disable
            </Button>
          )}
          <Button variant="outline" size="sm">
            Update
          </Button>
          <Button variant="ghost" size="icon">
            <Settings2 className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-muted-foreground">{plugin.description}</p>
        <p className="mt-2 font-mono text-xs text-muted-foreground/70">
          {plugin.id}
        </p>
        {isPython && expanded && <PythonRuntimeDetail />}
      </CardContent>
    </Card>
  );
}

export function PluginsPage() {
  const { plugins, fetchPlugins } = usePluginsStore();
  const [expandedId, setExpandedId] = useState<string | null>(PYTHON_RUNTIME_ID);

  useEffect(() => {
    void fetchPlugins();
    const id = window.setInterval(() => {
      void fetchPlugins();
    }, 3000);
    return () => window.clearInterval(id);
  }, [fetchPlugins]);

  return (
    <div>
      <PageHeader
        title="Plugins"
        description="Execution engines and extensions for your cluster"
        actions={<Button>Install Plugin</Button>}
      />

      <div className="grid gap-4">
        {plugins.map((plugin) => (
          <PluginCard
            key={plugin.id}
            plugin={plugin}
            expanded={expandedId === plugin.id}
            onToggle={() =>
              setExpandedId((current) =>
                current === plugin.id ? null : plugin.id,
              )
            }
          />
        ))}
      </div>
    </div>
  );
}
