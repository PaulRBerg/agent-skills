import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { createRequestHandler } from "./api";
import { parsePort } from "./server";

const temporaryDirectories: string[] = [];

interface FixtureOptions {
  proxyRequest?: (request: Request) => Promise<Response>;
  homeDirectory?: string;
  indexHtml?: string;
}

const FIXTURE_HOME = "/Users/fixture-home";
const FIXTURE_INDEX_HTML = "<html><head><title>t</title></head><body><main>dashboard</main></body></html>";

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

async function fixtureHandler({ indexHtml = FIXTURE_INDEX_HTML, homeDirectory = FIXTURE_HOME, ...options }: FixtureOptions = {}) {
  const distDirectory = await mkdtemp(join(tmpdir(), "ai-coord-dashboard-api-"));
  temporaryDirectories.push(distDirectory);
  await mkdir(join(distDirectory, "assets"));
  await writeFile(join(distDirectory, "index.html"), indexHtml, "utf8");
  await writeFile(join(distDirectory, "assets", "app.js"), "export {};", "utf8");
  return createRequestHandler({ distDirectory, homeDirectory, ...options });
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

  it("rejects forged Host headers on API and static routes (DNS-rebinding guard)", async () => {
    const handler = await fixtureHandler();
    const api = await handler(new Request("http://evil.example:1234/api/snapshot"));
    const asset = await handler(new Request("http://evil.example:1234/assets/app.js"));

    expect(api.status).toBe(403);
    expect(asset.status).toBe(403);
  });

  it("accepts loopback Host headers with any port", async () => {
    const handler = await fixtureHandler({
      proxyRequest: async () => new Response("{}", { headers: { "Content-Type": "application/json" } }),
    });
    for (const host of ["localhost:9999", "127.0.0.1:9999", "[::1]:9999", "LOCALHOST:9999"]) {
      const response = await handler(new Request(`http://${host}/api/snapshot`));
      expect(response.status).toBe(200);
    }
  });

  it("returns 400 for an unparseable request URL", async () => {
    const handler = await fixtureHandler();
    const request = { url: "/api/snapshot", method: "GET" } as unknown as Request;

    const response = await handler(request);

    expect(response.status).toBe(400);
  });

  it("serves assets and GET/HEAD SPA fallbacks", async () => {
    const handler = await fixtureHandler();
    const asset = await handler(new Request("http://localhost/assets/app.js"));
    const fallback = await handler(new Request("http://localhost/repository/example"));
    const head = await handler(new Request("http://localhost/repository/example", { method: "HEAD" }));
    const expectedHtml = FIXTURE_INDEX_HTML.replace(
      "</head>",
      `<meta name="dashboard-home" content="${FIXTURE_HOME}"></head>`,
    );

    expect(asset.headers.get("Content-Type")).toBe("text/javascript; charset=utf-8");
    expect(await asset.text()).toBe("export {};");
    expect(await fallback.text()).toBe(expectedHtml);
    expect(head.status).toBe(200);
    expect(await head.text()).toBe("");
    expect(head.headers.get("Content-Length")).toBe(String(Buffer.byteLength(expectedHtml)));
  });

  it("injects the server's home directory into served HTML for the client to read", async () => {
    const handler = await fixtureHandler({ homeDirectory: '/Users/o\'<hara> & "co"' });
    const index = await handler(new Request("http://localhost/index.html"));
    const fallback = await handler(new Request("http://localhost/repository/example"));

    const expectedMeta =
      '<meta name="dashboard-home" content="/Users/o\'&lt;hara&gt; &amp; &quot;co&quot;">';
    expect(await index.text()).toContain(expectedMeta);
    expect(await fallback.text()).toContain(expectedMeta);
  });

  it("prepends the home directory meta tag when the document has no <head>", async () => {
    const handler = await fixtureHandler({ indexHtml: "<main>dashboard</main>" });
    const response = await handler(new Request("http://localhost/missing"));

    expect(await response.text()).toBe(
      `<meta name="dashboard-home" content="${FIXTURE_HOME}"><main>dashboard</main>`,
    );
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
