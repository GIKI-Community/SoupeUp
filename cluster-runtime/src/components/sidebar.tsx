import {
  Activity,
  Box,
  Cpu,
  FileText,
  LayoutDashboard,
  Network,
  Puzzle,
  ScrollText,
  Server,
  Settings,
} from "lucide-react";
import { NavLink } from "react-router-dom";

import { cn } from "@/lib/utils";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, end: true },
  { to: "/cluster", label: "Cluster", icon: Network },
  { to: "/compute", label: "Compute", icon: Server },
  { to: "/nodes", label: "Nodes", icon: Server },
  { to: "/jobs", label: "Jobs", icon: Box },
  { to: "/plugins", label: "Plugins", icon: Puzzle },
  { to: "/metrics", label: "Metrics", icon: Activity },
  { to: "/logs", label: "Logs", icon: ScrollText },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

export function Sidebar() {
  return (
    <aside className="flex h-full w-60 flex-col border-r border-sidebar-border bg-sidebar">
      <div className="flex h-14 items-center gap-2 border-b border-sidebar-border px-5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/15">
          <Cpu className="h-4 w-4 text-primary" />
        </div>
        <div>
          <p className="text-sm font-semibold text-sidebar-foreground">
            Cluster Runtime
          </p>
          <p className="text-[11px] text-muted-foreground">v0.1.0</p>
        </div>
      </div>

      <nav className="flex-1 space-y-1 p-3">
        {navItems.map(({ to, label, icon: Icon, end }) => (
          <NavLink
            key={to}
            to={to}
            end={end}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-sidebar-accent text-sidebar-accent-foreground"
                  : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-foreground",
              )
            }
          >
            <Icon className="h-4 w-4" />
            {label}
          </NavLink>
        ))}
      </nav>

      <div className="border-t border-sidebar-border p-4">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <FileText className="h-3.5 w-3.5" />
          <span>Local cluster mode</span>
        </div>
      </div>
    </aside>
  );
}
