#!/usr/bin/env bun

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";

type Category = "guidance" | "manifest" | "workflow";
type Ecosystem = "cargo" | "go" | "npm" | "uv";

type Dependency = { ecosystem: Ecosystem; manifest: string; name: string; section: string; spec: string };
type Task = { file: string; name: string; runner: "just" | "package.json" };

type Repo = {
  dependencies: Dependency[];
  dirty: string[];
  ecosystems: Partial<Record<Ecosystem, number>>;
  files: Record<Category, string[]>;
  head: string | null;
  id: string;
  input: string;
  notes: string[];
  packageManagers: string[];
  root: string;
  tasks: Task[];
};

const usage = `Usage: bun run scripts/inventory.ts <repo-path> <repo-path> [more-repos...]

Print a JSON inventory of agent guidance, workflow, and manifest files per repository,
plus dependency and task gaps across the repositories.`;

const ecosystemManifests: Record<string, Ecosystem> = {
  "Cargo.toml": "cargo",
  "go.mod": "go",
  "package.json": "npm",
  "pyproject.toml": "uv",
};

const lockfilePackageManagers: Record<string, string> = {
  "Cargo.lock": "cargo",
  "Pipfile.lock": "pipenv",
  "bun.lock": "bun",
  "bun.lockb": "bun",
  "go.sum": "go",
  "package-lock.json": "npm",
  "pnpm-lock.yaml": "pnpm",
  "poetry.lock": "poetry",
  "uv.lock": "uv",
  "yarn.lock": "yarn",
};

const guidanceDirs = new Set([".agents", ".claude", ".codex", ".cursor", ".gemini", ".windsurf"]);
const guidanceDocument = /\.(json|jsonc|md|mdc|toml|ya?ml)$/;
const guidanceSupportSegments = new Set(["assets", "examples", "fixtures", "scripts", "tests"]);
const guidanceFile = basenamePattern([
  "AGENTS(\\.override)?\\.md",
  "CLAUDE(\\.local)?\\.md",
  "GEMINI\\.md",
  "CONTRIBUTING\\.md",
  "\\.cursorrules",
  "\\.windsurfrules",
  "copilot-instructions\\.md",
  "\\.mcp\\.json",
  ".+\\.(instructions|prompt)\\.md",
]);
const workflowPrefixes = [".circleci/", ".githooks/", ".github/actions/", ".github/workflows/", ".husky/"];
const workflowFile = basenamePattern([
  "\\.?[Jj]ustfile",
  ".+\\.just",
  "GNUmakefile",
  "[Mm]akefile",
  "Taskfile\\.ya?ml",
  "\\.gitlab-ci\\.ya?ml",
  "\\.?lefthook\\.ya?ml",
  "\\.pre-commit-config\\.ya?ml",
  "\\.lintstagedrc.*",
  "lint-staged\\.config\\..+",
  "\\.?mise\\.toml",
  "\\.tool-versions",
  "\\.nvmrc",
  "\\.node-version",
  "\\.python-version",
  "rust-toolchain(\\.toml)?",
  "\\.editorconfig",
  "renovate\\.json5?",
  "\\.renovaterc(\\.json)?",
  "dependabot\\.ya?ml",
  "biome\\.jsonc?",
  "\\.prettierrc.*",
  "prettier\\.config\\..+",
  "eslint\\.config\\..+",
  "\\.eslintrc.*",
  "\\.oxlintrc\\.json",
  "\\.oxfmtrc\\.jsonc?",
  "knip\\.(jsonc?|config\\..+)",
  "(vitest|jest|playwright)\\.(config|workspace)\\..+",
  "turbo\\.json",
  "nx\\.json",
  "\\.?ruff\\.toml",
  "pytest\\.ini",
  "tox\\.ini",
  "noxfile\\.py",
  "mypy\\.ini",
  "pyrightconfig\\.json",
  "\\.?rustfmt\\.toml",
  "\\.?clippy\\.toml",
  "deny\\.toml",
  "\\.golangci\\.ya?ml",
  "\\.goreleaser\\.ya?ml",
  "\\.solhint(rc|\\.json)",
  "codecov\\.ya?ml",
  "\\.markdownlint.*",
  "\\.yamllint.*",
  "\\.?cspell\\.json",
  "_?\\.?typos\\.toml",
  "lychee\\.toml",
]);
const manifestFile = basenamePattern([
  "package\\.json",
  "pnpm-workspace\\.yaml",
  "deno\\.jsonc?",
  "pyproject\\.toml",
  "requirements.*\\.txt",
  "Pipfile",
  "setup\\.(py|cfg)",
  "Cargo\\.toml",
  "go\\.(mod|work)",
  "Gemfile",
  "composer\\.json",
  "foundry\\.toml",
  "\\.gitmodules",
]);
const excludedSegments = new Set(["node_modules", "vendor"]);
const localNpmSpec = /^(file|link|portal|workspace):/;

