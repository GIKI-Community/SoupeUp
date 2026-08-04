import { useEffect, useState } from "react";
import {
  Check,
  Copy,
  Link2,
  Loader2,
  Network,
  RefreshCw,
  Unplug,
} from "lucide-react";
import { Link } from "react-router-dom";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { PageHeader } from "@/layouts/app-layout";
import {
  copyToClipboard,
  formatRelativeTime,
  shortEndpointId,
} from "@/lib/utils";
import { useDaskStore, useNetworkStore } from "@/stores";

function CopyButton({ value, label }: { value: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      className="shrink-0"
      title={label ?? "Copy"}
      onClick={() => {
        void copyToClipboard(value).then((ok) => {
          if (ok) {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1500);
          }
        });
      }}
    >
      {copied ? (
        <Check className="mr-1.5 h-3.5 w-3.5 text-emerald-500" />
      ) : (
        <Copy className="mr-1.5 h-3.5 w-3.5" />
      )}
      {copied ? "Copied" : "Copy"}
    </Button>
  );
}

export function NetworkPage() {
  const {
    localEndpointId,
    listenAddrs,
    peers,
    connectDraft,
    isBusy,
    error,
    lastConnected,
    setConnectDraft,
    refresh,
    connect,
  } = useNetworkStore();
  const setJoinAddress = useDaskStore((s) => s.setJoinAddress);
  const saveSettings = useDaskStore((s) => s.saveSettings);
  const settings = useDaskStore((s) => s.settings);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 4000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const useAsDaskScheduler = async (endpointId: string) => {
    setJoinAddress(endpointId);
    if (settings) {
      await saveSettings({
        ...settings,
        schedulerEndpointId: endpointId,
      });
    }
  };

  return (
    <div>
      <PageHeader
        title="Network"
        description="iroh mesh — dial other Cluster Runtime nodes by EndpointId (NAT traversal + relays)."
        actions={
          <Button
            variant="outline"
            size="sm"
            onClick={() => void refresh()}
            disabled={isBusy}
          >
            <RefreshCw className="mr-2 h-3.5 w-3.5" />
            Refresh
          </Button>
        }
      />

      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="border-border/60 bg-card/80">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <Network className="h-4 w-4" />
              This node
            </CardTitle>
            <CardDescription>
              Share this EndpointId with other machines so they can{" "}
              <code className="text-xs">peer connect</code> or join Dask over
              iroh.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {localEndpointId ? (
              <>
                <div className="space-y-2">
                  <Label>EndpointId</Label>
                  <div className="flex gap-2">
                    <Input
                      readOnly
                      className="font-mono text-xs"
                      value={localEndpointId}
                    />
                    <CopyButton value={localEndpointId} />
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Short:{" "}
                    <span className="font-mono">
                      {shortEndpointId(localEndpointId)}
                    </span>
                  </p>
                </div>
                {listenAddrs.length > 0 && (
                  <div className="space-y-2">
                    <Label>Addresses</Label>
                    <ul className="space-y-1 rounded-md border border-border/60 bg-muted/30 p-3 font-mono text-[11px] text-muted-foreground">
                      {listenAddrs.map((a) => (
                        <li key={a} className="break-all">
                          {a}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </>
            ) : (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Unplug className="h-4 w-4" />
                iroh mesh not started yet — wait for bootstrap, or check logs.
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="border-border/60 bg-card/80">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <Link2 className="h-4 w-4" />
              Connect peer
            </CardTitle>
            <CardDescription>
              Paste another node&apos;s EndpointId. Uses n0 relays / hole
              punching — no IP or port required.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="peer-endpoint">Remote EndpointId</Label>
              <Input
                id="peer-endpoint"
                className="font-mono text-xs"
                placeholder="Paste EndpointId…"
                value={connectDraft}
                onChange={(e) => setConnectDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void connect();
                }}
              />
            </div>
            {error && <p className="text-sm text-destructive">{error}</p>}
            {lastConnected && !error && (
              <p className="text-sm text-emerald-600 dark:text-emerald-400">
                Connected to {shortEndpointId(lastConnected)}
              </p>
            )}
            <Button
              type="button"
              disabled={isBusy || !connectDraft.trim()}
              onClick={() => void connect()}
            >
              {isBusy ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Link2 className="mr-2 h-4 w-4" />
              )}
              Connect
            </Button>
          </CardContent>
        </Card>
      </div>

      <Card className="mt-6 border-border/60 bg-card/80">
        <CardHeader>
          <div className="flex items-center justify-between gap-4">
            <div>
              <CardTitle className="text-base">Connected peers</CardTitle>
              <CardDescription>
                Runtimes you&apos;ve dialed or that dialed you. Use a peer as
                the Dask scheduler target to tunnel workers over iroh.
              </CardDescription>
            </div>
            <Badge variant="secondary">{peers.length} peer(s)</Badge>
          </div>
        </CardHeader>
        <CardContent>
          {peers.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No peers yet. Connect with an EndpointId above, or start another
              runtime with{" "}
              <code className="text-xs">--iroh-bootstrap &lt;this-id&gt;</code>.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>EndpointId</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Last seen</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {peers.map((p) => (
                  <TableRow key={p.node_id}>
                    <TableCell className="font-medium">
                      {p.node_name || "—"}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <span
                          className="font-mono text-xs"
                          title={p.node_id}
                        >
                          {shortEndpointId(p.node_id)}
                        </span>
                        <CopyButton value={p.node_id} label="Copy EndpointId" />
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge
                        variant={
                          p.status === "Online" ? "success" : "muted"
                        }
                      >
                        {p.status}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {formatRelativeTime(p.last_heartbeat)}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-2">
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          onClick={() => void useAsDaskScheduler(p.node_id)}
                        >
                          Use for Dask
                        </Button>
                        <Button type="button" variant="ghost" size="sm" asChild>
                          <Link to="/cluster">Cluster</Link>
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
