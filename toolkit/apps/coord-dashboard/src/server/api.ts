import { readFile, stat } from "node:fs/promises";
import { extname, resolve, sep } from "node:path";

const API_ORIGIN = "http://127.0.0.1:4477";

const CONTENT_TYPES: Record<string, string> = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".ico": "image/x-icon",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".jpg": "image/jpeg",
  ".map": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".webp": "image/webp",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
};

export interface RequestHandlerOptions {
  distDirectory: string;
  apiOrigin?: string;
  proxyRequest?: (request: Request) => Promise<Response>;
}

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
    console.error(`[ai-coord-dashboard] unable to serve ${path}`, error);
    return new Response("Internal Server Error", { status: 500 });
  }
}

export function createRequestHandler(options: RequestHandlerOptions): (request: Request) => Promise<Response> {
  const distDirectory = resolve(options.distDirectory);
  const indexPath = resolve(distDirectory, "index.html");
  const apiOrigin = options.apiOrigin ?? API_ORIGIN;
  const proxyRequest = options.proxyRequest ?? ((request: Request) => fetch(request));

  return async (request: Request): Promise<Response> => {
    const url = new URL(request.url);

    if (url.pathname === "/api" || url.pathname.startsWith("/api/")) {
      const upstream = new URL(`${url.pathname}${url.search}`, apiOrigin);
      try {
        return await proxyRequest(new Request(upstream, request));
      } catch (error) {
        console.error(`[ai-coord-dashboard] unable to reach ${upstream.origin}`, error);
        return new Response("Bad Gateway", { status: 502 });
      }
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
