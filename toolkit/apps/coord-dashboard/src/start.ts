import { writeFile } from "node:fs/promises";
import nodePath from "node:path";

import { inspectBuildFreshness, isBuildFresh } from "./freshness.js";
import { createRequestHandler } from "./server/api.js";
import { parsePort, reportStartupError, startServer } from "./server/server.js";

const projectRoot = nodePath.resolve(import.meta.dir, "..");
const stampPath = nodePath.resolve(projectRoot, "dist", ".build-stamp");

async function buildIfNeeded(): Promise<void> {
  if (isBuildFresh(await inspectBuildFreshness(projectRoot))) {
    return;
  }

  console.log("[ai-coord-dashboard] frontend build is stale; rebuilding");
  const build = Bun.spawnSync([process.execPath, "run", "build"], {
    cwd: projectRoot,
    stdout: "inherit",
    stderr: "inherit",
  });
  if (build.exitCode !== 0) {
    throw new Error(`frontend build failed with exit code ${build.exitCode}`);
  }

  await writeFile(stampPath, `${new Date().toISOString()}\n`, "utf-8");
}

try {
  const port = parsePort(process.env.AI_COORD_DASHBOARD_PORT);
  await buildIfNeeded();
  startServer(port, createRequestHandler({ distDirectory: nodePath.resolve(projectRoot, "dist") }));
} catch (error) {
  reportStartupError(error);
  process.exit(1);
}
