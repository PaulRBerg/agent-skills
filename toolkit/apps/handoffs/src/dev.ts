import { parsePort, reportStartupError } from "./server/server";

try {
  const uiPort = parsePort(process.env.AI_HANDOFFS_PORT);
  const apiPort = process.env.AI_HANDOFFS_API_PORT ?? "7778";
  parsePort(apiPort);

  const environment = {
    ...process.env,
    AI_HANDOFFS_PORT: apiPort,
    AI_HANDOFFS_API_PORT: apiPort,
  };
  const api = Bun.spawn([process.execPath, "--watch", "src/server/main.ts"], {
    env: environment,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  const vite = Bun.spawn(
    [process.execPath, "x", "vite", "--host", "127.0.0.1", "--port", String(uiPort), "--strictPort"],
    {
      env: environment,
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    },
  );

  const shutdown = (): void => {
    api.kill();
    vite.kill();
  };
  process.once("SIGINT", shutdown);
  process.once("SIGTERM", shutdown);

  const exitCode = await Promise.race([api.exited, vite.exited]);
  shutdown();
  process.exit(exitCode);
} catch (error) {
  reportStartupError(error);
  process.exit(1);
}

