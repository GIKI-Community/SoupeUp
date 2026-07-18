import { describe, expect, it } from "vitest";

import { parseClusterConfig, starterClusterConfig } from "../src/config";

describe("parseClusterConfig", () => {
  it("parses snake_case and camelCase keys", () => {
    const cfg = parseClusterConfig(
      [
        "scheduler: ray",
        "entry: main.py",
        "working_directory: ./src",
        "arguments: [--fast, 3]",
        "upload_project: true",
        "watch_changes: false",
      ].join("\n"),
    );
    expect(cfg).toEqual({
      scheduler: "ray",
      entry: "main.py",
      workingDirectory: "./src",
      arguments: ["--fast", "3"],
      uploadProject: true,
      watchChanges: false,
    });
  });

  it("returns an empty object for empty input", () => {
    expect(parseClusterConfig("")).toEqual({});
  });

  it("ignores unknown keys", () => {
    const cfg = parseClusterConfig("scheduler: dask\nunknown: value");
    expect(cfg).toEqual({ scheduler: "dask" });
  });

  it("throws on malformed yaml", () => {
    expect(() => parseClusterConfig("scheduler: [unclosed")).toThrow(
      /Invalid .cluster configuration/,
    );
  });

  it("throws when the document is not a mapping", () => {
    expect(() => parseClusterConfig("- just\n- a\n- list")).toThrow(
      /expected a YAML mapping/,
    );
  });

  it("produces a parseable starter config", () => {
    const cfg = parseClusterConfig(starterClusterConfig("ray", "app.py"));
    expect(cfg.scheduler).toBe("ray");
    expect(cfg.entry).toBe("app.py");
  });
});