main();

function main(): void {
  const args = process.argv.slice(2);
  if (args.includes("-h") || args.includes("--help")) {
    console.log(usage);
    return;
  }

  const errors: string[] = [];
  if (args.length < 2) errors.push("at least two repository paths are required");
  const roots = new Map<string, string>();
  const resolved: { input: string; root: string }[] = [];
  for (const input of args) {
    const root = resolveRoot(input, errors);
    if (!root) continue;
    const previous = roots.get(root);
    if (previous) {
      errors.push(`${input}: same repository as ${previous} (${root})`);
      continue;
    }
    roots.set(root, input);
    resolved.push({ input, root });
  }
  if (errors.length > 0) {
    for (const error of errors) console.error(`error: ${error}`);
    console.error(usage);
    process.exit(2);
  }

  const ids = new Set<string>();
  const repos = resolved.map(({ input, root }) => inventory(input, root, uniqueId(path.basename(root), ids)));
  console.log(JSON.stringify(report(repos), null, 2));
}

function resolveRoot(input: string, errors: string[]): string | null {
  const expanded = input === "~" || input.startsWith("~/") ? path.join(os.homedir(), input.slice(1)) : input;
  const absolute = path.resolve(expanded);
  if (!fs.existsSync(absolute) || !fs.statSync(absolute).isDirectory()) {
    errors.push(`${input}: directory not found`);
    return null;
  }
  const real = fs.realpathSync(absolute);
  let top: string;
  try {
    top = fs.realpathSync(git(real, ["rev-parse", "--show-toplevel"]).trim());
  } catch {
    errors.push(`${input}: not a Git repository`);
    return null;
  }
  if (top !== real) {
    errors.push(`${input}: not a repository root (root is ${top})`);
    return null;
  }
  return real;
}

function inventory(input: string, root: string, id: string): Repo {
  const repo: Repo = {
    dependencies: [],
    dirty: git(root, ["status", "--porcelain=v1"]).split("\n").filter(Boolean),
    ecosystems: {},
    files: { guidance: [], manifest: [], workflow: [] },
    head: tryGit(root, ["rev-parse", "--short", "HEAD"]),
    id,
    input,
    notes: [],
    packageManagers: [],
    root,
    tasks: [],
  };
  const packageManagers = new Set<string>();
  const tracked = git(root, ["ls-files", "-z"])
    .split("\0")
    .filter((file) => file && !file.split("/").some((segment) => excludedSegments.has(segment)))
    .sort();

  for (const file of tracked) {
    const base = path.posix.basename(file);
    const category = categorize(file, base);
    if (category) repo.files[category].push(file);
    const lockManager = lockfilePackageManagers[base];
    if (lockManager) packageManagers.add(lockManager);
    const ecosystem = ecosystemManifests[base];
    if (!ecosystem) continue;
    const count = repo.ecosystems[ecosystem] ?? 0;
    repo.ecosystems[ecosystem] = count;
    if (ecosystem === "cargo" || ecosystem === "go") packageManagers.add(ecosystem);
    try {
      const text = fs.readFileSync(path.join(root, file), "utf8");
      const found = parseManifest(ecosystem, file, text, repo, packageManagers);
      repo.dependencies.push(...found);
      repo.ecosystems[ecosystem] = count + found.length;
    } catch (error) {
      repo.notes.push(`${file}: not parsed (${(error as Error).message.split("\n")[0]})`);
    }
  }

  for (const file of tracked.filter((candidate) => /^\.?[Jj]ustfile$/.test(candidate))) {
    const summary = justSummary(root, file, repo.notes);
    for (const name of summary) repo.tasks.push({ file, name, runner: "just" });
  }
  repo.packageManagers = [...packageManagers].sort();
  return repo;
}

function categorize(file: string, base: string): Category | null {
  const segments = file.split("/");
  if (manifestFile.test(base)) return "manifest";
  if (guidanceFile.test(base) || isGuidanceEntry(segments, base)) return "guidance";
  if (workflowFile.test(base) || workflowPrefixes.some((prefix) => file.startsWith(prefix))) return "workflow";
  return null;
}

// Inside agent directories, list entry points only: SKILL.md per skill plus docs and configs outside support dirs.
function isGuidanceEntry(segments: string[], base: string): boolean {
  const agentDir = segments.findIndex((segment) => guidanceDirs.has(segment));
  if (agentDir < 0 || agentDir === segments.length - 1) return false;
  const skills = segments.indexOf("skills", agentDir);
  if (skills >= 0) return base === "SKILL.md" && segments.length === skills + 3;
  return guidanceDocument.test(base) && !segments.some((segment) => guidanceSupportSegments.has(segment));
}

