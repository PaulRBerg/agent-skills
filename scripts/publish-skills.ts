#!/usr/bin/env bun

import { execFileSync, spawn } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

type Command = "apply" | "check" | "plan";
type InstallTarget = "claude" | "codex" | "shared";
type DriftTarget = "agents" | "claude" | "codex" | "lock" | "source";
type NodeKind = "directory" | "file" | "missing" | "other" | "symlink";

type CliOptions = {
  command: Command;
  expectedHead: string | null;
  json: boolean;
  skills: string[];
};

type Drift = {
  detail: string;
  kind: string;
  path: string;
  skill: string;
  target: DriftTarget;
};

type SnapshotEntry =
  { executable: boolean; hash: string; type: "file" } | { target: string; type: "symlink" } | { type: "unsupported" };
type Snapshot = Map<string, SnapshotEntry>;
type IndexedTree = Map<string, IndexedTree | string>;

type SourceSkill = {
  name: string;
  root: string;
  snapshot: Snapshot;
  target: InstallTarget;
  treeHash: string;
};

type CliLock = {
  data: { skills: Record<string, unknown>; version: number };
  fingerprint: string;
  raw: string;
};

type Groups = Record<InstallTarget | "remove", string[]>;
type Plan = {
  candidateNames: string[];
  clean: boolean;
  drifts: Drift[];
  groups: Groups;
  lock: CliLock;
  selected: string[];
  sourceSkills: Map<string, SourceSkill>;
};

type ProcessLock = { owner: string; token: string };
type GlobalChanges = { lockChanged: boolean; paths: string[] };
type TargetPaths = Record<"agents" | "claude" | "codex", string>;
type ClaimScope = "file" | "recursive";
type RepoClaim = { canonical: boolean; paths: Array<{ path: string; scope: ClaimScope }>; root: string };

const repository = "PaulRBerg/agent-skills";
const sourceUrl = "https://github.com/PaulRBerg/agent-skills.git";
const validSkillName = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const scriptRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const homeDir = os.homedir();
const stateRoot = process.env.XDG_STATE_HOME ?? path.join(homeDir, ".local", "state");
const config = {
  agentsRoot: path.resolve(process.env.PUBLISH_SKILLS_AGENTS_ROOT ?? path.join(homeDir, ".agents")),
  bunx: process.env.PUBLISH_SKILLS_BUNX ?? "bunx",
  claudeRoot: path.resolve(process.env.PUBLISH_SKILLS_CLAUDE_ROOT ?? path.join(homeDir, ".claude")),
  codexRoot: path.resolve(process.env.PUBLISH_SKILLS_CODEX_ROOT ?? path.join(homeDir, ".codex")),
  lockFile: path.resolve(process.env.PUBLISH_SKILLS_LOCK_FILE ?? path.join(stateRoot, "skills", ".skill-lock.json")),
  processLock: path.resolve(
    process.env.PUBLISH_SKILLS_PROCESS_LOCK ?? path.join(stateRoot, "skills", ".publish-skills.lock"),
  ),
  sourceRoot: path.resolve(process.env.PUBLISH_SKILLS_SOURCE_ROOT ?? scriptRoot),
};

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.command === "apply") {
    process.exitCode = await applyPlan(options);
  } else {
    const plan = createPlan(options.skills);
    printPlan(plan, options.json, options.skills);
    if (options.command === "check" && plan.drifts.length > 0) process.exitCode = 1;
  }
} catch (error) {
  console.error(`Error: ${errorMessage(error)}`);
  process.exitCode = 2;
}

function parseArgs(args: string[]): CliOptions {
  const command = args.shift();
  if (!command || !["apply", "check", "plan"].includes(command)) {
    throw new Error(
      "Usage: publish-skills.ts plan|check [--json] [--skill NAME]... | " +
        "apply --expected-head SHA [--skill NAME]...",
    );
  }

  const options: CliOptions = { command: command as Command, expectedHead: null, json: false, skills: [] };
  while (args.length > 0) {
    const flag = args.shift();
    if (flag === "--json") {
      if (command === "apply") throw new Error("--json is available only for plan and check.");
      options.json = true;
      continue;
    }
    if (flag === "--skill") {
      const name = args.shift();
      if (!name || !validSkillName.test(name)) throw new Error(`Invalid --skill value: ${name ?? "missing"}`);
      options.skills.push(name);
      continue;
    }
    if (flag === "--expected-head") {
      if (command !== "apply") throw new Error("--expected-head is available only for apply.");
      options.expectedHead = args.shift() ?? null;
      if (!options.expectedHead) throw new Error("--expected-head requires a SHA.");
      continue;
    }
    throw new Error(`Unknown argument: ${flag}`);
  }

  options.skills = [...new Set(options.skills)].sort();
  if (command === "apply" && !options.expectedHead) {
    throw new Error("apply requires --expected-head SHA.");
  }
  return options;
}

