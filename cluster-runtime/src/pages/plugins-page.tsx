import { Download, Loader2, Play, Settings2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { openUrl } from "@tauri-apps/plugin-opener";

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
import type { Plugin, PluginUpdateCheck, PythonRuntimeHealth } from "@/types";

const PYTHON_RUNTIME_ID = "plugin-python-runtime";

const SETTINGS_TAB_BY_PLUGIN: Record<string, string> = {
  "plugin-python-runtime": "python",
  "plugin-dask-scheduler": "dask",
  "plugin-ray": "ray",
};

function pythonNeedsInstall(
  plugin: Plugin | undefined,
  health: PythonRuntimeHealth | null,
) {
  if (!plugin) return true;
  if (plugin.status === "running") {
    return health?.status === "failed";
  }
  // Avoid nagging while auto-start is still in progress.
  if (plugin.status === "initializing") {
    return health?.status === "failed";
  }
  return (
    plugin.status === "error" ||
    plugin.status === "discovered" ||
    plugin.status === "disabled" ||
    health?.status === "failed"
  );
}

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

function InstallPythonButton({
  busy,
  onClick,
  size = "sm",
}: {
  busy: boolean;
  onClick: () => void;
  size?: "sm" | "default";
}) {
  return (
    <Button size={size} disabled={busy} onClick={onClick}>
      {busy ? (
        <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
      ) : (
        <Download className="mr-1.5 h-3.5 w-3.5" />
      )}
      {busy ? "Installing Python…" : "Install Python"}
    </Button>
  );
}

function PythonRuntimeDetail({
  needsInstall,
  onInstall,
}: {
  needsInstall: boolean;
  onInstall: () => void;
}) {
  const {
    health,
    packages,
    isExecuting,
    isInstalling,
    isEnsuring,
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
    if (!needsInstall) {
      void fetchPackages();
    }
  }, [fetchHealth, fetchPackages, needsInstall]);

  const ready = health?.status === "ready" || health?.status === "degraded";

  return (
    <div className="mt-4 space-y-4 border-t border-border/60 pt-4">
      {needsInstall && (
        <div className="rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-3 text-sm">
          <p className="font-medium text-amber-700 dark:text-amber-300">
            Python is not ready
          </p>
          <p className="mt-1 text-muted-foreground">
            Download a compatible interpreter into the app data directory (or use
            system Python if available), then start the runtime. First run needs
            network access and may take a minute.
          </p>
          <div className="mt-3">
            <InstallPythonButton busy={isEnsuring} onClick={onInstall} />
          </div>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        {health?.pythonVersion && (
          <Badge variant="secondary">Python {health.pythonVersion}</Badge>
        )}
        {health && (
          <Badge variant={healthVariant(health.status)}>
            {health.status}
          </Badge>
        )}
        {health?.isBundled && <Badge variant="outline">managed</Badge>}
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
            {isLoading ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              "Refresh"
            )}
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
                    <TableCell className="font-mono text-xs">
                      {pkg.version}
                    </TableCell>
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

function UpdateNotice({ check }: { check: PluginUpdateCheck }) {
  if (check.recommendation === "none") {
    return (
      <p className="mt-3 rounded-md border border-border/60 bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
        {check.message}
      </p>
    );
  }

  if (check.recommendation === "pluginUpdate") {
    return (
      <div className="mt-3 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-sm">
        <p className="font-medium text-emerald-700 dark:text-emerald-300">
          Update available (compatible)
        </p>
        <p className="mt-1 text-muted-foreground">
          v{check.installedVersion}
          {check.availableVersion ? ` → v${check.availableVersion}` : ""} —{" "}
          {check.message}
        </p>
        <p className="mt-1 text-xs text-muted-foreground">
          Apply is not automated yet; reinstall from a newer package when ready.
        </p>
      </div>
    );
  }

  return (
    <div className="mt-3 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm">
      <p className="font-medium text-amber-700 dark:text-amber-300">
        App update required
      </p>
      <p className="mt-1 text-muted-foreground">{check.message}</p>
      {check.releaseUrl && (
        <Button
          className="mt-2"
          size="sm"
          variant="outline"
          onClick={() => void openUrl(check.releaseUrl!)}
        >
          Open app release
        </Button>
      )}
    </div>
  );
}

function PluginCard({
  plugin,
  expanded,
  onToggle,
  updateCheck,
  busy,
  showInstallPython,
  installBusy,
  onInstallPython,
  onEnable,
  onDisable,
  onCheckUpdate,
  onUninstall,
  onSettings,
}: {
  plugin: Plugin;
  expanded: boolean;
  onToggle: () => void;
  updateCheck?: PluginUpdateCheck;
  busy: boolean;
  showInstallPython: boolean;
  installBusy: boolean;
  onInstallPython: () => void;
  onEnable: () => void;
  onDisable: () => void;
  onCheckUpdate: () => void;
  onUninstall: () => void;
  onSettings: () => void;
}) {
  const isPython = plugin.id === PYTHON_RUNTIME_ID;
  const mandatory = plugin.mandatory === true;
  const enabled = plugin.enabled !== false && plugin.status !== "disabled";
  const incompatible = plugin.status === "incompatible";
  const hasSettings = plugin.id in SETTINGS_TAB_BY_PLUGIN;

  return (
    <Card className="border-border/60 bg-card/80 transition-colors hover:border-border">
      <CardHeader className="flex flex-row items-start justify-between space-y-0">
        <div className="space-y-1">
          <div className="flex flex-wrap items-center gap-2">
            <CardTitle className="text-base">{plugin.name}</CardTitle>
            <PluginStatusBadge status={plugin.status} />
            {mandatory && <Badge variant="secondary">Required</Badge>}
            {plugin.isDefault && <Badge variant="outline">Default</Badge>}
            {plugin.pluginType && (
              <Badge variant="outline">{plugin.pluginType}</Badge>
            )}
          </div>
          <p className="text-xs text-muted-foreground">
            v{plugin.version} · {plugin.author}
            {plugin.appCompat ? ` · app ${plugin.appCompat}` : ""}
          </p>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          {isPython && showInstallPython && (
            <InstallPythonButton busy={installBusy} onClick={onInstallPython} />
          )}
          {isPython && (
            <Button variant="outline" size="sm" onClick={onToggle}>
              {expanded ? "Hide" : "Manage"}
            </Button>
          )}
          {enabled ? (
            <Button
              variant="outline"
              size="sm"
              disabled={busy || mandatory || incompatible}
              title={
                mandatory
                  ? "Required plugins cannot be disabled"
                  : undefined
              }
              onClick={onDisable}
            >
              Disable
            </Button>
          ) : (
            <Button
              variant="outline"
              size="sm"
              disabled={busy || incompatible}
              onClick={onEnable}
            >
              Enable
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={onCheckUpdate}
          >
            Update
          </Button>
          {!mandatory && (
            <Button
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={onUninstall}
            >
              Uninstall
            </Button>
          )}
          {hasSettings && (
            <Button variant="ghost" size="icon" onClick={onSettings}>
              <Settings2 className="h-4 w-4" />
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-muted-foreground">{plugin.description}</p>
        <p className="mt-2 font-mono text-xs text-muted-foreground/70">
          {plugin.id}
        </p>
        {updateCheck && <UpdateNotice check={updateCheck} />}
        {isPython && expanded && (
          <PythonRuntimeDetail
            needsInstall={showInstallPython}
            onInstall={onInstallPython}
          />
        )}
      </CardContent>
    </Card>
  );
}

export function PluginsPage() {
  const navigate = useNavigate();
  const {
    plugins,
    actionError,
    updateChecks,
    fetchPlugins,
    setEnabled,
    install,
    uninstall,
    checkUpdate,
    clearActionError,
  } = usePluginsStore();
  const {
    health,
    isEnsuring,
    error: pythonError,
    fetchHealth,
    ensureRuntime,
  } = usePythonRuntimeStore();
  const [expandedId, setExpandedId] = useState<string | null>(PYTHON_RUNTIME_ID);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [installPath, setInstallPath] = useState("");
  const [showInstall, setShowInstall] = useState(false);

  useEffect(() => {
    void fetchPlugins();
    void fetchHealth();
    const id = window.setInterval(() => {
      void fetchPlugins();
      void fetchHealth();
    }, 3000);
    return () => window.clearInterval(id);
  }, [fetchPlugins, fetchHealth]);

  const pythonPlugin = plugins.find((p) => p.id === PYTHON_RUNTIME_ID);
  const needsPythonInstall = pythonNeedsInstall(pythonPlugin, health);

  const runBusy = async (id: string, fn: () => Promise<unknown>) => {
    setBusyId(id);
    try {
      await fn();
    } finally {
      setBusyId(null);
    }
  };

  const handleInstallPython = () => {
    void (async () => {
      const ok = await ensureRuntime();
      await fetchPlugins();
      if (ok) {
        setExpandedId(PYTHON_RUNTIME_ID);
      }
    })();
  };

  return (
    <div>
      <PageHeader
        title="Plugins"
        description="Execution engines and extensions for your cluster"
        actions={
          <Button
            onClick={() => {
              clearActionError();
              setShowInstall((v) => !v);
            }}
          >
            Install Plugin
          </Button>
        }
      />

      {needsPythonInstall && (
        <Card className="mb-4 border-amber-500/30 bg-amber-500/10">
          <CardContent className="flex flex-wrap items-center justify-between gap-3 pt-6">
            <div className="space-y-1">
              <p className="text-sm font-medium text-amber-700 dark:text-amber-300">
                Python Runtime is not installed
              </p>
              <p className="text-sm text-muted-foreground">
                Install a compatible Python (system or download) so Dask, Ray, and
                jobs can start. Files go under the app data directory.
              </p>
              {pythonError && (
                <p className="text-sm text-destructive">{pythonError}</p>
              )}
            </div>
            <InstallPythonButton
              busy={isEnsuring}
              onClick={handleInstallPython}
              size="default"
            />
          </CardContent>
        </Card>
      )}

      {actionError && (
        <p className="mb-4 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {actionError}
        </p>
      )}

      {showInstall && (
        <Card className="mb-4 border-border/60 bg-card/80">
          <CardHeader>
            <CardTitle className="text-base">Install from folder</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-wrap items-end gap-2">
            <div className="min-w-[16rem] flex-1 space-y-1">
              <p className="text-xs text-muted-foreground">
                Path to a plugin directory containing manifest.toml
              </p>
              <Input
                value={installPath}
                onChange={(e) => setInstallPath(e.target.value)}
                placeholder="C:\path\to\plugin-package"
                className="bg-background font-mono text-sm"
              />
            </div>
            <Button
              disabled={!installPath.trim() || busyId === "__install__"}
              onClick={() => {
                void runBusy("__install__", async () => {
                  const ok = await install(installPath.trim());
                  if (ok) {
                    setInstallPath("");
                    setShowInstall(false);
                  }
                });
              }}
            >
              {busyId === "__install__" ? (
                <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
              ) : null}
              Install
            </Button>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-4">
        {plugins.map((plugin) => (
          <PluginCard
            key={plugin.id}
            plugin={plugin}
            expanded={expandedId === plugin.id}
            updateCheck={updateChecks[plugin.id]}
            busy={busyId === plugin.id}
            showInstallPython={
              plugin.id === PYTHON_RUNTIME_ID && needsPythonInstall
            }
            installBusy={isEnsuring}
            onInstallPython={handleInstallPython}
            onToggle={() =>
              setExpandedId((current) =>
                current === plugin.id ? null : plugin.id,
              )
            }
            onEnable={() => {
              void runBusy(plugin.id, () => setEnabled(plugin.id, true));
            }}
            onDisable={() => {
              void runBusy(plugin.id, () => setEnabled(plugin.id, false));
            }}
            onCheckUpdate={() => {
              void runBusy(plugin.id, () => checkUpdate(plugin.id));
            }}
            onUninstall={() => {
              if (
                !window.confirm(
                  `Uninstall ${plugin.name}? This removes it from the plugins directory.`,
                )
              ) {
                return;
              }
              void runBusy(plugin.id, () => uninstall(plugin.id));
            }}
            onSettings={() => navigate("/settings")}
          />
        ))}
      </div>
    </div>
  );
}
