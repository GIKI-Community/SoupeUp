import type {
  DepsProbeResult,
  ModuleProbeEntry,
  ModuleProbeStatus,
} from "@cluster-runtime/client";
import * as vscode from "vscode";

import type { ConnectionService } from "../services/connection";

/** Local import parse (mirrors cluster-runtime-core parse_imports). */
export function parseImports(source: string): string[] {
  const found = new Set<string>();
  for (const raw of source.split(/\r?\n/)) {
    const line = stripComment(raw).trim();
    if (!line) continue;

    if (line.startsWith("from ")) {
      const rest = line.slice(5).trim();
      if (rest.startsWith(".")) continue;
      const module = rest.split(/\s+/)[0]?.replace(/[^\w.]/g, "") ?? "";
      const top = module.split(".")[0];
      if (top && top !== "__future__") found.add(top);
      continue;
    }

    if (line.startsWith("import ")) {
      for (const part of line.slice(7).split(",")) {
        const name = part.trim().split(/\s+/)[0]?.replace(/[^\w.]/g, "") ?? "";
        const top = name.split(".")[0];
        if (top && top !== "__future__") found.add(top);
      }
    }
  }
  return [...found].sort();
}

function stripComment(line: string): string {
  const idx = line.indexOf("#");
  if (idx < 0) return line;
  const before = line.slice(0, idx);
  const single = (before.match(/'/g) ?? []).length;
  const double = (before.match(/"/g) ?? []).length;
  if (single % 2 === 0 && double % 2 === 0) return before;
  return line;
}

const PIP_MAP: Record<string, string> = {
  cv2: "opencv-python",
  PIL: "Pillow",
  sklearn: "scikit-learn",
  bs4: "beautifulsoup4",
  yaml: "PyYAML",
  Crypto: "pycryptodome",
  skimage: "scikit-image",
  dateutil: "python-dateutil",
  dotenv: "python-dotenv",
  OpenSSL: "pyOpenSSL",
};

export function pipNameFor(importName: string): string {
  return PIP_MAP[importName] ?? importName;
}

export class ImportTreeItem extends vscode.TreeItem {
  constructor(
    public readonly entry: ModuleProbeEntry,
    public readonly fileLabel: string,
  ) {
    super(entry.importName, vscode.TreeItemCollapsibleState.None);
    this.description = `${entry.pipName} · ${entry.status}`;
    this.tooltip = `${entry.importName} → pip:${entry.pipName} (${entry.status}) [${fileLabel}]`;
    this.iconPath = new vscode.ThemeIcon(statusIcon(entry.status));
    this.contextValue = `import-${entry.status}`;
  }
}

function statusIcon(status: ModuleProbeStatus): string {
  switch (status) {
    case "present":
      return "check";
    case "missing":
      return "warning";
    case "stdlib":
      return "library";
    default:
      return "circle-outline";
  }
}

/**
 * Shows imports for the active Python editor, enriched after a cluster probe.
 */
export class ImportsTreeProvider
  implements vscode.TreeDataProvider<vscode.TreeItem>, vscode.Disposable
{
  private readonly _onDidChangeTreeData = new vscode.EventEmitter<
    vscode.TreeItem | undefined | null | void
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private lastProbe: DepsProbeResult | undefined;
  private disposables: vscode.Disposable[] = [];

  constructor(private readonly connection: ConnectionService) {
    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor(() => this.refresh()),
      vscode.workspace.onDidChangeTextDocument((e) => {
        const ed = vscode.window.activeTextEditor;
        if (ed && e.document === ed.document && ed.document.languageId === "python") {
          this.refresh();
        }
      }),
    );
  }

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  setProbeResult(result: DepsProbeResult | undefined): void {
    this.lastProbe = result;
    this.refresh();
  }

  getProbeResult(): DepsProbeResult | undefined {
    return this.lastProbe;
  }

  getActivePythonSource(): { fileName: string; source: string } | undefined {
    const ed = vscode.window.activeTextEditor;
    if (!ed || ed.document.languageId !== "python") return undefined;
    return {
      fileName: ed.document.fileName.split(/[\\/]/).pop() ?? "script.py",
      source: ed.document.getText(),
    };
  }

  getTreeItem(element: vscode.TreeItem): vscode.TreeItem {
    return element;
  }

  getChildren(): vscode.TreeItem[] {
    const active = this.getActivePythonSource();
    if (!active) {
      const item = new vscode.TreeItem("Open a Python file");
      item.iconPath = new vscode.ThemeIcon("info");
      return [item];
    }

    const imports = parseImports(active.source);
    if (imports.length === 0) {
      return [new vscode.TreeItem("No imports detected")];
    }

    const statusByImport = new Map<string, ModuleProbeStatus>();
    if (this.lastProbe) {
      for (const m of this.lastProbe.modules) {
        statusByImport.set(m.importName, m.status);
      }
      // Prefer cluster-wide missing when workers were checked.
      for (const name of this.lastProbe.missingAnywhere ?? []) {
        statusByImport.set(name, "missing");
      }
      for (const name of this.lastProbe.head?.skippedStdlib ?? []) {
        if (!statusByImport.has(name)) statusByImport.set(name, "stdlib");
      }
    }

    return imports.map((name) => {
      const entry: ModuleProbeEntry = {
        importName: name,
        pipName: pipNameFor(name),
        status: statusByImport.get(name) ?? "unknown",
      };
      return new ImportTreeItem(entry, active.fileName);
    });
  }

  dispose(): void {
    for (const d of this.disposables) d.dispose();
    this._onDidChangeTreeData.dispose();
  }
}
