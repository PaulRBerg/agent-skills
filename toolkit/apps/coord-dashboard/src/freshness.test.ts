import { mkdir, mkdtemp, rm, unlink, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import nodePath from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { BUILD_INPUTS, inspectBuildFreshness, isBuildFresh } from "./freshness.js";

const temporaryDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(
    temporaryDirectories.splice(0).map((path) => rm(path, { recursive: true, force: true }))
  );
});

describe("isBuildFresh", () => {
  it("requires both build outputs and at least one input", () => {
    expect(isBuildFresh({ indexMtimeMs: null, stampMtimeMs: 20, latestInputMtimeMs: 10 })).toBe(
      false
    );
    expect(isBuildFresh({ indexMtimeMs: 20, stampMtimeMs: null, latestInputMtimeMs: 10 })).toBe(
      false
    );
    expect(isBuildFresh({ indexMtimeMs: 20, stampMtimeMs: 20, latestInputMtimeMs: null })).toBe(
      false
    );
  });

  it("requires the stamp and index to be at least as new as every input", () => {
    expect(isBuildFresh({ indexMtimeMs: 30, stampMtimeMs: 30, latestInputMtimeMs: 20 })).toBe(true);
    expect(isBuildFresh({ indexMtimeMs: 19, stampMtimeMs: 30, latestInputMtimeMs: 20 })).toBe(
      false
    );
    expect(isBuildFresh({ indexMtimeMs: 30, stampMtimeMs: 19, latestInputMtimeMs: 20 })).toBe(
      false
    );
  });

  it("includes newly added source directories when inspecting the build", async () => {
    const projectRoot = await mkdtemp(nodePath.join(tmpdir(), "ai-coord-dashboard-freshness-"));
    temporaryDirectories.push(projectRoot);
    const sourceDirectory = nodePath.join(projectRoot, "src");
    const addedDirectory = nodePath.join(sourceDirectory, "added");
    const distDirectory = nodePath.join(projectRoot, "dist");
    const old = new Date("2026-08-09T08:00:00Z");
    const built = new Date("2026-08-10T08:00:00Z");
    const added = new Date("2026-08-11T08:00:00Z");

    await mkdir(sourceDirectory, { recursive: true });
    await mkdir(distDirectory);
    await writeFile(nodePath.join(sourceDirectory, "main.tsx"), "export {};", "utf-8");
    await Promise.all(
      BUILD_INPUTS.filter((input) => input !== "src").map(async (input) => {
        const inputPath = nodePath.join(projectRoot, input);
        await writeFile(inputPath, input, "utf-8");
        await utimes(inputPath, old, old);
      })
    );
    await writeFile(nodePath.join(distDirectory, "index.html"), "built", "utf-8");
    await writeFile(nodePath.join(distDirectory, ".build-stamp"), "built", "utf-8");
    await utimes(nodePath.join(sourceDirectory, "main.tsx"), old, old);
    await utimes(sourceDirectory, old, old);
    await utimes(nodePath.join(distDirectory, "index.html"), built, built);
    await utimes(nodePath.join(distDirectory, ".build-stamp"), built, built);

    expect(isBuildFresh(await inspectBuildFreshness(projectRoot))).toBe(true);

    await mkdir(addedDirectory);
    await utimes(addedDirectory, added, added);

    expect(isBuildFresh(await inspectBuildFreshness(projectRoot))).toBe(false);
  });

  it("treats a missing declared input as stale", async () => {
    const projectRoot = await mkdtemp(nodePath.join(tmpdir(), "ai-coord-dashboard-freshness-"));
    temporaryDirectories.push(projectRoot);
    const old = new Date("2026-08-09T08:00:00Z");
    const built = new Date("2026-08-10T08:00:00Z");
    const distDirectory = nodePath.join(projectRoot, "dist");

    await mkdir(nodePath.join(projectRoot, "src"), { recursive: true });
    await Promise.all(
      BUILD_INPUTS.filter((input) => input !== "src").map(async (input) => {
        const inputPath = nodePath.join(projectRoot, input);
        await writeFile(inputPath, input, "utf-8");
        await utimes(inputPath, old, old);
      })
    );
    await utimes(nodePath.join(projectRoot, "src"), old, old);
    await mkdir(distDirectory);
    await writeFile(nodePath.join(distDirectory, "index.html"), "built", "utf-8");
    await writeFile(nodePath.join(distDirectory, ".build-stamp"), "built", "utf-8");
    await utimes(nodePath.join(distDirectory, "index.html"), built, built);
    await utimes(nodePath.join(distDirectory, ".build-stamp"), built, built);
    expect(isBuildFresh(await inspectBuildFreshness(projectRoot))).toBe(true);

    await unlink(nodePath.join(projectRoot, "index.html"));

    const freshness = await inspectBuildFreshness(projectRoot);
    expect(freshness.latestInputMtimeMs).toBeNull();
    expect(isBuildFresh(freshness)).toBe(false);
  });
});