function createPlan(requestedSkills: string[], { emptyMeansAll = true }: { emptyMeansAll?: boolean } = {}): Plan {
  const sourceSkills = readSourceSkills();
  const lock = readCliLock();
  const sourceOwnedLockNames = Object.entries(lock.data.skills)
    .filter(([, entry]) => isObject(entry) && entry.source === repository)
    .map(([name]) => name);
  const candidateNames = [...new Set([...sourceSkills.keys(), ...sourceOwnedLockNames])].sort();
  const selected = requestedSkills.length > 0 ? requestedSkills : emptyMeansAll ? candidateNames : [];
  const unknown = selected.filter((name) => !candidateNames.includes(name));
  if (unknown.length > 0)
    throw new Error(`Unknown source-owned skill${unknown.length === 1 ? "" : "s"}: ${unknown.join(", ")}`);

  const drifts: Drift[] = [];
  const restrictionsChanged = new Set<string>();
  for (const name of selected) {
    const skill = sourceSkills.get(name);
    const lockEntry = lock.data.skills[name];
    if (!skill) {
      addDrift(
        drifts,
        name,
        "source",
        "deleted",
        path.join(config.sourceRoot, "skills", name),
        "source skill was deleted",
      );
      inspectDeletedTargets(name, drifts);
      continue;
    }

    inspectLockEntry(skill, lockEntry, drifts);
    if (inspectTargetLayout(skill, drifts)) restrictionsChanged.add(name);
  }

  drifts.sort(compareDrifts);
  const driftNames = new Set(drifts.map((drift) => drift.skill));
  const groups: Groups = { claude: [], codex: [], remove: [], shared: [] };
  for (const name of selected) {
    if (!driftNames.has(name)) continue;
    const skill = sourceSkills.get(name);
    if (!skill) {
      groups.remove.push(name);
      continue;
    }
    if (restrictionsChanged.has(name)) groups.remove.push(name);
    groups[skill.target].push(name);
  }
  for (const names of Object.values(groups)) names.sort();

  return {
    candidateNames,
    clean: drifts.length === 0,
    drifts,
    groups,
    lock,
    selected,
    sourceSkills,
  };
}

function readSourceSkills(): Map<string, SourceSkill> {
  const skillsRoot = path.join(config.sourceRoot, "skills");
  const sourceFiles = readSourceFileIndex();
  let entries;
  try {
    entries = fs.readdirSync(skillsRoot, { withFileTypes: true });
  } catch (error) {
    throw new Error(`Cannot read source skills at ${skillsRoot}: ${errorMessage(error)}`);
  }

  const skills = new Map<string, SourceSkill>();
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (!entry.isDirectory() || !validSkillName.test(entry.name)) continue;
    const root = path.join(skillsRoot, entry.name);
    const skillFile = path.join(root, "SKILL.md");
    if (!isRegularFile(skillFile)) continue;
    const frontmatter = readFrontmatter(skillFile);
    validateFrontmatterName(skillFile, entry.name, frontmatter);
    const relativeFiles = sourceFiles
      .filter((sourcePath) => sourcePath.startsWith(`skills/${entry.name}/`))
      .map((sourcePath) => sourcePath.slice(`skills/${entry.name}/`.length));
    skills.set(entry.name, {
      name: entry.name,
      root,
      snapshot: snapshotDirectory(root, relativeFiles),
      target: readInstallTarget(skillFile, frontmatter),
      treeHash: gitTreeHash(root, relativeFiles),
    });
  }
  return skills;
}

