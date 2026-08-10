import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import type { HandoffRecord } from "../shared/handoff";
import { createRequestHandler } from "./api";
import { parsePort } from "./server";

const temporaryDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

async function fixtureHandler(records: HandoffRecord[] = []) {
  const distDirectory = await mkdtemp(join(tmpdir(), "ai-handoffs-api-"));
  temporaryDirectories.push(distDirectory);
  await mkdir(join(distDirectory, "assets"));
  await writeFile(join(distDirectory, "index.html"), "<main>shell</main>", "utf8");
  await writeFile(join(distDirectory, "assets", "app.js"), "export {};", "utf8");
  return createRequestHandler({ distDirectory, loadHandoffs: async () => records });
}

describe("request handler", () => {
  it("returns the scanner result without caching or accepting a client path", async () => {
    const record = {
      id: "/tmp/VIEW.md",
      state: "live",
      root: "/tmp",
      repository: "tmp",
      filename: "VIEW.md",
      path: "/tmp/VIEW.md",
      format: "legacy",
      title: "View",
      category: null,
      created: null,
      modifiedAt: "2026-08-10T08:00:00.000Z",
      frontmatter: null,
      markdown: "# View\n",
    } satisfies HandoffRecord;
    const handler = await fixtureHandler([record]);

    const response = await handler(new Request("http://localhost/api/handoffs?path=/etc/passwd"));

    expect(response.status).toBe(200);
    expect(response.headers.get("Cache-Control")).toBe("no-store");
    expect(await response.json()).toEqual({ handoffs: [record] });
  });

  it("returns 405 for unsupported endpoint methods and 404 for other API routes", async () => {
    const handler = await fixtureHandler();
    const method = await handler(new Request("http://localhost/api/handoffs", { method: "POST" }));
    const missing = await handler(new Request("http://localhost/api/other"));

    expect(method.status).toBe(405);
    expect(method.headers.get("Allow")).toBe("GET");
    expect(missing.status).toBe(404);
  });

  it("serves assets and GET/HEAD SPA fallbacks", async () => {
    const handler = await fixtureHandler();
    const asset = await handler(new Request("http://localhost/assets/app.js"));
    const fallback = await handler(new Request("http://localhost/handoffs/example"));
    const head = await handler(new Request("http://localhost/handoffs/example", { method: "HEAD" }));

    expect(asset.headers.get("Content-Type")).toBe("text/javascript; charset=utf-8");
    expect(await asset.text()).toBe("export {};");
    expect(await fallback.text()).toBe("<main>shell</main>");
    expect(head.status).toBe(200);
    expect(await head.text()).toBe("");
    expect(head.headers.get("Content-Length")).toBe(String(Buffer.byteLength("<main>shell</main>")));
  });
});

describe("parsePort", () => {
  it("defaults to 7777 and accepts only integer ports in range", () => {
    expect(parsePort(undefined)).toBe(7777);
    expect(parsePort("1")).toBe(1);
    expect(parsePort("65535")).toBe(65_535);
    for (const invalid of ["", "0", "65536", "7.5", " 7777", "abc"]) {
      expect(() => parsePort(invalid)).toThrow("AI_HANDOFFS_PORT must be an integer from 1 to 65535");
    }
  });
});

