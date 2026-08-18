import { readdir, stat } from "node:fs/promises";
import { resolve } from "node:path";

export interface BuildFreshness {
  indexMtimeMs: number | null;
  stampMtimeMs: number | null;
  latestInputMtimeMs: number | null;
}

export const BUILD_INPUTS = [
  "src",
  "index.html",
  "vite.config.ts",
  "tsconfig.json",
  "package.json",
  "bun.lock",
] as const;

export function isBuildFresh(freshness: BuildFreshness): boolean {
  if (
    freshness.indexMtimeMs === null ||
    freshness.stampMtimeMs === null ||
    freshness.latestInputMtimeMs === null
  ) {
    return false;
  }

  return (
    freshness.stampMtimeMs >= freshness.latestInputMtimeMs &&
    freshness.indexMtimeMs >= freshness.latestInputMtimeMs
  );
}

async function mtime(path: string): Promise<number | null> {
  try {
    return (await stat(path)).mtimeMs;
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") return null;
    throw error;
  }
}

async function latestTreeMtime(path: string): Promise<number | null> {
  let metadata;
  try {
    metadata = await stat(path);
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") return null;
    throw error;
  }

  let latest = metadata.mtimeMs;
  if (!metadata.isDirectory()) return latest;

  const entries = await readdir(path);
  for (const entry of entries) {
    const childMtime = await latestTreeMtime(resolve(path, entry));
    if (childMtime !== null) latest = Math.max(latest, childMtime);
  }
  return latest;
}

export async function inspectBuildFreshness(projectRoot: string): Promise<BuildFreshness> {
  const inputMtimes = await Promise.all(BUILD_INPUTS.map((path) => latestTreeMtime(resolve(projectRoot, path))));
  const presentInputMtimes = inputMtimes.filter((value): value is number => value !== null);

  return {
    indexMtimeMs: await mtime(resolve(projectRoot, "dist", "index.html")),
    stampMtimeMs: await mtime(resolve(projectRoot, "dist", ".build-stamp")),
    latestInputMtimeMs: presentInputMtimes.length > 0 ? Math.max(...presentInputMtimes) : null,
  };
}
