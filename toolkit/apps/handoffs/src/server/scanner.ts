import { readdir, readFile, stat } from "node:fs/promises";
import { homedir } from "node:os";
import { basename, join } from "node:path";

import type { HandoffRecord } from "../shared/handoff";
import { parseHandoff } from "./parser";

export interface ScanOptions {
  homeDir?: string;
  logError?: (message: string, error: unknown) => void;
}

interface ScanTarget {
  state: HandoffRecord["state"];
  root: string;
  repository: string;
  handoffDirectory: string;
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

export function compareHandoffs(left: HandoffRecord, right: HandoffRecord): number {
  const state = Number(left.state === "archived") - Number(right.state === "archived");
  if (state !== 0) return state;

  const repository = compareText(left.repository, right.repository);
  if (repository !== 0) return repository;

  const newest = compareText(right.modifiedAt, left.modifiedAt);
  return newest !== 0 ? newest : compareText(left.path, right.path);
}

function defaultLogError(message: string, error: unknown): void {
  console.error(`[ai-handoffs] ${message}`, error);
}

function isMissing(error: unknown): boolean {
  return error instanceof Error && "code" in error && error.code === "ENOENT";
}

async function immediateDirectories(directory: string, logError: ScanOptions["logError"]): Promise<string[]> {
  try {
    const entries = await readdir(directory, { withFileTypes: true });
    return entries
      .filter((entry) => entry.isDirectory())
      .map((entry) => join(directory, entry.name))
      .sort(compareText);
  } catch (error) {
    if (!isMissing(error)) logError?.(`unable to scan root ${directory}`, error);
    return [];
  }
}

async function scanTarget(
  target: ScanTarget,
  logError: NonNullable<ScanOptions["logError"]>,
): Promise<HandoffRecord[]> {
  let entries;
  try {
    entries = await readdir(target.handoffDirectory, { withFileTypes: true });
  } catch (error) {
    if (!isMissing(error)) logError(`unable to scan handoff directory ${target.handoffDirectory}`, error);
    return [];
  }

  const filenames = entries
    .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".md"))
    .map((entry) => entry.name)
    .sort(compareText);

  const records = await Promise.all(
    filenames.map(async (filename): Promise<HandoffRecord | null> => {
      const path = join(target.handoffDirectory, filename);
      try {
        const [source, metadata] = await Promise.all([readFile(path, "utf8"), stat(path)]);
        const parsed = parseHandoff(source, filename);
        return {
          id: path,
          state: target.state,
          root: target.root,
          repository: target.repository,
          filename,
          path,
          modifiedAt: metadata.mtime.toISOString(),
          ...parsed,
        };
      } catch (error) {
        logError(`unable to read handoff ${path}`, error);
        return null;
      }
    }),
  );

  return records.filter((record): record is HandoffRecord => record !== null);
}

export async function scanHandoffs(options: ScanOptions = {}): Promise<HandoffRecord[]> {
  const home = options.homeDir ?? homedir();
  const logError = options.logError ?? defaultLogError;
  const targets: ScanTarget[] = [];

  for (const container of [join(home, "projects"), join(home, "work")]) {
    for (const repositoryRoot of await immediateDirectories(container, logError)) {
      targets.push({
        state: "live",
        root: container,
        repository: basename(repositoryRoot),
        handoffDirectory: join(repositoryRoot, ".ai", "task-handoffs"),
      });
    }
  }

  const desktop = join(home, "Desktop");
  targets.push({
    state: "live",
    root: desktop,
    repository: "Desktop",
    handoffDirectory: join(desktop, ".ai", "task-handoffs"),
  });

  const archive = join(home, ".local", "share", "task-handoffs", "archive");
  for (const originDirectory of await immediateDirectories(archive, logError)) {
    targets.push({
      state: "archived",
      root: archive,
      repository: basename(originDirectory),
      handoffDirectory: originDirectory,
    });
  }

  const groups = await Promise.all(targets.map((target) => scanTarget(target, logError)));
  return groups.flat().sort(compareHandoffs);
}