function readSourceFileIndex(): string[] {
  let output: string;
  try {
    output = execFileSync("git", ["ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "skills"], {
      cwd: config.sourceRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    throw new Error(`Cannot enumerate source skill files: ${commandErrorMessage(error)}`);
  }
  return output
    .split("\0")
    .filter((sourcePath) => sourcePath && nodeKind(path.join(config.sourceRoot, sourcePath)) !== "missing");
}

function readFrontmatter(skillFile: string): string {
  const text = fs.readFileSync(skillFile, "utf8");
  const frontmatter = text.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/)?.[1];
  if (frontmatter === undefined) throw new Error(`${skillFile}: missing YAML frontmatter.`);
  return frontmatter;
}

function readInstallTarget(skillFile: string, frontmatter: string): InstallTarget {
  const lines = frontmatter.split(/\r?\n/);
  let value: string | undefined;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (!line || !/^metadata:\s*$/.test(line)) continue;
    for (index += 1; index < lines.length; index += 1) {
      const nestedLine = lines[index];
      if (nestedLine === undefined || !/^(?:\s|$)/.test(nestedLine)) break;
      const match = nestedLine.match(/^\s{2}install-targets:\s*(.*?)\s*$/);
      if (match?.[1] !== undefined) value = stripYamlQuotes(match[1]);
    }
    break;
  }

  if (value === undefined || value === "claude-code codex") return "shared";
  if (value === "claude-code") return "claude";
  if (value === "codex") return "codex";
  throw new Error(`${skillFile}: invalid metadata.install-targets: ${JSON.stringify(value)}`);
}

function validateFrontmatterName(skillFile: string, directoryName: string, frontmatter: string): void {
  for (const line of frontmatter.split(/\r?\n/)) {
    const match = line.match(/^name:\s*(.*?)\s*$/);
    if (match?.[1] === undefined) continue;
    const value = stripYamlQuotes(match[1]);
    if (value !== directoryName) {
      throw new Error(`${skillFile}: frontmatter name "${value}" does not match directory "${directoryName}"`);
    }
    return;
  }
}

function stripYamlQuotes(value: string): string {
  if (
    value.length >= 2 &&
    ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'")))
  ) {
    return value.slice(1, -1);
  }
  return value;
}

function readCliLock(): CliLock {
  let raw: string;
  try {
    raw = fs.readFileSync(config.lockFile, "utf8");
  } catch (error) {
    throw new Error(`Cannot read skills CLI lock at ${config.lockFile}: ${errorMessage(error)}`);
  }

  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch (error) {
    throw new Error(`Malformed skills CLI lock at ${config.lockFile}: ${errorMessage(error)}`);
  }
  if (!isObject(data) || data.version !== 3 || !isObject(data.skills)) {
    throw new Error(`${config.lockFile}: expected skills CLI lock version 3 with a skills object.`);
  }
  return { data: data as CliLock["data"], fingerprint: digest(Buffer.from(raw)), raw };
}

function inspectLockEntry(skill: SourceSkill, entry: unknown, drifts: Drift[]): void {
  const lockPath = config.lockFile;
  if (!isObject(entry)) {
    addDrift(drifts, skill.name, "lock", "lock-metadata", lockPath, "entry is missing or is not an object");
    return;
  }

  const expected = {
    skillFolderHash: skill.treeHash,
    skillPath: `skills/${skill.name}/SKILL.md`,
    source: repository,
    sourceType: "github",
    sourceUrl,
  };
  for (const [field, value] of Object.entries(expected)) {
    if (entry[field] === value) continue;
    addDrift(
      drifts,
      skill.name,
      "lock",
      `lock-${field.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`,
      lockPath,
      `${field} is ${JSON.stringify(entry[field] ?? null)}, expected ${JSON.stringify(value)}`,
    );
  }
}

function inspectTargetLayout(skill: SourceSkill, drifts: Drift[]): boolean {
  const paths = targetPaths(skill.name);
  const agentsKind = nodeKind(paths.agents);
  const claudeKind = nodeKind(paths.claude);
  const codexKind = nodeKind(paths.codex);
  let restrictionChanged = false;

  if (skill.target === "shared") {
    compareInstalledDirectory(skill, paths.agents, "agents", drifts);
    expectClaudeSymlink(skill.name, paths, drifts);
    expectAbsent(skill.name, "codex", paths.codex, drifts);
    restrictionChanged = claudeKind === "directory" || codexKind !== "missing";
  } else if (skill.target === "claude") {
    expectAbsent(skill.name, "agents", paths.agents, drifts);
    compareInstalledDirectory(skill, paths.claude, "claude", drifts);
    expectAbsent(skill.name, "codex", paths.codex, drifts);
    restrictionChanged = agentsKind !== "missing" || claudeKind === "symlink" || codexKind !== "missing";
  } else {
    compareInstalledDirectory(skill, paths.agents, "agents", drifts);
    expectAbsent(skill.name, "claude", paths.claude, drifts);
    expectAbsent(skill.name, "codex", paths.codex, drifts);
    restrictionChanged = claudeKind !== "missing" || codexKind !== "missing";
  }
  return restrictionChanged;
}