function parseManifest(
  ecosystem: Ecosystem,
  file: string,
  text: string,
  repo: Repo,
  packageManagers: Set<string>,
): Dependency[] {
  switch (ecosystem) {
    case "npm":
      return npmDependencies(file, text, repo, packageManagers);
    case "uv":
      return uvDependencies(file, text);
    case "cargo":
      return cargoDependencies(file, text);
    case "go":
      return goDependencies(file, text);
  }
}

function npmDependencies(file: string, text: string, repo: Repo, packageManagers: Set<string>): Dependency[] {
  const manifest = JSON.parse(text) as Record<string, unknown>;
  if (typeof manifest.packageManager === "string") packageManagers.add(manifest.packageManager.replace(/@.*/, ""));
  for (const name of Object.keys(asRecord(manifest.scripts))) repo.tasks.push({ file, name, runner: "package.json" });
  const found: Dependency[] = [];
  for (const section of ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"]) {
    for (const [name, spec] of Object.entries(asRecord(manifest[section]))) {
      if (typeof spec === "string" && localNpmSpec.test(spec)) continue;
      found.push({ ecosystem: "npm", manifest: file, name, section, spec: String(spec) });
    }
  }
  return found;
}

function uvDependencies(file: string, text: string): Dependency[] {
  const doc = Bun.TOML.parse(text) as Record<string, unknown>;
  const project = asRecord(doc.project);
  const tool = asRecord(doc.tool);
  const uv = asRecord(tool.uv);
  const localSources = new Set(
    Object.entries(asRecord(uv.sources))
      .filter(([, source]) => "workspace" in asRecord(source) || "path" in asRecord(source))
      .map(([name]) => normalizePythonName(name)),
  );
  const found: Dependency[] = [];
  const addRequirements = (requirements: unknown, section: string) => {
    for (const requirement of Array.isArray(requirements) ? requirements : []) {
      if (typeof requirement !== "string") continue;
      const match = /^\s*([A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?)\s*(.*)$/.exec(requirement);
      if (!match) continue;
      const [, name = "", spec = ""] = match;
      if (localSources.has(normalizePythonName(name))) continue;
      found.push({ ecosystem: "uv", manifest: file, name, section, spec: spec.trim() });
    }
  };

  addRequirements(project.dependencies, "project.dependencies");
  for (const [extra, requirements] of Object.entries(asRecord(project["optional-dependencies"]))) {
    addRequirements(requirements, `project.optional-dependencies.${extra}`);
  }
  for (const [group, requirements] of Object.entries(asRecord(doc["dependency-groups"]))) {
    addRequirements(requirements, `dependency-groups.${group}`);
  }
  addRequirements(uv["dev-dependencies"], "tool.uv.dev-dependencies");
  return found;
}

function cargoDependencies(file: string, text: string): Dependency[] {
  const doc = Bun.TOML.parse(text) as Record<string, unknown>;
  const found: Dependency[] = [];
  const add = (table: unknown, section: string) => {
    for (const [key, value] of Object.entries(asRecord(table))) {
      const detail = asRecord(value);
      // Workspace inheritance is counted at the workspace root; path-only crates are repository-internal.
      if (detail.workspace === true || ("path" in detail && !("version" in detail))) continue;
      const name = typeof detail.package === "string" ? detail.package : key;
      const spec =
        typeof value === "string"
          ? value
          : String(detail.version ?? (typeof detail.git === "string" ? `git:${detail.git}` : ""));
      found.push({ ecosystem: "cargo", manifest: file, name, section, spec });
    }
  };
  const kinds = ["dependencies", "dev-dependencies", "build-dependencies"];
  for (const kind of kinds) add(doc[kind], kind);
  add(asRecord(doc.workspace).dependencies, "workspace.dependencies");
  for (const [target, table] of Object.entries(asRecord(doc.target))) {
    for (const kind of kinds) add(asRecord(table)[kind], `target.${target}.${kind}`);
  }
  return found;
}

function goDependencies(file: string, text: string): Dependency[] {
  const found: Dependency[] = [];
  let block: string | null = null;
  const add = (section: string, entry: string) => {
    if (/\/\/\s*indirect\b/.test(entry)) return;
    const [name, version = ""] = entry
      .replace(/\/\/.*$/, "")
      .trim()
      .split(/\s+/);
    if (name) found.push({ ecosystem: "go", manifest: file, name, section, spec: section === "tool" ? "" : version });
  };
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (block) {
      if (line === ")") block = null;
      else if (line) add(block, line);
      continue;
    }
    const opener = /^(require|tool)\s*\($/.exec(line);
    if (opener) {
      block = opener[1] ?? null;
      continue;
    }
    const single = /^(require|tool)\s+(.+)$/.exec(line);
    if (single) add(single[1] ?? "", single[2] ?? "");
  }
  return found;
}

function justSummary(root: string, file: string, notes: string[]): string[] {
  try {
    const output = execFileSync("just", ["--justfile", file, "--working-directory", ".", "--summary"], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 10_000,
    });
    return output.trim().split(/\s+/).filter(Boolean);
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code;
    notes.push(`${file}: recipes not listed (${code === "ENOENT" ? "just not installed" : "just --summary failed"})`);
    return [];
  }
}

