import { readdir, stat } from "node:fs/promises";
import nodePath from "node:path";

export type BuildFreshness = {
  indexMtimeMs: number | null;
  stampMtimeMs: number | null;
  latestInputMtimeMs: number | null;
};

export const BUILD_INPUTS = [
  "src",
  "public",
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

async function statIfPresent(path: string) {
  try {
    return await stat(path);
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") {
      return null;
    }
    throw error;
  }
}

async function mtime(path: string): Promise<number | null> {
  const metadata = await statIfPresent(path);
  return metadata?.mtimeMs ?? null;
}

async function latestTreeMtime(path: string): Promise<number | null> {
  const metadata = await statIfPresent(path);
  if (metadata === null) {
    return null;
  }

  let latest = metadata.mtimeMs;
  if (!metadata.isDirectory()) {
    return latest;
  }

  const entries = await readdir(path);
  for (const entry of entries) {
    // oxlint-disable-next-line no-await-in-loop -- Bound filesystem traversal to one child at a time.
    const childMtime = await latestTreeMtime(nodePath.resolve(path, entry));
    if (childMtime !== null) {
      latest = Math.max(latest, childMtime);
    }
  }
  return latest;
}

export async function inspectBuildFreshness(projectRoot: string): Promise<BuildFreshness> {
  const inputMtimes = await Promise.all(
    BUILD_INPUTS.map((path) => latestTreeMtime(nodePath.resolve(projectRoot, path)))
  );
  const allInputsPresent = inputMtimes.every((value): value is number => value !== null);

  return {
    indexMtimeMs: await mtime(nodePath.resolve(projectRoot, "dist", "index.html")),
    stampMtimeMs: await mtime(nodePath.resolve(projectRoot, "dist", ".build-stamp")),
    latestInputMtimeMs: allInputsPresent ? Math.max(...inputMtimes) : null,
  };
}
