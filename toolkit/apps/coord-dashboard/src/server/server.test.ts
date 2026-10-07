import { execFile } from "node:child_process";
import { promisify } from "node:util";

import { expect, it } from "vitest";

const execFileAsync = promisify(execFile);

it("keeps SSE responses open across quiet intervals", { timeout: 25_000 }, async () => {
  const serverModule = new URL("server.ts", import.meta.url).href;
  const { stdout } = await execFileAsync(
    "bun",
    [
      "--no-install",
      "--eval",
      `import { startServer } from ${JSON.stringify(serverModule)};
      const encoder = new TextEncoder();
      let heartbeat;
      const server = startServer(0, async () => new Response(new ReadableStream({
        start(controller) {
          controller.enqueue(encoder.encode(": connected\\n\\n"));
          heartbeat = setTimeout(() => {
            controller.enqueue(encoder.encode(": heartbeat\\n\\n"));
            controller.close();
          }, 15_000);
        },
        cancel() { clearTimeout(heartbeat); }
      }), { headers: { "Content-Type": "text/event-stream; charset=utf-8" } }));
      try {
        const response = await fetch(server.url);
        console.log(await response.text());
      } finally {
        clearTimeout(heartbeat);
        await server.stop(true);
      }`,
    ],
    { timeout: 23_000 }
  );

  expect(stdout).toContain(": connected\n\n: heartbeat\n\n");
});
