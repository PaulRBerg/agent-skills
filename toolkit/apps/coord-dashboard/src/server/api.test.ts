import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { createRequestHandler } from "./api";
import { parsePort } from "./server";

const temporaryDirectories: string[] = [];

interface FixtureOptions {
  proxyRequest?: (request: Request) => Promise<Response>;
}

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

async function fixtureHandler(options: FixtureOptions = {}) {
  const distDirectory = await mkdtemp(join(tmpdir(), "ai-coord-dashboard-api-"));
  temporaryDirectories.push(distDirectory);
  await mkdir(join(distDirectory, "assets"));
  await writeFile(join(distDirectory, "index.html"), "<main>dashboard</main>", "utf8");
  await writeFile(join(distDirectory, "assets", "app.js"), "export {};", "utf8");
  return createRequestHandler({ distDirectory, ...options });
}

describe("request handler", () => {
  it("proxies API requests without buffering SSE responses", async () => {
    let received: Request | undefined;
    const handler = await fixtureHandler({
      proxyRequest: async (request) => {
        received = request;
        return new Response("event: snapshot\ndata: {}\n\n", {
          headers: { "Content-Type": "text/event-stream" },
        });
      },
    });

    const response = await handler(
      new Request("http://localhost/api/events?generation=9", {
        headers: { Accept: "text/event-stream" },
      }),
    );

    expect(received?.url).toBe("http://127.0.0.1:4477/api/events?generation=9");
    expect(received?.headers.get("Accept")).toBe("text/event-stream");
    expect(response.headers.get("Content-Type")).toBe("text/event-stream");
    expect(await response.text()).toBe("event: snapshot\ndata: {}\n\n");
  });

  it("returns 502 when the coordination API is unavailable", async () => {
    const handler = await fixtureHandler({
      proxyRequest: async () => {
        throw new Error("connection refused");
      },
    });

    const response = await handler(new Request("http://localhost/api/snapshot"));

    expect(response.status).toBe(502);
    expect(await response.text()).toBe("Bad Gateway");
  });

  it("serves assets and GET/HEAD SPA fallbacks", async () => {
    const handler = await fixtureHandler();
    const asset = await handler(new Request("http://localhost/assets/app.js"));
    const fallback = await handler(new Request("http://localhost/repository/example"));
    const head = await handler(new Request("http://localhost/repository/example", { method: "HEAD" }));

    expect(asset.headers.get("Content-Type")).toBe("text/javascript; charset=utf-8");
    expect(await asset.text()).toBe("export {};");
    expect(await fallback.text()).toBe("<main>dashboard</main>");
    expect(head.status).toBe(200);
    expect(await head.text()).toBe("");
    expect(head.headers.get("Content-Length")).toBe(String(Buffer.byteLength("<main>dashboard</main>")));
  });

  it("rejects unsupported static methods and malformed URL escapes", async () => {
    const handler = await fixtureHandler();
    const method = await handler(new Request("http://localhost/dashboard", { method: "POST" }));
    const malformed = await handler(new Request("http://localhost/%E0%A4%A"));

    expect(method.status).toBe(405);
    expect(method.headers.get("Allow")).toBe("GET, HEAD");
    expect(malformed.status).toBe(400);
  });
});

describe("parsePort", () => {
  it("defaults to 4173 and accepts only integer ports in range", () => {
    expect(parsePort(undefined)).toBe(4173);
    expect(parsePort("1")).toBe(1);
    expect(parsePort("65535")).toBe(65_535);
    for (const invalid of ["", "0", "65536", "4.1", " 4173", "abc"]) {
      expect(() => parsePort(invalid)).toThrow(
        "AI_COORD_DASHBOARD_PORT must be an integer from 1 to 65535",
      );
    }
  });
});
