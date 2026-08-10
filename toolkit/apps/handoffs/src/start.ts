import { writeFile } from "node:fs/promises";
import { resolve } from "node:path";

import { inspectBuildFreshness, isBuildFresh } from "./freshness";
import { parsePort, reportStartupError, startServer } from "./server/server";

const projectRoot = resolve(import.meta.dir, "..");
const stampPath = resolve(projectRoot, "dist", ".build-stamp");

async function buildIfNeeded(): Promise<void> {
  if (isBuildFresh(await inspectBuildFreshness(projectRoot))) return;

  console.log("[ai-handoffs] frontend build is stale; rebuilding");
  const build = Bun.spawnSync([process.execPath, "run", "build"], {
    cwd: projectRoot,
    stdout: "inherit",
    stderr: "inherit",
  });
  if (build.exitCode !== 0) {
    throw new Error(`frontend build failed with exit code ${build.exitCode}`);
  }

  await writeFile(stampPath, `${new Date().toISOString()}\n`, "utf8");
}

try {
  const port = parsePort(process.env.AI_HANDOFFS_PORT);
  await buildIfNeeded();
  startServer(port, resolve(projectRoot, "dist"));
} catch (error) {
  reportStartupError(error);
  process.exit(1);
}
