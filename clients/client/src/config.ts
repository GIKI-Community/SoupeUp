import yaml from "js-yaml";

/** Parsed `.cluster` project configuration file. */
export interface ClusterConfig {
  scheduler?: string;
  entry?: string;
  workingDirectory?: string;
  arguments?: string[];
  environment?: string;
  uploadProject?: boolean;
  watchChanges?: boolean;
}

/**
 * Parse a `.cluster` YAML config. Unknown keys are ignored; missing keys
 * stay undefined. Throws on malformed YAML with a clear message.
 */
export function parseClusterConfig(text: string): ClusterConfig {
  let doc: unknown;
  try {
    doc = yaml.load(text);
  } catch (err) {
    throw new Error(
      `Invalid .cluster configuration: ${err instanceof Error ? err.message : String(err)}`,
    );
  }

  if (doc == null) return {};
  if (typeof doc !== "object" || Array.isArray(doc)) {
    throw new Error("Invalid .cluster configuration: expected a YAML mapping");
  }

  const raw = doc as Record<string, unknown>;
  const config: ClusterConfig = {};

  if (typeof raw.scheduler === "string") config.scheduler = raw.scheduler;
  if (typeof raw.entry === "string") config.entry = raw.entry;
  if (typeof raw.working_directory === "string")
    config.workingDirectory = raw.working_directory;
  else if (typeof raw.workingDirectory === "string")
    config.workingDirectory = raw.workingDirectory;
  if (Array.isArray(raw.arguments))
    config.arguments = raw.arguments.map((a) => String(a));
  if (typeof raw.environment === "string") config.environment = raw.environment;
  if (typeof raw.upload_project === "boolean")
    config.uploadProject = raw.upload_project;
  else if (typeof raw.uploadProject === "boolean")
    config.uploadProject = raw.uploadProject;
  if (typeof raw.watch_changes === "boolean")
    config.watchChanges = raw.watch_changes;
  else if (typeof raw.watchChanges === "boolean")
    config.watchChanges = raw.watchChanges;

  return config;
}

/** Serialize a starter `.cluster` config for `Initialize Project`. */
export function starterClusterConfig(scheduler = "dask", entry = "main.py"): string {
  return [
    `scheduler: ${scheduler}`,
    `entry: ${entry}`,
    "working_directory: .",
    "arguments: []",
    "environment: default",
    "upload_project: false",
    "watch_changes: false",
    "",
  ].join("\n");
}
