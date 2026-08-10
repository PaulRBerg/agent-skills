import { mkdir, mkdtemp, rm, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { inspectBuildFreshness, isBuildFresh } from "./freshness";

const temporaryDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

describe("isBuildFresh", () => {
  it("requires both build outputs and at least one input", () => {
    expect(isBuildFresh({ indexMtimeMs: null, stampMtimeMs: 20, latestInputMtimeMs: 10 })).toBe(false);
    expect(isBuildFresh({ indexMtimeMs: 20, stampMtimeMs: null, latestInputMtimeMs: 10 })).toBe(false);
    expect(isBuildFresh({ indexMtimeMs: 20, stampMtimeMs: 20, latestInputMtimeMs: null })).toBe(false);
  });

  it("requires the stamp and index to be at least as new as every input", () => {
    expect(isBuildFresh({ indexMtimeMs: 30, stampMtimeMs: 30, latestInputMtimeMs: 20 })).toBe(true);
    expect(isBuildFresh({ indexMtimeMs: 19, stampMtimeMs: 30, latestInputMtimeMs: 20 })).toBe(false);
    expect(isBuildFresh({ indexMtimeMs: 30, stampMtimeMs: 19, latestInputMtimeMs: 20 })).toBe(false);
  });

  it("includes frontend directory mtimes when inspecting the build", async () => {
    const projectRoot = await mkdtemp(join(tmpdir(), "ai-handoffs-freshness-"));
    temporaryDirectories.push(projectRoot);
    const clientDirectory = join(projectRoot, "src", "client");
    const addedDirectory = join(clientDirectory, "added");
    const distDirectory = join(projectRoot, "dist");
    const old = new Date("2026-08-09T08:00:00Z");
    const built = new Date("2026-08-10T08:00:00Z");
    const added = new Date("2026-08-11T08:00:00Z");

    await mkdir(clientDirectory, { recursive: true });
    await mkdir(distDirectory);
    await writeFile(join(clientDirectory, "main.tsx"), "export {};", "utf8");
    await writeFile(join(distDirectory, "index.html"), "built", "utf8");
    await writeFile(join(distDirectory, ".build-stamp"), "built", "utf8");
    await utimes(join(clientDirectory, "main.tsx"), old, old);
    await utimes(clientDirectory, old, old);
    await utimes(join(distDirectory, "index.html"), built, built);
    await utimes(join(distDirectory, ".build-stamp"), built, built);

    expect(isBuildFresh(await inspectBuildFreshness(projectRoot))).toBe(true);

    await mkdir(addedDirectory);
    await utimes(addedDirectory, added, added);

    expect(isBuildFresh(await inspectBuildFreshness(projectRoot))).toBe(false);
  });
});
