import { promises as fs } from "node:fs";
import path from "node:path";
import { afterAll, describe, expect, it } from "vitest";

import {
  appDataDir,
  discoverEndpoint,
  endpointFilePath,
  APP_IDENTIFIER,
} from "../src/discovery";

const TEST_ID = `dev.cluster-runtime.test-${process.pid}`;

afterAll(async () => {
  await fs.rm(appDataDir(TEST_ID), { recursive: true, force: true });
});

describe("discovery", () => {
  it("uses the default bundle identifier in the data dir", () => {
    expect(appDataDir()).toContain(APP_IDENTIFIER);
  });

  it("builds the endpoint file path under api/endpoint.json", () => {
    const p = endpointFilePath(TEST_ID);
    expect(p.endsWith(path.join("api", "endpoint.json"))).toBe(true);
    expect(p).toContain(TEST_ID);
  });

  it("returns null when no endpoint file exists", async () => {
    const result = await discoverEndpoint(`missing-${process.pid}-xyz`);
    expect(result).toBeNull();
  });

  it("reads a written endpoint file", async () => {
    const file = endpointFilePath(TEST_ID);
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.writeFile(
      file,
      JSON.stringify({ url: "http://127.0.0.1:8129", token: "abc", pid: 42 }),
    );

    const result = await discoverEndpoint(TEST_ID);
    expect(result).toEqual({ url: "http://127.0.0.1:8129", token: "abc", pid: 42 });
  });

  it("returns null for a malformed endpoint file", async () => {
    const file = endpointFilePath(TEST_ID);
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.writeFile(file, "{ not json");
    expect(await discoverEndpoint(TEST_ID)).toBeNull();
  });
});
