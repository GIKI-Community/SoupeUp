import { useEffect, useState } from "react";

import { PageHeader } from "@/layouts/app-layout";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { useDaskStore, useJobsStore, usePythonRuntimeStore, useRayStore, useSettingsStore } from "@/stores";
import type { DaskSettings, RaySettings, UpdateCheckResult } from "@/types";
import { UpdateApi } from "@/api";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

function DefaultSchedulerSettings() {
  const {
    activeScheduler,
    fetchActiveScheduler,
    setActiveScheduler,
  } = useJobsStore();
  const [schedulers, setSchedulers] = useState<
    { pluginId: string; displayName: string }[]
  >([]);

  useEffect(() => {
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
  }, [fetchActiveScheduler]);

  return (
    <div className="space-y-2">
      <Label>Default Scheduler</Label>
      <Select
        value={activeScheduler ?? undefined}
        onValueChange={(v) => void setActiveScheduler(v)}
      >
        <SelectTrigger className="max-w-md bg-background">
          <SelectValue placeholder="Select scheduler" />
        </SelectTrigger>
        <SelectContent>
          {schedulers.map((s) => (
            <SelectItem key={s.pluginId} value={s.pluginId}>
              {s.displayName}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">
        Jobs submitted through the platform API use this scheduler unless overridden.
      </p>
    </div>
  );
}

function DaskSchedulerSettings() {
  const { settings, isBusy, error, fetchSettings, saveSettings } = useDaskStore();
  const [draft, setDraft] = useState<DaskSettings | null>(null);

  useEffect(() => {
    void fetchSettings();
  }, [fetchSettings]);

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  if (!draft) {
    return (
      <Card className="bg-card/50 border-border/60 shadow-sm">
        <CardHeader>
          <CardTitle>Dask Scheduler</CardTitle>
          <CardDescription>Loading settings…</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  const update = <K extends keyof DaskSettings>(key: K, value: DaskSettings[K]) => {
    setDraft({ ...draft, [key]: value });
  };

  return (
    <Card className="bg-card/50 border-border/60 shadow-sm">
      <CardHeader>
        <CardTitle>Dask Scheduler</CardTitle>
        <CardDescription>
          Configure scheduler bind address, dashboard port, and worker defaults.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <p className="text-sm text-destructive">{error}</p>
        )}
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label>Scheduler Host</Label>
            <Input
              value={draft.schedulerHost}
              onChange={(e) => update("schedulerHost", e.target.value)}
              className="bg-background font-mono text-xs"
            />
          </div>
          <div className="space-y-2">
            <Label>Scheduler Port</Label>
            <Input
              type="number"
              value={draft.schedulerPort}
              onChange={(e) => update("schedulerPort", Number(e.target.value))}
              className="bg-background"
            />
          </div>
          <div className="space-y-2">
            <Label>Dashboard Port</Label>
            <Input
              type="number"
              value={draft.dashboardPort}
              onChange={(e) => update("dashboardPort", Number(e.target.value))}
              className="bg-background"
            />
          </div>
          <div className="space-y-2">
            <Label>Scheduler Address (workers join this)</Label>
            <Input
              value={draft.schedulerAddress}
              onChange={(e) => update("schedulerAddress", e.target.value)}
              className="bg-background font-mono text-xs"
              placeholder="tcp://192.168.1.10:8786"
            />
          </div>
          <div className="space-y-2">
            <Label>Worker Threads (0 = auto)</Label>
            <Input
              type="number"
              value={draft.workerThreads}
              onChange={(e) => update("workerThreads", Number(e.target.value))}
              className="bg-background"
            />
          </div>
          <div className="space-y-2">
            <Label>Worker Memory Limit</Label>
            <Input
              value={draft.workerMemoryLimit}
              onChange={(e) => update("workerMemoryLimit", e.target.value)}
              className="bg-background"
              placeholder="4GB"
            />
          </div>
          <div className="space-y-2">
            <Label>Worker Name</Label>
            <Input
              value={draft.workerName}
              onChange={(e) => update("workerName", e.target.value)}
              className="bg-background"
            />
          </div>
          <div className="space-y-2">
            <Label>Local Directory</Label>
            <Input
              value={draft.localDirectory}
              onChange={(e) => update("localDirectory", e.target.value)}
              className="bg-background font-mono text-xs"
            />
          </div>
          <div className="space-y-2">
            <Label>Logging Level</Label>
            <Input
              value={draft.loggingLevel}
              onChange={(e) => update("loggingLevel", e.target.value)}
              className="bg-background"
              placeholder="info"
            />
          </div>
        </div>
        <Button
          disabled={isBusy}
          onClick={() => void saveSettings(draft)}
        >
          Save Dask Settings
        </Button>
      </CardContent>
    </Card>
  );
}

function RaySchedulerSettings() {
  const { settings, isBusy, error, fetchSettings, saveSettings } = useRayStore();
  const [draft, setDraft] = useState<RaySettings | null>(null);

  useEffect(() => {
    void fetchSettings();
  }, [fetchSettings]);

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  if (!draft) {
    return (
      <Card className="bg-card/50 border-border/60 shadow-sm">
        <CardHeader>
          <CardTitle>Ray</CardTitle>
          <CardDescription>Loading settings…</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  const update = <K extends keyof RaySettings>(key: K, value: RaySettings[K]) => {
    setDraft({ ...draft, [key]: value });
  };

  return (
    <Card className="bg-card/50 border-border/60 shadow-sm">
      <CardHeader>
        <CardTitle>Ray</CardTitle>
        <CardDescription>
          Configure Ray head bind address, dashboard port, and worker defaults.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <p className="text-sm text-destructive">{error}</p>
        )}
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label>Head Host</Label>
            <Input
              value={draft.headHost}
              onChange={(e) => update("headHost", e.target.value)}
              className="bg-background font-mono text-xs"
            />
          </div>
          <div className="space-y-2">
            <Label>GCS Port</Label>
            <Input
              type="number"
              value={draft.gcsPort}
              onChange={(e) => update("gcsPort", Number(e.target.value))}
              className="bg-background"
            />
          </div>
          <div className="space-y-2">
            <Label>Dashboard Port</Label>
            <Input
              type="number"
              value={draft.dashboardPort}
              onChange={(e) => update("dashboardPort", Number(e.target.value))}
              className="bg-background"
            />
          </div>
          <div className="space-y-2">
            <Label>Head Address (workers join this)</Label>
            <Input
              value={draft.headAddress}
              onChange={(e) => update("headAddress", e.target.value)}
              className="bg-background font-mono text-xs"
              placeholder="192.168.1.10:6379"
            />
          </div>
          <div className="space-y-2">
            <Label>Worker CPUs (0 = auto)</Label>
            <Input
              type="number"
              value={draft.workerCpus}
              onChange={(e) => update("workerCpus", Number(e.target.value))}
              className="bg-background"
            />
          </div>
          <div className="space-y-2">
            <Label>Object Store Memory</Label>
            <Input
              value={draft.objectStoreMemory}
              onChange={(e) => update("objectStoreMemory", e.target.value)}
              className="bg-background"
              placeholder="2GB"
            />
          </div>
          <div className="space-y-2">
            <Label>Worker Name</Label>
            <Input
              value={draft.workerName}
              onChange={(e) => update("workerName", e.target.value)}
              className="bg-background"
            />
          </div>
          <div className="space-y-2">
            <Label>Logging Level</Label>
            <Input
              value={draft.loggingLevel}
              onChange={(e) => update("loggingLevel", e.target.value)}
              className="bg-background"
              placeholder="info"
            />
          </div>
        </div>
        <Button
          disabled={isBusy}
          onClick={() => void saveSettings(draft)}
        >
          Save Ray Settings
        </Button>
      </CardContent>
    </Card>
  );
}

function PythonRuntimeSettings() {
  const { health, packageIndex, fetchHealth, fetchPackageIndex } =
    usePythonRuntimeStore();

  useEffect(() => {
    void fetchHealth();
    void fetchPackageIndex();
  }, [fetchHealth, fetchPackageIndex]);

  const environmentsRoot = health?.environmentPath
    ? health.environmentPath.replace(/[\\/][^\\/]+$/, "")
    : null;
  const packageCachePath = environmentsRoot
    ? `${environmentsRoot}${environmentsRoot.includes("\\") ? "\\" : "/"}cache`
    : null;

  return (
    <Card className="bg-card/50 border-border/60 shadow-sm">
      <CardHeader>
        <div className="flex items-center gap-3">
          <CardTitle>Python Runtime</CardTitle>
          {health && (
            <Badge
              variant={
                health.status === "ready"
                  ? "success"
                  : health.status === "failed"
                    ? "destructive"
                    : "warning"
              }
            >
              {health.status}
            </Badge>
          )}
        </div>
        <CardDescription>
          Read-only view of the embedded Python interpreter and managed
          environments.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>Interpreter path</Label>
          <Input
            readOnly
            value={health?.interpreterPath ?? "Not available"}
            className="max-w-2xl bg-background font-mono text-xs"
          />
        </div>
        <div className="space-y-2">
          <Label>Active environment</Label>
          <Input
            readOnly
            value={
              health?.environmentPath
                ? `${health.activeEnvironment ?? "default"} — ${health.environmentPath}`
                : "Not available"
            }
            className="max-w-2xl bg-background font-mono text-xs"
          />
        </div>
        <div className="space-y-2">
          <Label>Package cache path</Label>
          <Input
            readOnly
            value={packageCachePath ?? "Not available"}
            className="max-w-2xl bg-background font-mono text-xs"
          />
        </div>
        <div className="space-y-2">
          <Label>Package index URL</Label>
          <Input
            readOnly
            value={packageIndex ?? "https://pypi.org/simple"}
            className="max-w-2xl bg-background font-mono text-xs"
          />
        </div>
        {health?.pythonVersion && (
          <p className="text-sm text-muted-foreground">
            Python {health.pythonVersion}
            {health.isBundled ? " (bundled)" : " (system)"}
          </p>
        )}
      </CardContent>
    </Card>
  );
}

export function SettingsPage() {
  return (
    <div>
      <PageHeader
        title="Settings"
        description="Configure your cluster runtime and preferences"
      />

      <Tabs defaultValue="general" className="w-full">
        <TabsList className="mb-4 w-full justify-start overflow-x-auto bg-muted/50">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="appearance">Appearance</TabsTrigger>
          <TabsTrigger value="networking">Networking</TabsTrigger>
          <TabsTrigger value="python">Python Runtime</TabsTrigger>
          <TabsTrigger value="dask">Dask Scheduler</TabsTrigger>
          <TabsTrigger value="ray">Ray</TabsTrigger>
          <TabsTrigger value="plugins">Plugins</TabsTrigger>
          <TabsTrigger value="security">Security</TabsTrigger>
          <TabsTrigger value="updates">Updates</TabsTrigger>
        </TabsList>

        <TabsContent value="general">
          <Card className="border-border/60 bg-card/50 shadow-sm">
            <CardHeader>
              <CardTitle>General Settings</CardTitle>
              <CardDescription>
                Basic configuration for the cluster runtime.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <DefaultSchedulerSettings />
              <div className="space-y-2">
                <Label htmlFor="cluster-name">Cluster Name</Label>
                <Input
                  id="cluster-name"
                  placeholder="Default Cluster"
                  className="max-w-md bg-background"
                />
              </div>
              <div className="flex items-center space-x-2 pt-4">
                <Switch id="auto-start" />
                <Label htmlFor="auto-start">
                  Start runtime automatically on system startup
                </Label>
              </div>
              <div className="pt-4">
                <Button>Save Changes</Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="appearance">
          <Card className="border-border/60 bg-card/50 shadow-sm">
            <CardHeader>
              <CardTitle>Appearance</CardTitle>
              <CardDescription>
                Customize how the application looks.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center space-x-2">
                <Switch id="dark-mode" defaultChecked />
                <Label htmlFor="dark-mode">Dark Mode</Label>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="networking">
          <Card className="border-border/60 bg-card/50 shadow-sm">
            <CardHeader>
              <CardTitle>Networking</CardTitle>
              <CardDescription>
                Configure ports and network interfaces.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="bind-address">Bind Address</Label>
                <Input
                  id="bind-address"
                  placeholder="0.0.0.0"
                  className="max-w-md bg-background"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="port">API Port</Label>
                <Input
                  id="port"
                  type="number"
                  placeholder="8080"
                  className="max-w-md bg-background"
                />
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="python">
          <PythonRuntimeSettings />
        </TabsContent>

        <TabsContent value="dask">
          <DaskSchedulerSettings />
        </TabsContent>

        <TabsContent value="ray">
          <RaySchedulerSettings />
        </TabsContent>

        <TabsContent value="plugins">
          <Card className="border-border/60 bg-card/50 shadow-sm">
            <CardHeader>
              <CardTitle>Plugin Security</CardTitle>
              <CardDescription>
                Manage plugin execution policies.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center space-x-2">
                <Switch id="allow-unsigned" />
                <Label htmlFor="allow-unsigned">Allow unsigned plugins</Label>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="security">
          <Card className="border-border/60 bg-card/50 shadow-sm">
            <CardHeader>
              <CardTitle>Security</CardTitle>
              <CardDescription>
                Manage authentication and access control.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center space-x-2">
                <Switch id="enable-auth" defaultChecked />
                <Label htmlFor="enable-auth">
                  Require authentication for API
                </Label>
              </div>
              <div className="pt-4">
                <Button variant="outline">Generate New API Token</Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="updates">
          <UpdatesSettings />
        </TabsContent>
      </Tabs>
    </div>
  );
}

function UpdatesSettings() {
  const autoCheckUpdates =
    useSettingsStore((s) => s.settings.autoCheckUpdates) ?? true;
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<UpdateCheckResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentVersion, setCurrentVersion] = useState<string>("…");

  useEffect(() => {
    void UpdateApi.getVersion()
      .then(setCurrentVersion)
      .catch(() => setCurrentVersion("unknown"));
  }, []);

  const runCheck = async () => {
    setChecking(true);
    setError(null);
    try {
      const r = await UpdateApi.check();
      setResult(r);
    } catch (e) {
      setResult(null);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setChecking(false);
    }
  };

  return (
    <Card className="border-border/60 bg-card/50 shadow-sm">
      <CardHeader>
        <CardTitle>Updates</CardTitle>
        <CardDescription>
          Check GitHub Releases for a newer Cluster Runtime version. Updates are
          notify-only — download and install from the release page.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="text-sm text-muted-foreground">
          Current version:{" "}
          <span className="font-medium text-foreground">v{currentVersion}</span>
        </div>
        <div className="flex items-center space-x-2">
          <Switch
            id="auto-update"
            checked={autoCheckUpdates}
            onCheckedChange={(checked) =>
              updateSettings({ autoCheckUpdates: checked })
            }
          />
          <Label htmlFor="auto-update">
            Check for updates automatically on startup
          </Label>
        </div>
        <div className="flex flex-wrap items-center gap-2 pt-2">
          <Button
            variant="secondary"
            disabled={checking}
            onClick={() => {
              void runCheck();
            }}
          >
            {checking ? "Checking…" : "Check for Updates Now"}
          </Button>
          {result?.updateAvailable && result.releaseUrl && (
            <Button
              onClick={() => {
                void openUrl(result.releaseUrl!);
              }}
            >
              Open release page
            </Button>
          )}
        </div>
        {error && (
          <p className="text-sm text-destructive" role="alert">
            {error}
          </p>
        )}
        {result && !error && (
          <div className="rounded-md border border-border/60 bg-background/60 p-3 text-sm">
            <div className="flex items-center gap-2">
              {result.updateAvailable ? (
                <Badge variant="default">Update available</Badge>
              ) : (
                <Badge variant="secondary">Up to date</Badge>
              )}
              <span>{result.message}</span>
            </div>
            {result.latestVersion && (
              <p className="mt-2 text-muted-foreground">
                Latest release: v{result.latestVersion}
              </p>
            )}
            {result.updateAvailable && result.releaseNotes && (
              <pre className="mt-3 max-h-40 overflow-auto whitespace-pre-wrap text-xs text-muted-foreground">
                {result.releaseNotes}
              </pre>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
