import { resolve } from "node:path";

import { createRequestHandler } from "./api";

const DEFAULT_PORT = 7777;

export function parsePort(rawPort: string | undefined): number {
  if (rawPort === undefined) return DEFAULT_PORT;
  if (!/^\d+$/.test(rawPort)) {
    throw new Error(`AI_HANDOFFS_PORT must be an integer from 1 to 65535; received ${JSON.stringify(rawPort)}`);
  }

  const port = Number(rawPort);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new Error(`AI_HANDOFFS_PORT must be an integer from 1 to 65535; received ${JSON.stringify(rawPort)}`);
  }
  return port;
}

export function startServer(port: number, distDirectory = resolve("dist")): Bun.Server<undefined> {
  const handler = createRequestHandler({ distDirectory });
  const server = Bun.serve({
    hostname: "127.0.0.1",
    port,
    fetch: handler,
  });
  console.log(`[ai-handoffs] listening on http://127.0.0.1:${port}`);
  return server;
}

export function reportStartupError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[ai-handoffs] failed to start: ${message}`);
}