function inspectDeletedTargets(name: string, drifts: Drift[]): void {
  for (const [target, targetPath] of Object.entries(targetPaths(name))) {
    if (nodeKind(targetPath) === "missing") continue;
    addDrift(
      drifts,
      name,
      target as keyof TargetPaths,
      "deleted-install",
      targetPath,
      "deleted source skill remains installed",
    );
  }
}

function targetPaths(name: string): TargetPaths {
  return {
    agents: path.join(config.agentsRoot, "skills", name),
    claude: path.join(config.claudeRoot, "skills", name),
    codex: path.join(config.codexRoot, "skills", name),
  };
}

function compareInstalledDirectory(skill: SourceSkill, targetPath: string, target: DriftTarget, drifts: Drift[]): void {
  const kind = nodeKind(targetPath);
  if (kind === "missing") {
    addDrift(drifts, skill.name, target, "missing", targetPath, "required installation is missing");
    return;
  }
  if (kind !== "directory") {
    addDrift(drifts, skill.name, target, "layout", targetPath, `expected a directory, found ${kind}`);
    return;
  }

  const actual = snapshotInstalledDirectory(targetPath, target);
  const relativePaths = [...new Set([...skill.snapshot.keys(), ...actual.keys()])].sort();
  for (const relativePath of relativePaths) {
    const expectedEntry = skill.snapshot.get(relativePath);
    const actualEntry = actual.get(relativePath);
    if (!actualEntry) {
      addDrift(drifts, skill.name, target, "content", targetPath, `${relativePath} is missing`);
    } else if (!expectedEntry) {
      addDrift(drifts, skill.name, target, "extra", targetPath, `${relativePath} is not in the source`);
    } else if (expectedEntry.type !== actualEntry.type) {
      addDrift(
        drifts,
        skill.name,
        target,
        "content",
        targetPath,
        `${relativePath} is ${actualEntry.type}, expected ${expectedEntry.type}`,
      );
    } else if (
      expectedEntry.type === "file" &&
      actualEntry.type === "file" &&
      expectedEntry.hash !== actualEntry.hash
    ) {
      addDrift(drifts, skill.name, target, "content", targetPath, `${relativePath} content differs`);
    } else if (
      expectedEntry.type === "file" &&
      actualEntry.type === "file" &&
      expectedEntry.executable !== actualEntry.executable
    ) {
      addDrift(
        drifts,
        skill.name,
        target,
        "mode",
        targetPath,
        `${relativePath} executable bit is ${actualEntry.executable}, expected ${expectedEntry.executable}`,
      );
    } else if (
      expectedEntry.type === "symlink" &&
      actualEntry.type === "symlink" &&
      expectedEntry.target !== actualEntry.target
    ) {
      addDrift(drifts, skill.name, target, "content", targetPath, `${relativePath} symlink target differs`);
    }
  }
}

