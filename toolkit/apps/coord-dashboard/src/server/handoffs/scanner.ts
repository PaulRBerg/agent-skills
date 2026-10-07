import { lstatSync, readdirSync, readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import nodePath from "node:path";

import type { HandoffRecord } from "../../lib/handoff-types.js";
import { parseHandoff } from "./parser.js";

export type ScanOptions = {
  homeDir?: string;
  logError?: (message: string, error: unknown) => void;
};

export type ScanTarget = {
  state: HandoffRecord["state"];
  root: string;
  repository: string;
  handoffDirectory: string;
};

function compareText(left: string, right: string): number {
  if (left === right) {
    return 0;
  }
  return left < right ? -1 : 1;
}

export function compareHandoffs(left: HandoffRecord, right: HandoffRecord): number {
  const state = Number(left.state === "archived") - Number(right.state === "archived");
  if (state !== 0) {
    return state;
  }

  const repository = compareText(left.repository, right.repository);
  if (repository !== 0) {
    return repository;
  }

  const newest = compareText(right.modifiedAt, left.modifiedAt);
  return newest === 0 ? compareText(left.path, right.path) : newest;
}

function defaultLogError(message: string, error: unknown): void {
  console.error(`[ai-coord-dashboard] ${message}`, error);
}

function isMissing(error: unknown): boolean {
  return error instanceof Error && "code" in error && error.code === "ENOENT";
}

function immediateDirectories(directory: string, logError: ScanOptions["logError"]): string[] {
  try {
    const entries = readdirSync(directory, { withFileTypes: true });
    return entries
      .filter((entry) => entry.isDirectory())
      .map((entry) => nodePath.join(directory, entry.name))
      .toSorted(compareText);
  } catch (error) {
    if (!isMissing(error)) {
      logError?.(`unable to scan root ${directory}`, error);
    }
    return [];
  }
}

function hasPhysicalHandoffDirectory(
  target: ScanTarget,
  logError: NonNullable<ScanOptions["logError"]>
): boolean {
  const directories =
    target.state === "live"
      ? [
          nodePath.dirname(nodePath.dirname(target.handoffDirectory)),
          nodePath.dirname(target.handoffDirectory),
          target.handoffDirectory,
        ]
      : [target.handoffDirectory];

  for (const directory of directories) {
    try {
      if (!lstatSync(directory).isDirectory()) {
        return false;
      }
    } catch (error) {
      if (!isMissing(error)) {
        logError(`unable to inspect handoff directory ${directory}`, error);
      }
      return false;
    }
  }
  return true;
}

export function scanTarget(
  target: ScanTarget,
  logError: NonNullable<ScanOptions["logError"]>
): HandoffRecord[] {
  if (!hasPhysicalHandoffDirectory(target, logError)) {
    return [];
  }

  let entries;
  try {
    entries = readdirSync(target.handoffDirectory, { withFileTypes: true });
  } catch (error) {
    if (!isMissing(error)) {
      logError(`unable to scan handoff directory ${target.handoffDirectory}`, error);
    }
    return [];
  }

  const filenames = entries
    .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".md"))
    .map((entry) => entry.name)
    .toSorted(compareText);

  const records = filenames.map((filename): HandoffRecord | null => {
    const path = nodePath.join(target.handoffDirectory, filename);
    try {
      const source = readFileSync(path, "utf-8");
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
  logError: NonNullable<ScanOptions["logError"]>
): Promise<HandoffRecord[]> {
  const worker = Bun.spawn(
    [process.execPath, nodePath.join(import.meta.dir, "scanner-worker.ts"), JSON.stringify(target)],
    {
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    }
  );
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    worker.kill("SIGKILL");
  }, 1000);

  const [output, errors, exitCode] = await Promise.all([
    new Response(worker.stdout).text(),
    new Response(worker.stderr).text(),
    worker.exited,
  ]);
  clearTimeout(timeout);

  if (timedOut) {
    logError(
      `timed out scanning protected handoff directory ${target.handoffDirectory}`,
      "scan timed out"
    );
    return [];
  }
  if (exitCode !== 0) {
    logError(
      `unable to scan protected handoff directory ${target.handoffDirectory}`,
      errors.trim() || exitCode
    );
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

  for (const container of [nodePath.join(home, "projects"), nodePath.join(home, "work")]) {
    for (const repositoryRoot of immediateDirectories(container, logError)) {
      targets.push({
        state: "live",
        root: container,
        repository: nodePath.basename(repositoryRoot),
        handoffDirectory: nodePath.join(repositoryRoot, ".ai", "task-handoffs"),
      });
    }
  }

  const desktop = nodePath.join(home, "Desktop");
  const desktopTarget: ScanTarget = {
    state: "live",
    root: desktop,
    repository: "Desktop",
    handoffDirectory: nodePath.join(desktop, ".ai", "task-handoffs"),
  };

  const archive = nodePath.join(home, ".local", "share", "task-handoffs", "archive");
  for (const originDirectory of immediateDirectories(archive, logError)) {
    targets.push({
      state: "archived",
      root: archive,
      repository: nodePath.basename(originDirectory),
      handoffDirectory: originDirectory,
    });
  }

  const groups = targets.map((target) => scanTarget(target, logError));
  groups.push(
    options.homeDir === undefined
      ? await scanIsolatedTarget(desktopTarget, logError)
      : scanTarget(desktopTarget, logError)
  );
  return groups.flat().toSorted(compareHandoffs);
}