function report(repos: Repo[]) {
  const dependencyGaps = [];
  const summary: Record<string, { gaps: number; repos: string[]; shared: number }> = {};
  const byDependency = new Map<string, { ecosystem: Ecosystem; name: string; uses: Map<string, Dependency[]> }>();
  for (const repo of repos) {
    for (const dependency of repo.dependencies) {
      const name = normalizeName(dependency.ecosystem, dependency.name);
      const key = `${dependency.ecosystem}\0${name}`;
      const entry = byDependency.get(key) ?? { ecosystem: dependency.ecosystem, name, uses: new Map() };
      entry.uses.set(repo.id, [...(entry.uses.get(repo.id) ?? []), dependency]);
      byDependency.set(key, entry);
    }
  }
  for (const ecosystem of ["cargo", "go", "npm", "uv"] as const) {
    const eligible = repos.filter((repo) => ecosystem in repo.ecosystems).map((repo) => repo.id);
    if (eligible.length >= 2) summary[ecosystem] = { gaps: 0, repos: eligible, shared: 0 };
  }
  for (const entry of [...byDependency.values()].sort(compareEntries)) {
    const counts = summary[entry.ecosystem];
    if (!counts) continue;
    const missingIn = counts.repos.filter((id) => !entry.uses.has(id));
    if (missingIn.length === 0) {
      counts.shared += 1;
      continue;
    }
    counts.gaps += 1;
    const presentIn = [...entry.uses].flatMap(([repo, uses]) => {
      const [first] = uses.sort((a, b) => a.manifest.localeCompare(b.manifest));
      return first
        ? [{ manifest: first.manifest, repo, section: first.section, spec: first.spec, uses: uses.length }]
        : [];
    });
    dependencyGaps.push({ ecosystem: entry.ecosystem, missingIn, name: entry.name, presentIn });
  }

  const taskRepos = new Map<string, Set<string>>();
  for (const repo of repos) {
    for (const task of repo.tasks) taskRepos.set(task.name, (taskRepos.get(task.name) ?? new Set()).add(repo.id));
  }
  const allIds = repos.map((repo) => repo.id);
  const taskGaps = [...taskRepos]
    .filter(([, present]) => present.size < repos.length)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, present]) => ({
      missingIn: allIds.filter((id) => !present.has(id)),
      name,
      presentIn: allIds.filter((id) => present.has(id)),
    }));

  return {
    dependencyGaps,
    repos: repos.map(({ dependencies: _dependencies, ...repo }) => repo),
    schemaVersion: 1,
    summary: { dependencies: summary, tasks: { gaps: taskGaps.length, shared: taskRepos.size - taskGaps.length } },
    taskGaps,
  };
}

function compareEntries(a: { ecosystem: string; name: string }, b: { ecosystem: string; name: string }): number {
  return a.ecosystem.localeCompare(b.ecosystem) || a.name.localeCompare(b.name);
}

function normalizeName(ecosystem: Ecosystem, name: string): string {
  if (ecosystem === "uv") return normalizePythonName(name);
  if (ecosystem === "cargo") return name.toLowerCase().replaceAll("_", "-");
  return name;
}

function normalizePythonName(name: string): string {
  return name.toLowerCase().replace(/[-_.]+/g, "-");
}

function uniqueId(base: string, ids: Set<string>): string {
  let id = base;
  for (let suffix = 2; ids.has(id); suffix += 1) id = `${base}-${suffix}`;
  ids.add(id);
  return id;
}

function basenamePattern(alternatives: string[]): RegExp {
  return new RegExp(`^(${alternatives.join("|")})$`);
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function git(cwd: string, args: string[]): string {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function tryGit(cwd: string, args: string[]): string | null {
  try {
    return git(cwd, args).trim();
  } catch {
    return null;
  }
}
