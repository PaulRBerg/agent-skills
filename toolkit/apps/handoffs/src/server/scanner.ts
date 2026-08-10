import { readdirSync, readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { basename, join } from "node:path";

import type { HandoffRecord } from "../shared/handoff";
import { parseHandoff } from "./parser";

export interface ScanOptions {
  homeDir?: string;
  logError?: (message: string, error: unknown) => void;
}

export interface ScanTarget {
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

function immediateDirectories(directory: string, logError: ScanOptions["logError"]): string[] {
  try {
    const entries = readdirSync(directory, { withFileTypes: true });
    return entries
      .filter((entry) => entry.isDirectory())
      .map((entry) => join(directory, entry.name))
      .sort(compareText);
  } catch (error) {
    if (!isMissing(error)) logError?.(`unable to scan root ${directory}`, error);
    return [];
  }
}

export function scanTarget(target: ScanTarget, logError: NonNullable<ScanOptions["logError"]>): HandoffRecord[] {
  let entries;
  try {
    entries = readdirSync(target.handoffDirectory, { withFileTypes: true });
  } catch (error) {
    if (!isMissing(error)) logError(`unable to scan handoff directory ${target.handoffDirectory}`, error);
    return [];
  }

  const filenames = entries
    .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".md"))
    .map((entry) => entry.name)
    .sort(compareText);

  const records = filenames.map((filename): HandoffRecord | null => {
    const path = join(target.handoffDirectory, filename);
    try {
      const source = readFileSync(path, "utf8");
      const metadata = statSync(path);
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
  });

  return records.filter((record): record is HandoffRecord => record !== null);
}

async function scanIsolatedTarget(
  target: ScanTarget,
  logError: NonNullable<ScanOptions["logError"]>,
): Promise<HandoffRecord[]> {
  const worker = Bun.spawn([process.execPath, join(import.meta.dir, "scanner-worker.ts"), JSON.stringify(target)], {
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    worker.kill("SIGKILL");
  }, 1_000);

  const [output, errors, exitCode] = await Promise.all([
    new Response(worker.stdout).text(),
    new Response(worker.stderr).text(),
    worker.exited,
  ]);
  clearTimeout(timeout);

  if (timedOut) {
    logError(`timed out scanning protected handoff directory ${target.handoffDirectory}`, "scan timed out");
    return [];
  }
  if (exitCode !== 0) {
    logError(`unable to scan protected handoff directory ${target.handoffDirectory}`, errors.trim() || exitCode);
    return [];
  }

  try {
    return JSON.parse(output) as HandoffRecord[];
  } catch (error) {
    logError(`unable to parse scan result for ${target.handoffDirectory}`, error);
    return [];
  }
}

export async function scanHandoffs(options: ScanOptions = {}): Promise<HandoffRecord[]> {
  const home = options.homeDir ?? homedir();
  const logError = options.logError ?? defaultLogError;
  const targets: ScanTarget[] = [];

  for (const container of [join(home, "projects"), join(home, "work")]) {
    for (const repositoryRoot of immediateDirectories(container, logError)) {
      targets.push({
        state: "live",
        root: container,
        repository: basename(repositoryRoot),
        handoffDirectory: join(repositoryRoot, ".ai", "task-handoffs"),
      });
    }
  }

  const desktop = join(home, "Desktop");
  const desktopTarget: ScanTarget = {
    state: "live",
    root: desktop,
    repository: "Desktop",
    handoffDirectory: join(desktop, ".ai", "task-handoffs"),
  };

  const archive = join(home, ".local", "share", "task-handoffs", "archive");
  for (const originDirectory of immediateDirectories(archive, logError)) {
    targets.push({
      state: "archived",
      root: archive,
      repository: basename(originDirectory),
      handoffDirectory: originDirectory,
    });
  }

  const groups = targets.map((target) => scanTarget(target, logError));
  groups.push(
    options.homeDir === undefined
      ? await scanIsolatedTarget(desktopTarget, logError)
      : scanTarget(desktopTarget, logError),
  );
  return groups.flat().sort(compareHandoffs);
}
