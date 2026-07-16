import { useState } from "react";
import { Activity } from "lucide-react";
import { PageHeader } from "@/layouts/app-layout";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";

export function ComputePage() {
  const [enabled, setEnabled] = useState(false);
  
  // Dummy values for 0.50 effort mode
  const cpuUsage = 15;
  const ramUsage = 30;
  
  return (
    <div>
      <PageHeader
        title="Compute Sharing"
        description="Offer local resources to the cluster and manage execution limits."
        actions={
          <div className="flex items-center space-x-2">
            <Switch id="offer-compute" checked={enabled} onCheckedChange={setEnabled} />
            <Label htmlFor="offer-compute" className="font-semibold">{enabled ? "Offering Compute" : "Compute Disabled"}</Label>
          </div>
        }
      />
      
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4 mb-4">
        <Card className="border-border/60 bg-card/80">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Node Name</CardTitle>
              <Activity className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">LocalNode</div>
              <p className="text-xs text-muted-foreground">Status: {enabled ? "Online" : "Offline"}</p>
            </CardContent>
        </Card>
        <Card className="border-border/60 bg-card/80">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">CPU Usage</CardTitle>
              <Activity className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{cpuUsage}%</div>
            </CardContent>
        </Card>
        <Card className="border-border/60 bg-card/80">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">RAM Usage</CardTitle>
              <Activity className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{ramUsage}%</div>
            </CardContent>
        </Card>
        <Card className="border-border/60 bg-card/80">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Active Workers</CardTitle>
              <Activity className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{enabled ? 2 : 0}</div>
            </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card className="border-border/60 bg-card/80">
          <CardHeader>
            <CardTitle>Resource Limits</CardTitle>
            <CardDescription>Configure how much of your local system the cluster can use.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label>Maximum CPU (%)</Label>
              <input type="range" className="w-full" min="10" max="100" defaultValue="80" />
            </div>
            <div className="space-y-2">
              <Label>Maximum RAM (%)</Label>
              <input type="range" className="w-full" min="10" max="100" defaultValue="80" />
            </div>
            <div className="space-y-2">
              <Label>Maximum Workers</Label>
              <input type="range" className="w-full" min="1" max="16" defaultValue="4" />
            </div>
          </CardContent>
        </Card>
        
        <Card className="border-border/60 bg-card/80">
          <CardHeader>
            <CardTitle>Active Workers</CardTitle>
          </CardHeader>
          <CardContent>
            {enabled ? (
              <div className="space-y-2">
                  <div className="flex justify-between border-b pb-2 text-sm text-muted-foreground">
                      <span>Worker</span><span>Task</span><span>State</span>
                  </div>
                  <div className="flex justify-between text-sm">
                      <span>worker-0</span><span>-</span><span className="text-green-500">Idle</span>
                  </div>
                  <div className="flex justify-between text-sm">
                      <span>worker-1</span><span>-</span><span className="text-green-500">Idle</span>
                  </div>
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">Offer compute to start workers.</p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
