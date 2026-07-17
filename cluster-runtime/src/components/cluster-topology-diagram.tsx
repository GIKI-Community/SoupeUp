import { useMemo, useState } from "react";
import { Network } from "lucide-react";

import { NodeStatusBadge } from "@/components/status-badges";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { cn, formatPercent } from "@/lib/utils";
import type { Node } from "@/types";

type TopologyRole = "master" | "worker";
type TopologyFamily = "dask" | "ray" | "other";

interface TopologyNode {
  node: Node;
  role: TopologyRole;
  family: TopologyFamily;
}

interface TopologyCluster {
  family: TopologyFamily;
  label: string;
  masters: TopologyNode[];
  workers: TopologyNode[];
}

const FAMILY_LABEL: Record<TopologyFamily, string> = {
  dask: "Dask",
  ray: "Ray",
  other: "Other",
};

function classifyBackend(backend: string): { family: TopologyFamily; role: TopologyRole } {
  const b = backend.toLowerCase();
  if (b.includes("dask-scheduler")) {
    return { family: "dask", role: "master" };
  }
  if (b.includes("dask-worker")) {
    return { family: "dask", role: "worker" };
  }
  if (b.includes("dask")) {
    return {
      family: "dask",
      role: b.includes("scheduler") || b.includes("head") ? "master" : "worker",
    };
  }
  if (b.includes("ray-head")) {
    return { family: "ray", role: "master" };
  }
  if (b.includes("ray-worker")) {
    return { family: "ray", role: "worker" };
  }
  if (b.includes("ray")) {
    return {
      family: "ray",
      role: b.includes("head") || b.includes("gcs") ? "master" : "worker",
    };
  }
  if (b.includes("scheduler") || b.includes("head") || b.includes("master")) {
    return { family: "other", role: "master" };
  }
  return { family: "other", role: "worker" };
}

function buildClusters(nodes: Node[]): TopologyCluster[] {
  const buckets: Record<TopologyFamily, TopologyCluster> = {
    dask: { family: "dask", label: FAMILY_LABEL.dask, masters: [], workers: [] },
    ray: { family: "ray", label: FAMILY_LABEL.ray, masters: [], workers: [] },
    other: { family: "other", label: FAMILY_LABEL.other, masters: [], workers: [] },
  };

  for (const node of nodes) {
    const { family, role } = classifyBackend(node.backend);
    const entry: TopologyNode = { node, role, family };
    if (role === "master") {
      buckets[family].masters.push(entry);
    } else {
      buckets[family].workers.push(entry);
    }
  }

  return (["dask", "ray", "other"] as TopologyFamily[])
    .map((f) => buckets[f])
    .filter((c) => c.masters.length > 0 || c.workers.length > 0);
}

function statusStroke(status: Node["status"]): string {
  switch (status) {
    case "online":
      return "var(--color-emerald-500, #10b981)";
    case "degraded":
      return "var(--color-amber-500, #f59e0b)";
    case "offline":
      return "var(--color-red-500, #ef4444)";
    default:
      return "var(--color-slate-400, #94a3b8)";
  }
}

function shortName(name: string, max = 18): string {
  if (name.length <= max) return name;
  return `${name.slice(0, max - 1)}…`;
}

interface ClusterTopologyDiagramProps {
  nodes: Node[];
  selectedId: string | null;
  onSelect: (nodeId: string | null) => void;
}

