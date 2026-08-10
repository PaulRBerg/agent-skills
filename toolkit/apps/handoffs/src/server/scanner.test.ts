import { mkdir, mkdtemp, rm, symlink, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { scanHandoffs } from "./scanner";

const temporaryDirectories: string[] = [];

async function writeHandoff(path: string, title: string, modifiedAt: Date): Promise<void> {
  await mkdir(join(path, ".."), { recursive: true });
  await writeFile(path, `# ${title}\n`, "utf8");
  await utimes(path, modifiedAt, modifiedAt);
}

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

describe("scanHandoffs", () => {
  it("scans only the bounded live and archive layouts with stable grouping and newest-first order", async () => {
    const home = await mkdtemp(join(tmpdir(), "ai-handoffs-scanner-"));
    temporaryDirectories.push(home);
    const older = new Date("2026-08-09T08:00:00Z");
    const newer = new Date("2026-08-10T08:00:00Z");

    await writeHandoff(join(home, "projects", "alpha", ".ai", "task-handoffs", "OLD.md"), "Old", older);
    await writeHandoff(join(home, "projects", "alpha", ".ai", "task-handoffs", "NEW.md"), "New", newer);
    await writeHandoff(join(home, "work", "beta", ".ai", "task-handoffs", "WORK.md"), "Work", newer);
    await writeHandoff(join(home, "Desktop", ".ai", "task-handoffs", "DESKTOP.md"), "Desktop", newer);
    await writeHandoff(
      join(home, ".local", "share", "task-handoffs", "archive", "alpha", "ARCHIVED.md"),
      "Archived",
      newer,
    );
    await writeHandoff(
      join(home, "projects", "alpha", ".ai", "task-handoffs", "nested", "IGNORED.md"),
      "Ignored",
      newer,
    );
    await writeHandoff(join(home, "outside", "ESCAPE.md"), "Escape", newer);
    await symlink(join(home, "outside"), join(home, "projects", "linked-repository"));
    await symlink(
      join(home, "outside", "ESCAPE.md"),
      join(home, "projects", "alpha", ".ai", "task-handoffs", "LINKED.md"),
    );
    await writeFile(join(home, "projects", "alpha", ".ai", "task-handoffs", "IGNORED.txt"), "ignored", "utf8");

    const records = await scanHandoffs({ homeDir: home });

    expect(records.map((record) => `${record.state}:${record.repository}:${record.filename}`)).toEqual([
      "live:Desktop:DESKTOP.md",
      "live:alpha:NEW.md",
      "live:alpha:OLD.md",
      "live:beta:WORK.md",
      "archived:alpha:ARCHIVED.md",
    ]);
    expect(records.every((record) => record.id === record.path && record.path.startsWith(home))).toBe(true);
    expect(records.find((record) => record.filename === "NEW.md")).toMatchObject({
      root: join(home, "projects"),
      repository: "alpha",
      title: "New",
      format: "legacy",
    });
    expect(records.find((record) => record.filename === "ARCHIVED.md")?.root).toBe(
      join(home, ".local", "share", "task-handoffs", "archive"),
    );
  });

  it("treats entirely missing roots as an empty scan without logging errors", async () => {
    const home = await mkdtemp(join(tmpdir(), "ai-handoffs-scanner-"));
    temporaryDirectories.push(home);
    const errors: unknown[] = [];

    expect(await scanHandoffs({ homeDir: home, logError: (...args) => errors.push(args) })).toEqual([]);
    expect(errors).toEqual([]);
  });
});
