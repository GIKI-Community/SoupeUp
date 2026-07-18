import { afterEach, describe, expect, it, vi } from "vitest";

import { HttpTransport } from "../src/http-transport";
import { ClusterError } from "../src/transport";

const BASE = { url: "http://127.0.0.1:8129", token: "secret" };

afterEach(() => {
  vi.restoreAllMocks();
});

describe("HttpTransport.request", () => {
  it("sends the bearer token and parses JSON", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(JSON.stringify({ status: "ok" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const t = new HttpTransport(BASE);
    const out = await t.request<{ status: string }>("GET", "/health");

    expect(out).toEqual({ status: "ok" });
    const [, init] = fetchMock.mock.calls[0];
    expect((init as RequestInit).headers).toMatchObject({
      Authorization: "Bearer secret",
    });
  });

  it("maps error responses to ClusterError with the server message", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        new Response(JSON.stringify({ error: "boom" }), { status: 400 }),
      ),
    );

    const t = new HttpTransport(BASE);
    await expect(t.request("GET", "/v1/jobs")).rejects.toMatchObject({
      message: "boom",
      status: 400,
    });
  });

  it("wraps network failures in a friendly ClusterError", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new TypeError("fetch failed");
      }),
    );

    const t = new HttpTransport(BASE);
    await expect(t.request("GET", "/health")).rejects.toBeInstanceOf(ClusterError);
  });

  it("serializes a JSON body with content-type", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(JSON.stringify({ jobId: "1", status: "queued" }), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const t = new HttpTransport(BASE);
    await t.request("POST", "/v1/jobs", { name: "x" });

    const [, init] = fetchMock.mock.calls[0];
    expect((init as RequestInit).body).toBe(JSON.stringify({ name: "x" }));
    expect((init as RequestInit).headers).toMatchObject({
      "Content-Type": "application/json",
    });
  });
});
