import { ArrowUpDown, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { ClusterTopologyDiagram } from "@/components/cluster-topology-diagram";
import { NodeStatusBadge } from "@/components/status-badges";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { PageHeader } from "@/layouts/app-layout";
import { cn, formatPercent, formatRelativeTime } from "@/lib/utils";
import { useNodesStore } from "@/stores";
import type { Node, NodeStatus } from "@/types";

const platformLabels: Record<Node["platform"], string> = {
  windows: "Windows",
  linux: "Linux",
  macOS: "macOS",
  android: "Android",
  raspberryPi: "Raspberry Pi",
  other: "Other",
};

const sortableColumns: { key: keyof Node; label: string }[] = [
  { key: "name", label: "Name" },
  { key: "platform", label: "Platform" },
  { key: "status", label: "Status" },
  { key: "cpuPercent", label: "CPU" },
  { key: "memoryPercent", label: "Memory" },
  { key: "backend", label: "Backend" },
  { key: "version", label: "Version" },
  { key: "lastSeen", label: "Last Seen" },
];

export function NodesPage() {
  const {
    nodes,
    summary,
    search,
    statusFilter,
    sortField,
    sortDirection,
    setSearch,
    setStatusFilter,
    setSort,
    fetchNodes,
    fetchSummary,
  } = useNodesStore();

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  useEffect(() => {
    void fetchNodes();
    void fetchSummary();
    const timer = window.setInterval(() => {
      void fetchNodes();
      void fetchSummary();
    }, 2500);
    return () => window.clearInterval(timer);
  }, [fetchNodes, fetchSummary]);

  // Drop selection if the node disappeared from the latest poll.
  useEffect(() => {
    if (selectedNodeId && !nodes.some((n) => n.id === selectedNodeId)) {
      setSelectedNodeId(null);
    }
  }, [nodes, selectedNodeId]);

  const filteredNodes = useMemo(() => {
    return nodes
      .filter((node) => {
        const matchesSearch =
          search === "" ||
          node.name.toLowerCase().includes(search.toLowerCase()) ||
          node.backend.toLowerCase().includes(search.toLowerCase());
        const matchesStatus =
          statusFilter === "all" || node.status === statusFilter;
        return matchesSearch && matchesStatus;
      })
      .sort((a, b) => {
        const aVal = a[sortField];
        const bVal = b[sortField];
        const direction = sortDirection === "asc" ? 1 : -1;

        if (typeof aVal === "string" && typeof bVal === "string") {
          return aVal.localeCompare(bVal) * direction;
        }
        if (typeof aVal === "number" && typeof bVal === "number") {
          return (aVal - bVal) * direction;
        }
        return 0;
      });
  }, [nodes, search, statusFilter, sortField, sortDirection]);

  return (
    <div>
      <PageHeader
        title="Nodes"
        description="Manage compute nodes across your cluster"
      />

      {/* Cluster Summary Cards */}
      {summary && (
        <div className="mb-6 grid grid-cols-2 gap-4 sm:grid-cols-4 lg:grid-cols-7">
          <div className="rounded-lg border border-border/60 bg-card/80 p-4 text-center">
            <div className="text-2xl font-bold">{summary.total_nodes}</div>
            <div className="text-xs text-muted-foreground">Total Nodes</div>
          </div>
          <div className="rounded-lg border border-border/60 bg-card/80 p-4 text-center">
            <div className="text-2xl font-bold text-green-500">{summary.online_nodes}</div>
            <div className="text-xs text-muted-foreground">Online</div>
          </div>
          <div className="rounded-lg border border-border/60 bg-card/80 p-4 text-center">
            <div className="text-2xl font-bold">{summary.total_cpus}</div>
            <div className="text-xs text-muted-foreground">Total CPUs</div>
          </div>
          <div className="rounded-lg border border-border/60 bg-card/80 p-4 text-center">
            <div className="text-2xl font-bold">{(summary.total_ram / (1024 * 1024 * 1024)).toFixed(1)} GB</div>
            <div className="text-xs text-muted-foreground">Total RAM</div>
          </div>
          <div className="rounded-lg border border-border/60 bg-card/80 p-4 text-center">
            <div className="text-2xl font-bold">{summary.total_gpus}</div>
            <div className="text-xs text-muted-foreground">Total GPUs</div>
          </div>
          <div className="rounded-lg border border-border/60 bg-card/80 p-4 text-center">
            <div className="text-2xl font-bold">{summary.total_workers}</div>
            <div className="text-xs text-muted-foreground">Total Workers</div>
          </div>
          <div className="rounded-lg border border-border/60 bg-card/80 p-4 text-center">
            <div className="text-2xl font-bold text-blue-500">{summary.total_available_compute.toFixed(0)}%</div>
            <div className="text-xs text-muted-foreground">Available Compute</div>
          </div>
        </div>
      )}

      <ClusterTopologyDiagram
        nodes={nodes}
        selectedId={selectedNodeId}
        onSelect={setSelectedNodeId}
      />

      <div className="mb-6 flex flex-col gap-4 sm:flex-row">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search nodes..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select
          value={statusFilter}
          onValueChange={(value) =>
            setStatusFilter(value as NodeStatus | "all")
          }
        >
          <SelectTrigger className="w-full sm:w-44">
            <SelectValue placeholder="Filter status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All statuses</SelectItem>
            <SelectItem value="online">Online</SelectItem>
            <SelectItem value="offline">Offline</SelectItem>
            <SelectItem value="degraded">Degraded</SelectItem>
            <SelectItem value="maintenance">Maintenance</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-xl border border-border/60 bg-card/80">
        <Table>
          <TableHeader>
            <TableRow>
              {sortableColumns.map((col) => (
                <TableHead key={col.key}>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="-ml-3 h-8"
                    onClick={() => setSort(col.key)}
                  >
                    {col.label}
                    <ArrowUpDown className="ml-1 h-3.5 w-3.5" />
                  </Button>
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredNodes.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={sortableColumns.length}
                  className="py-8 text-center text-sm text-muted-foreground"
                >
                  No nodes connected. Start the Dask scheduler and a worker on the Cluster page.
                </TableCell>
              </TableRow>
            ) : (
              filteredNodes.map((node) => (
                <TableRow
                  key={node.id}
                  className={cn(
                    "cursor-pointer",
                    selectedNodeId === node.id && "bg-muted/60",
                  )}
                  onClick={() =>
                    setSelectedNodeId(
                      selectedNodeId === node.id ? null : node.id,
                    )
                  }
                  data-state={selectedNodeId === node.id ? "selected" : undefined}
                >
                  <TableCell className="font-medium">{node.name}</TableCell>
                  <TableCell>{platformLabels[node.platform]}</TableCell>
                  <TableCell>
                    <NodeStatusBadge status={node.status} />
                  </TableCell>
                  <TableCell>{formatPercent(node.cpuPercent)}</TableCell>
                  <TableCell>{formatPercent(node.memoryPercent)}</TableCell>
                  <TableCell className="font-mono text-xs">{node.backend}</TableCell>
                  <TableCell className="font-mono text-xs">{node.version}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {formatRelativeTime(node.lastSeen)}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
