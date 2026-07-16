import { PageHeader } from "@/layouts/app-layout";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";

export function SettingsPage() {
  return (
    <div>
      <PageHeader
        title="Settings"
        description="Configure your cluster runtime and preferences"
      />

      <Tabs defaultValue="general" className="w-full">
        <TabsList className="mb-4 bg-muted/50 w-full justify-start overflow-x-auto">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="appearance">Appearance</TabsTrigger>
          <TabsTrigger value="networking">Networking</TabsTrigger>
          <TabsTrigger value="plugins">Plugins</TabsTrigger>
          <TabsTrigger value="security">Security</TabsTrigger>
          <TabsTrigger value="updates">Updates</TabsTrigger>
        </TabsList>
        
        <TabsContent value="general">
          <Card className="bg-card/50 border-border/60 shadow-sm">
            <CardHeader>
              <CardTitle>General Settings</CardTitle>
              <CardDescription>
                Basic configuration for the cluster runtime.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="cluster-name">Cluster Name</Label>
                <Input id="cluster-name" placeholder="Default Cluster" className="max-w-md bg-background" />
              </div>
              <div className="flex items-center space-x-2 pt-4">
                <Switch id="auto-start" />
                <Label htmlFor="auto-start">Start runtime automatically on system startup</Label>
              </div>
              <div className="pt-4">
                <Button>Save Changes</Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="appearance">
          <Card className="bg-card/50 border-border/60 shadow-sm">
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
          <Card className="bg-card/50 border-border/60 shadow-sm">
            <CardHeader>
              <CardTitle>Networking</CardTitle>
              <CardDescription>
                Configure ports and network interfaces.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="bind-address">Bind Address</Label>
                <Input id="bind-address" placeholder="0.0.0.0" className="max-w-md bg-background" />
              </div>
              <div className="space-y-2">
                <Label htmlFor="port">API Port</Label>
                <Input id="port" type="number" placeholder="8080" className="max-w-md bg-background" />
              </div>
            </CardContent>
          </Card>
        </TabsContent>
        
        <TabsContent value="plugins">
          <Card className="bg-card/50 border-border/60 shadow-sm">
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
          <Card className="bg-card/50 border-border/60 shadow-sm">
            <CardHeader>
              <CardTitle>Security</CardTitle>
              <CardDescription>
                Manage authentication and access control.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center space-x-2">
                <Switch id="enable-auth" defaultChecked />
                <Label htmlFor="enable-auth">Require authentication for API</Label>
              </div>
              <div className="pt-4">
                <Button variant="outline">Generate New API Token</Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="updates">
          <Card className="bg-card/50 border-border/60 shadow-sm">
            <CardHeader>
              <CardTitle>Updates</CardTitle>
              <CardDescription>
                Configure automatic updates.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center space-x-2">
                <Switch id="auto-update" defaultChecked />
                <Label htmlFor="auto-update">Check for updates automatically</Label>
              </div>
              <div className="pt-4">
                <Button variant="secondary">Check for Updates Now</Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