export function ClusterTopologyDiagram({
  nodes,
  selectedId,
  onSelect,
}: ClusterTopologyDiagramProps) {
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const clusters = useMemo(() => buildClusters(nodes), [nodes]);
  const selectedNode = nodes.find((n) => n.id === selectedId) ?? null;

  if (nodes.length === 0) {
    return (
      <Card className="mb-6 border-border/60 bg-card/80">
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <Network className="h-4 w-4" />
            Cluster topology
          </CardTitle>
          <CardDescription>
            Master and worker relationships appear here once a scheduler or head is running.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex h-40 items-center justify-center rounded-lg border border-dashed border-border/60 text-sm text-muted-foreground">
            No nodes to diagram yet. Start a cluster on the Cluster page.
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="mb-6 border-border/60 bg-card/80">
      <CardHeader className="pb-3">
        <CardTitle className="flex items-center gap-2 text-base">
          <Network className="h-4 w-4" />
          Cluster topology
        </CardTitle>
        <CardDescription>
          Click a node to inspect it. Masters (schedulers / heads) sit at the hub; workers connect as spokes.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 lg:grid-cols-[1fr_minmax(220px,280px)]">
          <div className="space-y-4">
            {clusters.map((cluster) => (
              <ClusterCanvas
                key={cluster.family}
                cluster={cluster}
                selectedId={selectedId}
                hoveredId={hoveredId}
                onSelect={onSelect}
                onHover={setHoveredId}
              />
            ))}
          </div>

          <div className="rounded-lg border border-border/60 bg-muted/30 p-4 text-sm">
            {selectedNode ? (
              <div className="space-y-3">
                <div>
                  <p className="font-medium leading-snug">{selectedNode.name}</p>
                  <p className="mt-1 font-mono text-xs text-muted-foreground">
                    {selectedNode.backend}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <NodeStatusBadge status={selectedNode.status} />
                  <span className="text-xs text-muted-foreground capitalize">
                    {classifyBackend(selectedNode.backend).role}
                  </span>
                </div>
                <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-xs">
                  <dt className="text-muted-foreground">CPU</dt>
                  <dd>{formatPercent(selectedNode.cpuPercent)}</dd>
                  <dt className="text-muted-foreground">Memory</dt>
                  <dd>{formatPercent(selectedNode.memoryPercent)}</dd>
                  <dt className="text-muted-foreground">Address</dt>
                  <dd className="truncate font-mono" title={selectedNode.version}>
                    {selectedNode.version || "—"}
                  </dd>
                </dl>
                <button
                  type="button"
                  className="text-xs text-muted-foreground underline-offset-2 hover:underline"
                  onClick={() => onSelect(null)}
                >
                  Clear selection
                </button>
              </div>
            ) : (
              <p className="text-muted-foreground">
                Select a master or worker in the diagram to see details.
              </p>
            )}
          </div>
        </div>

        <div className="flex flex-wrap gap-4 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1.5">
            <span className="inline-block h-2.5 w-2.5 rounded-full bg-sky-500" />
            Master
          </span>
          <span className="inline-flex items-center gap-1.5">
            <span className="inline-block h-2.5 w-2.5 rounded-sm bg-emerald-500" />
            Worker
          </span>
          <span className="inline-flex items-center gap-1.5">
            <span
              className="inline-block h-0.5 w-4"
              style={{ background: "var(--color-emerald-500, #10b981)" }}
            />
            Online link
          </span>
          <span className="inline-flex items-center gap-1.5">
            <span
              className="inline-block h-0.5 w-4 opacity-50"
              style={{ background: "var(--color-slate-400, #94a3b8)" }}
            />
            Offline / other
          </span>
        </div>
      </CardContent>
    </Card>
  );
}

interface ClusterCanvasProps {
  cluster: TopologyCluster;
  selectedId: string | null;
  hoveredId: string | null;
  onSelect: (nodeId: string | null) => void;
  onHover: (nodeId: string | null) => void;
}

function ClusterCanvas({
  cluster,
  selectedId,
  hoveredId,
  onSelect,
  onHover,
}: ClusterCanvasProps) {
  const width = 640;
  const height = cluster.workers.length > 6 ? 320 : 260;
  const masterY = 56;
  const workerY = height - 70;
  const masterCx = width / 2;

  const masters =
    cluster.masters.length > 0
      ? cluster.masters
      : ([
          {
            node: {
              id: `${cluster.family}-missing-master`,
              name: "No master running",
              platform: "other",
              status: "offline",
              cpuPercent: 0,
              memoryPercent: 0,
              backend: `${cluster.family}-master`,
              version: "",
              lastSeen: new Date().toISOString(),
            } satisfies Node,
            role: "master" as const,
            family: cluster.family,
          },
        ] as TopologyNode[]);

  const masterPositions = masters.map((m, i) => {
    const spread = Math.min(160, 40 * masters.length);
    const x =
      masters.length === 1
        ? masterCx
        : masterCx - spread / 2 + (spread / Math.max(masters.length - 1, 1)) * i;
    return { ...m, x, y: masterY };
  });

  const workerPositions = cluster.workers.map((w, i) => {
    const n = cluster.workers.length;
    const margin = 48;
    const x =
      n === 1
        ? width / 2
        : margin + ((width - margin * 2) * i) / Math.max(n - 1, 1);
    return { ...w, x, y: workerY };
  });

  const primaryMaster = masterPositions[0];
  const ghostMaster = cluster.masters.length === 0;

  return (
    <div className="overflow-hidden rounded-lg border border-border/60 bg-background/40">
      <div className="border-b border-border/50 px-3 py-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
        {cluster.label}
        <span className="ml-2 font-normal normal-case text-muted-foreground/80">
          {cluster.masters.length} master{cluster.masters.length === 1 ? "" : "s"}
          {" · "}
          {cluster.workers.length} worker{cluster.workers.length === 1 ? "" : "s"}
        </span>
      </div>
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="h-auto w-full select-none"
        role="img"
        aria-label={`${cluster.label} topology: ${cluster.masters.length} masters, ${cluster.workers.length} workers`}
      >
        {/* Links: each worker → primary master */}
        {primaryMaster &&
          workerPositions.map((w) => {
            const active =
              selectedId === w.node.id ||
              selectedId === primaryMaster.node.id ||
              hoveredId === w.node.id ||
              hoveredId === primaryMaster.node.id;
            const online =
              w.node.status === "online" &&
              primaryMaster.node.status === "online" &&
              !ghostMaster;
            return (
              <line
                key={`link-${w.node.id}`}
                x1={primaryMaster.x}
                y1={primaryMaster.y + 28}
                x2={w.x}
                y2={w.y - 22}
                stroke={online ? statusStroke("online") : statusStroke(w.node.status)}
                strokeWidth={active ? 2.5 : 1.5}
                strokeOpacity={active ? 0.95 : online ? 0.55 : 0.3}
                strokeDasharray={online ? undefined : "4 4"}
              />
            );
          })}

        {masterPositions.map((m) => (
          <TopologyNodeShape
            key={m.node.id}
            x={m.x}
            y={m.y}
            label={shortName(m.node.name)}
            role="master"
            status={m.node.status}
            selected={selectedId === m.node.id}
            hovered={hoveredId === m.node.id}
            disabled={ghostMaster}
            onSelect={() => {
              if (!ghostMaster) onSelect(m.node.id === selectedId ? null : m.node.id);
            }}
            onHover={(h) => onHover(h ? m.node.id : null)}
          />
        ))}

        {workerPositions.map((w) => (
          <TopologyNodeShape
            key={w.node.id}
            x={w.x}
            y={w.y}
            label={shortName(w.node.name)}
            role="worker"
            status={w.node.status}
            selected={selectedId === w.node.id}
            hovered={hoveredId === w.node.id}
            onSelect={() => onSelect(w.node.id === selectedId ? null : w.node.id)}
            onHover={(h) => onHover(h ? w.node.id : null)}
          />
        ))}

        {cluster.workers.length === 0 && (
          <text
            x={width / 2}
            y={workerY}
            textAnchor="middle"
            className="fill-muted-foreground text-[12px]"
          >
            No workers connected
          </text>
        )}
      </svg>
    </div>
  );
}

interface TopologyNodeShapeProps {
  x: number;
  y: number;
  label: string;
  role: TopologyRole;
  status: Node["status"];
  selected: boolean;
  hovered: boolean;
  disabled?: boolean;
  onSelect: () => void;
  onHover: (hovered: boolean) => void;
}

function TopologyNodeShape({
  x,
  y,
  label,
  role,
  status,
  selected,
  hovered,
  disabled,
  onSelect,
  onHover,
}: TopologyNodeShapeProps) {
  const isMaster = role === "master";
  const r = isMaster ? 28 : 22;
  const fill = isMaster
    ? "var(--color-sky-500, #0ea5e9)"
    : "var(--color-emerald-500, #10b981)";
  const emphasis = selected || hovered;

  return (
    <g
      transform={`translate(${x}, ${y})`}
      className={cn(!disabled && "cursor-pointer")}
      opacity={disabled ? 0.45 : 1}
      onClick={() => {
        if (!disabled) onSelect();
      }}
      onMouseEnter={() => {
        if (!disabled) onHover(true);
      }}
      onMouseLeave={() => onHover(false)}
      role="button"
      tabIndex={disabled ? -1 : 0}
      aria-pressed={selected}
      aria-label={`${role} ${label}, ${status}`}
      onKeyDown={(e) => {
        if (disabled) return;
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect();
        }
      }}
    >
      {emphasis && (
        <circle
          r={r + 8}
          fill="none"
          stroke={fill}
          strokeWidth={2}
          strokeOpacity={0.35}
        />
      )}
      {isMaster ? (
        <circle
          r={r}
          fill={fill}
          fillOpacity={0.2}
          stroke={fill}
          strokeWidth={selected ? 3 : 2}
        />
      ) : (
        <rect
          x={-r}
          y={-r}
          width={r * 2}
          height={r * 2}
          rx={6}
          fill={fill}
          fillOpacity={0.2}
          stroke={fill}
          strokeWidth={selected ? 3 : 2}
        />
      )}
      <circle
        cx={isMaster ? r - 6 : r - 4}
        cy={isMaster ? -r + 6 : -r + 4}
        r={4}
        fill={statusStroke(status)}
      />
      <text
        y={4}
        textAnchor="middle"
        className="fill-foreground text-[10px] font-semibold"
        style={{ pointerEvents: "none" }}
      >
        {isMaster ? "M" : "W"}
      </text>
      <text
        y={r + 16}
        textAnchor="middle"
        className="fill-foreground text-[11px]"
        style={{ pointerEvents: "none" }}
      >
        {label}
      </text>
    </g>
  );
}