function snapshotInstalledDirectory(targetPath: string, target: DriftTarget): Snapshot {
  const repositoryRoot =
    target === "agents" ? config.agentsRoot : target === "claude" ? config.claudeRoot : config.codexRoot;
  try {
    const relativeRoot = path.relative(repositoryRoot, targetPath);
    const output = execFileSync(
      "git",
      ["ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", relativeRoot],
      { cwd: repositoryRoot, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    );
    const prefix = `${relativeRoot}/`;
    const relativeFiles = output
      .split("\0")
      .filter((repositoryPath) => repositoryPath.startsWith(prefix))
      .map((repositoryPath) => repositoryPath.slice(prefix.length))
      .filter((relativePath) => relativePath && nodeKind(path.join(targetPath, relativePath)) !== "missing");
    return snapshotDirectory(targetPath, relativeFiles);
  } catch {
    return snapshotDirectory(targetPath);
  }
}

function expectClaudeSymlink(name: string, paths: TargetPaths, drifts: Drift[]): void {
  const kind = nodeKind(paths.claude);
  const expected = path.relative(path.dirname(paths.claude), paths.agents);
  if (kind === "missing") {
    addDrift(drifts, name, "claude", "missing", paths.claude, "required Claude Code symlink is missing");
  } else if (kind === "directory" || kind === "file") {
    addDrift(drifts, name, "claude", "forbidden-copy", paths.claude, `expected symlink to ${expected}`);
  } else if (kind !== "symlink") {
    addDrift(drifts, name, "claude", "layout", paths.claude, `expected symlink to ${expected}, found ${kind}`);
  } else {
    const actual = fs.readlinkSync(paths.claude);
    if (actual !== expected) {
      addDrift(drifts, name, "claude", "symlink", paths.claude, `target is ${actual}, expected ${expected}`);
    }
  }
}

function expectAbsent(name: string, target: DriftTarget, targetPath: string, drifts: Drift[]): void {
  const kind = nodeKind(targetPath);
  if (kind === "missing") return;
  addDrift(
    drifts,
    name,
    target,
    kind === "directory" ? "forbidden-copy" : "forbidden",
    targetPath,
    `expected absence, found ${kind}`,
  );
}

function snapshotDirectory(root: string, relativeFiles: string[] | null = null): Snapshot {
  const snapshot: Snapshot = new Map();
  if (relativeFiles) {
    for (const relativePath of relativeFiles.sort(compareNames)) snapshotPath(root, relativePath, snapshot);
  } else {
    walkDirectory(root, "", snapshot);
  }
  return snapshot;
}

function walkDirectory(root: string, relativeRoot: string, snapshot: Snapshot): void {
  const current = relativeRoot ? path.join(root, relativeRoot) : root;
  const names = fs.readdirSync(current).sort(compareNames);
  for (const name of names) {
    const relativePath = relativeRoot ? path.join(relativeRoot, name) : name;
    const fullPath = path.join(root, relativePath);
    const stats = fs.lstatSync(fullPath);
    if (stats.isDirectory()) {
      walkDirectory(root, relativePath, snapshot);
    } else {
      snapshotPath(root, relativePath, snapshot);
    }
  }
}

function snapshotPath(root: string, relativePath: string, snapshot: Snapshot): void {
  const fullPath = path.join(root, relativePath);
  const stats = fs.lstatSync(fullPath);
  if (stats.isFile()) {
    snapshot.set(relativePath, {
      executable: (stats.mode & 0o111) !== 0,
      hash: digest(fs.readFileSync(fullPath)),
      type: "file",
    });
  } else if (stats.isSymbolicLink()) {
    snapshot.set(relativePath, { target: fs.readlinkSync(fullPath), type: "symlink" });
  } else {
    snapshot.set(relativePath, { type: "unsupported" });
  }
}

function gitTreeHash(root: string, relativeFiles: string[]): string {
  const tree: IndexedTree = new Map();
  for (const relativePath of relativeFiles) {
    const parts = relativePath.split("/");
    let current = tree;
    for (const part of parts.slice(0, -1)) {
      const existing = current.get(part);
      if (existing instanceof Map) {
        current = existing;
      } else {
        const child: IndexedTree = new Map();
        current.set(part, child);
        current = child;
      }
    }
    const name = parts.at(-1);
    if (name) current.set(name, relativePath);
  }
  return hashIndexedTree(root, tree).toString("hex");
}

function hashIndexedTree(root: string, tree: IndexedTree): Buffer {
  const entries = [...tree.entries()].map(([name, value]) => {
    if (value instanceof Map) return { mode: "40000", name, oid: hashIndexedTree(root, value), tree: true };
    const fullPath = path.join(root, value);
    const stats = fs.lstatSync(fullPath);
    if (stats.isSymbolicLink()) {
      return { mode: "120000", name, oid: hashGitObject("blob", Buffer.from(fs.readlinkSync(fullPath))), tree: false };
    }
    if (stats.isFile()) {
      const mode = (stats.mode & 0o111) !== 0 ? "100755" : "100644";
      return { mode, name, oid: hashGitObject("blob", fs.readFileSync(fullPath)), tree: false };
    }
    throw new Error(`${fullPath}: unsupported source file type.`);
  });
  entries.sort((left, right) =>
    Buffer.compare(Buffer.from(left.name + (left.tree ? "/" : "")), Buffer.from(right.name + (right.tree ? "/" : ""))),
  );
  const body = Buffer.concat(entries.flatMap((entry) => [Buffer.from(`${entry.mode} ${entry.name}\0`), entry.oid]));
  return hashGitObject("tree", body);
}

function hashGitObject(type: "blob" | "tree", body: Buffer): Buffer {
  return createHash("sha1").update(`${type} ${body.length}\0`).update(body).digest();
}

function digest(body: Buffer): string {
  return createHash("sha256").update(body).digest("hex");
}

async function applyPlan(options: CliOptions): Promise<number> {
  const processLock = acquireProcessLock();
  let before: Map<string, string> | undefined;
  let beforeLockFingerprint: string | undefined;
  let names: string[] = [];
  const completed: string[] = [];
  try {
    const plan = createPlan(options.skills);
    assertApplyGuards(options.expectedHead, plan);
    names = plan.candidateNames;
    before = snapshotGlobalPaths(names);
    beforeLockFingerprint = plan.lock.fingerprint;

    const commands = buildCommands(plan.groups);
    for (const command of commands) {
      assertApplyGuards(options.expectedHead, plan);
      await runSkillsCli(command.args);
      completed.push(command.label);
    }

    assertApplyGuards(options.expectedHead, plan);
    verifyAppliedPlan(options.skills, plan, names);
    const changes = collectGlobalChanges(before, names, beforeLockFingerprint);
    printApplyResult(true, completed, changes);
    return 0;
  } catch (error) {
    const changes = before
      ? collectGlobalChanges(before, names, beforeLockFingerprint)
      : { lockChanged: false, paths: [] };
    printApplyResult(false, completed, changes);
    console.error(`Apply failed: ${errorMessage(error)}`);
    return 1;
  } finally {
    releaseProcessLock(processLock);
  }
}

function buildCommands(groups: Groups): Array<{ args: string[]; label: string }> {
  const commands: Array<{ args: string[]; label: string }> = [];
  if (groups.remove.length > 0) {
    commands.push({
      args: ["skills", "remove", "--global", "--skill", ...groups.remove, "--yes"],
      label: `remove: ${groups.remove.join(", ")}`,
    });
  }
  if (groups.shared.length > 0) {
    commands.push({
      args: [
        "skills",
        "add",
        repository,
        "--global",
        "--agent",
        "claude-code",
        "codex",
        "--skill",
        ...groups.shared,
        "--yes",
      ],
      label: `shared: ${groups.shared.join(", ")}`,
    });
  }
  if (groups.claude.length > 0) {
    commands.push({
      args: ["skills", "add", repository, "--global", "--agent", "claude-code", "--skill", ...groups.claude, "--yes"],
      label: `claude: ${groups.claude.join(", ")}`,
    });
  }
  if (groups.codex.length > 0) {
    commands.push({
      args: ["skills", "add", repository, "--global", "--agent", "codex", "--skill", ...groups.codex, "--yes"],
      label: `codex: ${groups.codex.join(", ")}`,
    });
  }
  return commands;
}

async function runSkillsCli(args: string[]): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const child = spawn(config.bunx, args, { cwd: config.agentsRoot, stdio: "inherit" });
    const signals: NodeJS.Signals[] = ["SIGINT", "SIGTERM"];
    const handlers = new Map(signals.map((signal) => [signal, () => child.kill(signal)]));
    for (const [signal, handler] of handlers) process.once(signal, handler);

    const cleanup = () => {
      for (const [signal, handler] of handlers) process.removeListener(signal, handler);
    };
    child.once("error", (error) => {
      cleanup();
      reject(new Error(`${config.bunx} failed to start: ${error.message}`));
    });
    child.once("exit", (code, signal) => {
      cleanup();
      if (signal) reject(new Error(`${config.bunx} ${args.slice(0, 2).join(" ")} terminated by ${signal}`));
      else if (code !== 0) reject(new Error(`${config.bunx} ${args.slice(0, 2).join(" ")} exited ${code}`));
      else resolve();
    });
  });
}

