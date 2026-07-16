import { Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { LogLevelText } from "@/components/status-badges";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
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
import { LogsApi } from "@/api";
import type { LogEntry, LogLevel } from "@/types";

export function LogsPage() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [search, setSearch] = useState("");
  const [levelFilter, setLevelFilter] = useState<LogLevel | "all">("all");
  const [moduleFilter, setModuleFilter] = useState<string>("all");

  useEffect(() => {
    void LogsApi.list().then(setLogs);
  }, []);

  const modules = useMemo(
    () => [...new Set(logs.map((log) => log.module))].sort(),
    [logs],
  );

  const filteredLogs = useMemo(() => {
    return logs.filter((log) => {
      const matchesSearch =
        search === "" ||
        log.message.toLowerCase().includes(search.toLowerCase()) ||
        log.module.toLowerCase().includes(search.toLowerCase());
      const matchesLevel = levelFilter === "all" || log.level === levelFilter;
      const matchesModule = moduleFilter === "all" || log.module === moduleFilter;
      return matchesSearch && matchesLevel && matchesModule;
    });
  }, [logs, search, levelFilter, moduleFilter]);

  return (
    <div>
      <PageHeader
        title="Logs"
        description="Application and cluster event logs"
      />

      <div className="mb-6 flex flex-col gap-4 sm:flex-row">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search logs..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select
          value={levelFilter}
          onValueChange={(value) => setLevelFilter(value as LogLevel | "all")}
        >
          <SelectTrigger className="w-full sm:w-36">
            <SelectValue placeholder="Level" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All levels</SelectItem>
            <SelectItem value="trace">Trace</SelectItem>
            <SelectItem value="debug">Debug</SelectItem>
            <SelectItem value="info">Info</SelectItem>
            <SelectItem value="warn">Warn</SelectItem>
            <SelectItem value="error">Error</SelectItem>
          </SelectContent>
        </Select>
        <Select value={moduleFilter} onValueChange={setModuleFilter}>
          <SelectTrigger className="w-full sm:w-36">
            <SelectValue placeholder="Module" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All modules</SelectItem>
            {modules.map((mod) => (
              <SelectItem key={mod} value={mod}>
                {mod}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-xl border border-border/60 bg-card/80">
        <ScrollArea className="h-[calc(100vh-280px)]">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-44">Timestamp</TableHead>
                <TableHead className="w-28">Module</TableHead>
                <TableHead className="w-20">Level</TableHead>
                <TableHead>Message</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredLogs.map((log) => (
                <TableRow key={log.id}>
                  <TableCell className="font-mono text-xs text-muted-foreground">
                    {new Date(log.timestamp).toLocaleString()}
                  </TableCell>
                  <TableCell className="font-mono text-xs">{log.module}</TableCell>
                  <TableCell>
                    <LogLevelText level={log.level} />
                  </TableCell>
                  <TableCell className="text-sm">{log.message}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </ScrollArea>
      </div>
    </div>
  );
}
