import { readFile, stat } from "node:fs/promises";
import { extname, resolve, sep } from "node:path";

import type { HandoffsResponse } from "../shared/handoff";
import { scanHandoffs } from "./scanner";

export interface RequestHandlerOptions {
  distDirectory: string;
  loadHandoffs?: typeof scanHandoffs;
}

const CONTENT_TYPES: Record<string, string> = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
};

function responseBody(method: string, bytes: Uint8Array): BodyInit | null {
  return method === "HEAD" ? null : (bytes as BodyInit);
}

async function fileResponse(path: string, method: string): Promise<Response | null> {
  try {
    const metadata = await stat(path);
    if (!metadata.isFile()) return null;
    const bytes = new Uint8Array(await readFile(path));
    return new Response(responseBody(method, bytes), {
      headers: {
        "Content-Length": String(bytes.byteLength),
        "Content-Type": CONTENT_TYPES[extname(path)] ?? "application/octet-stream",
      },
    });
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") return null;
    console.error(`[ai-handoffs] unable to serve ${path}`, error);
    return new Response("Internal Server Error", { status: 500 });
  }
}

export function createRequestHandler(options: RequestHandlerOptions): (request: Request) => Promise<Response> {
  const loadHandoffs = options.loadHandoffs ?? scanHandoffs;
  const distDirectory = resolve(options.distDirectory);
  const indexPath = resolve(distDirectory, "index.html");

  return async (request: Request): Promise<Response> => {
    const url = new URL(request.url);

    if (url.pathname === "/api/handoffs") {
      if (request.method !== "GET") {
        return new Response("Method Not Allowed", {
          status: 405,
          headers: { Allow: "GET" },
        });
      }

      const payload: HandoffsResponse = { handoffs: await loadHandoffs() };
      return Response.json(payload, {
        headers: { "Cache-Control": "no-store" },
      });
    }

    if (url.pathname === "/api" || url.pathname.startsWith("/api/")) {
      return new Response("Not Found", { status: 404 });
    }

    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("Method Not Allowed", {
        status: 405,
        headers: { Allow: "GET, HEAD" },
      });
    }

    let decodedPath: string;
    try {
      decodedPath = decodeURIComponent(url.pathname);
    } catch {
      return new Response("Bad Request", { status: 400 });
    }

    const assetPath = resolve(distDirectory, `.${decodedPath}`);
    if (assetPath === distDirectory || assetPath.startsWith(`${distDirectory}${sep}`)) {
      const asset = await fileResponse(assetPath, request.method);
      if (asset) return asset;
    }

    return (await fileResponse(indexPath, request.method)) ?? new Response("Not Found", { status: 404 });
  };
}