function assertApplyGuards(expectedHead: string | null, plan: Plan): void {
  if (!expectedHead) throw new Error("apply requires --expected-head SHA");
  const branch = git(["branch", "--show-current"]);
  if (branch !== "main") throw new Error(`source branch is ${branch || "detached"}, expected main`);
  const head = git(["rev-parse", "HEAD"]);
  const expected = git(["rev-parse", "--verify", `${expectedHead}^{commit}`]);
  if (head !== expected) throw new Error(`HEAD is ${head}, expected ${expected}`);

  let upstream;
  try {
    upstream = git(["rev-parse", "@{upstream}"]);
  } catch {
    throw new Error("main has no readable upstream");
  }
  if (head !== upstream) throw new Error(`main ${head} does not equal upstream ${upstream}`);

  const sourcePaths = plan.selected.filter((name) => plan.sourceSkills.has(name)).map((name) => `skills/${name}`);
  if (sourcePaths.length === 0) return;
  const dirty = git(["status", "--porcelain=v1", "--untracked-files=all", "--", ...sourcePaths]);
  if (dirty) throw new Error(`selected source paths are dirty:\n${dirty}`);
  readCliLock();
}

function git(args: string[]): string {
  return execFileSync("git", args, {
    cwd: config.sourceRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function verifyAppliedPlan(requestedSkills: string[], currentPlan: Plan, originalNames: string[]): void {
  const requested = requestedSkills.length > 0 ? requestedSkills : originalNames;
  const currentSourceNames = requested.filter((name) => currentPlan.sourceSkills.has(name));
  const verification = createPlan(currentSourceNames, { emptyMeansAll: false });
  const residual = [...verification.drifts];
  const lock = verification.lock.data.skills;
  for (const name of requested.filter((candidate) => !currentPlan.sourceSkills.has(candidate))) {
    const entry = lock[name];
    if (isObject(entry) && entry.source === repository) {
      addDrift(residual, name, "lock", "deleted", config.lockFile, "deleted source skill remains in the CLI lock");
    }
    inspectDeletedTargets(name, residual);
  }
  if (residual.length > 0) {
    throw new Error(
      `verification found ${residual.length} residual drift finding${residual.length === 1 ? "" : "s"}: ` +
        [...new Set(residual.map((drift) => drift.skill))].sort().join(", "),
    );
  }
}

function acquireProcessLock(): ProcessLock {
  const token = `${process.pid}-${randomUUID()}`;
  try {
    fs.mkdirSync(config.processLock);
  } catch (error) {
    if (errorCode(error) === "EEXIST") throw new Error(`another publish-skills apply holds ${config.processLock}`);
    throw new Error(`cannot create process lock ${config.processLock}: ${errorMessage(error)}`);
  }
  const owner = path.join(config.processLock, "owner.json");
  fs.writeFileSync(owner, `${JSON.stringify({ pid: process.pid, token })}\n`, { flag: "wx" });
  return { owner, token };
}

function releaseProcessLock(processLock: ProcessLock): void {
  try {
    const owner = JSON.parse(fs.readFileSync(processLock.owner, "utf8"));
    if (owner.token === processLock.token) fs.rmSync(config.processLock, { recursive: true });
  } catch {
    // Preserve a lock whose ownership can no longer be proven.
  }
}

function snapshotGlobalPaths(names: string[]): Map<string, string> {
  const snapshot = new Map<string, string>();
  for (const name of names) {
    for (const targetPath of Object.values(targetPaths(name))) snapshot.set(targetPath, fingerprintNode(targetPath));
  }
  return snapshot;
}

function fingerprintNode(targetPath: string): string {
  const kind = nodeKind(targetPath);
  if (kind === "missing") return "missing";
  if (kind === "symlink") return `symlink:${fs.readlinkSync(targetPath)}`;
  if (kind === "file") {
    const stats = fs.lstatSync(targetPath);
    return `file:${stats.mode & 0o111}:${digest(fs.readFileSync(targetPath))}`;
  }
  if (kind === "directory") return `directory:${fingerprintDirectory(targetPath)}`;
  return kind;
}

function fingerprintDirectory(root: string): string {
  const parts: string[] = [];
  const names = fs.readdirSync(root).sort(compareNames);
  for (const name of names) {
    const targetPath = path.join(root, name);
    const kind = nodeKind(targetPath);
    parts.push(`${name}\0${kind}\0${fingerprintNode(targetPath)}\0`);
  }
  return digest(Buffer.from(parts.join("")));
}

function collectGlobalChanges(
  before: Map<string, string>,
  names: string[],
  beforeLockFingerprint: string | undefined,
): GlobalChanges {
  const after = snapshotGlobalPaths(names);
  const changedPaths = [...before.entries()]
    .filter(([targetPath, fingerprint]) => after.get(targetPath) !== fingerprint)
    .map(([targetPath]) => targetPath)
    .sort();
  let lockChanged = false;
  try {
    lockChanged = readCliLock().fingerprint !== beforeLockFingerprint;
  } catch {
    lockChanged = true;
  }
  return { lockChanged, paths: changedPaths };
}

function buildRepoClaims(plan: Plan): RepoClaim[] {
  const perRoot = new Map<string, Map<string, ClaimScope>>();
  const addPath = (root: string, relativePath: string, scope: ClaimScope): void => {
    let paths = perRoot.get(root);
    if (!paths) {
      paths = new Map();
      perRoot.set(root, paths);
    }
    if (paths.get(relativePath) !== "recursive") paths.set(relativePath, scope);
  };

  for (const name of plan.groups.shared) {
    addPath(config.agentsRoot, `skills/${name}`, "recursive");
    const claudeDrifted = plan.drifts.some((drift) => drift.skill === name && drift.target === "claude");
    if (claudeDrifted) addPath(config.claudeRoot, `skills/${name}`, "file");
  }
  for (const name of plan.groups.claude) addPath(config.claudeRoot, `skills/${name}`, "recursive");
  for (const name of plan.groups.codex) addPath(config.agentsRoot, `skills/${name}`, "recursive");
  for (const name of plan.groups.remove) {
    for (const [target, targetPath] of Object.entries(targetPaths(name)) as Array<[keyof TargetPaths, string]>) {
      const kind = nodeKind(targetPath);
      if (kind === "missing") continue;
      const root = target === "agents" ? config.agentsRoot : target === "claude" ? config.claudeRoot : config.codexRoot;
      addPath(root, `skills/${name}`, kind === "directory" ? "recursive" : "file");
    }
  }

  const canonicalRoot = plan.groups.shared.length > 0 || plan.groups.codex.length > 0 ? config.agentsRoot : config.claudeRoot;
  return [...perRoot.keys()].sort().map((root) => ({
    canonical: root === canonicalRoot,
    paths: [...perRoot.get(root)!.entries()]
      .sort(([left], [right]) => compareNames(left, right))
      .map(([claimPath, scope]) => ({ path: claimPath, scope })),
    root,
  }));
}

function printPlan(plan: Plan, json: boolean, filters: string[]): void {
  const driftSkills = [...new Set(plan.drifts.map((drift) => drift.skill))].sort();
  if (json) {
    console.log(
      JSON.stringify(
        {
          clean: plan.clean,
          driftSkills,
          drifts: plan.drifts,
          filters,
          groups: plan.groups,
          head: git(["rev-parse", "HEAD"]),
          repos: buildRepoClaims(plan),
          selectedCount: plan.selected.length,
          version: 2,
        },
        null,
        2,
      ),
    );
    return;
  }

  if (plan.clean) {
    console.log(
      `No publish-skills drift (${plan.selected.length} skill${plan.selected.length === 1 ? "" : "s"} checked).`,
    );
    return;
  }
  console.log(
    `Found ${plan.drifts.length} drift finding${plan.drifts.length === 1 ? "" : "s"} across ` +
      `${driftSkills.length} skill${driftSkills.length === 1 ? "" : "s"}.`,
  );
  printGroup("Remove", plan.groups.remove);
  printGroup("Add shared", plan.groups.shared);
  printGroup("Add Claude Code only", plan.groups.claude);
  printGroup("Add Codex only", plan.groups.codex);
  for (const name of driftSkills) {
    console.log(`${name}:`);
    for (const drift of plan.drifts.filter((entry) => entry.skill === name)) {
      console.log(`  ${drift.target}/${drift.kind}: ${drift.detail} (${drift.path})`);
    }
  }
}

function printGroup(label: string, names: string[]): void {
  if (names.length > 0) console.log(`${label}: ${names.join(", ")}`);
}

function printApplyResult(success: boolean, completed: string[], changes: GlobalChanges): void {
  console.log(success ? "Apply completed and verified." : "Apply stopped with partial progress.");
  console.log(`Completed commands: ${completed.length === 0 ? "none" : completed.join("; ")}`);
  console.log("Changed global paths:");
  if (changes.paths.length === 0) console.log("  none");
  for (const targetPath of changes.paths) console.log(`  ${targetPath}`);
  console.log(`CLI lock changed: ${changes.lockChanged ? config.lockFile : "no"}`);
}

function addDrift(
  drifts: Drift[],
  skill: string,
  target: DriftTarget,
  kind: string,
  targetPath: string,
  detail: string,
): void {
  drifts.push({ detail, kind, path: targetPath, skill, target });
}

function compareDrifts(left: Drift, right: Drift): number {
  return (
    left.skill.localeCompare(right.skill) ||
    left.target.localeCompare(right.target) ||
    left.kind.localeCompare(right.kind) ||
    left.detail.localeCompare(right.detail)
  );
}

function compareNames(left: string, right: string): number {
  return Buffer.compare(Buffer.from(left), Buffer.from(right));
}

function nodeKind(targetPath: string): NodeKind {
  try {
    const stats = fs.lstatSync(targetPath);
    if (stats.isSymbolicLink()) return "symlink";
    if (stats.isDirectory()) return "directory";
    if (stats.isFile()) return "file";
    return "other";
  } catch (error) {
    if (errorCode(error) === "ENOENT" || errorCode(error) === "ENOTDIR") return "missing";
    throw error;
  }
}

function isRegularFile(targetPath: string): boolean {
  try {
    return fs.statSync(targetPath).isFile();
  } catch {
    return false;
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function commandErrorMessage(error: unknown): string {
  if (isObject(error) && typeof error.stderr === "string" && error.stderr.trim()) return error.stderr.trim();
  return errorMessage(error);
}

function errorCode(error: unknown): string | undefined {
  return isObject(error) && typeof error.code === "string" ? error.code : undefined;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
