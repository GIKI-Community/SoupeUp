import { Settings2 } from "lucide-react";
import { useEffect } from "react";

import { PluginStatusBadge } from "@/components/status-badges";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/layouts/app-layout";
import { usePluginsStore } from "@/stores";

export function PluginsPage() {
  const { plugins, fetchPlugins } = usePluginsStore();

  useEffect(() => {
    void fetchPlugins();
  }, [fetchPlugins]);

  return (
    <div>
      <PageHeader
        title="Plugins"
        description="Execution engines and extensions for your cluster"
        actions={
          <Button>Install Plugin</Button>
        }
      />

      <div className="grid gap-4">
        {plugins.map((plugin) => (
          <Card
            key={plugin.id}
            className="border-border/60 bg-card/80 transition-colors hover:border-border"
          >
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
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
