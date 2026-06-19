#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const ignoredDirs = new Set([
  ".git",
  ".next",
  ".venv",
  "build",
  "coverage",
  "dist",
  "node_modules",
  "out",
  "target",
  "vendor",
]);

const options = parseArgs(process.argv.slice(2));
if (options.help) {
  printUsage();
  process.exit(0);
}

const cwd = path.resolve(options.cwd);
const argErrors = [];
if (options.version && !isSemver(options.version)) {
  argErrors.push(`invalid --version: ${options.version}`);
}

let repoRoot;
try {
  repoRoot = git(["rev-parse", "--show-toplevel"], cwd).trim();
} catch {
  console.error(`ERROR: not inside a git repository: ${cwd}`);
  process.exit(2);
}

const rootPackagePath = path.join(repoRoot, "package.json");
if (!fs.existsSync(rootPackagePath)) {
  console.error(`ERROR: no package.json found at repo root: ${repoRoot}`);
  process.exit(2);
}

const rootPackage = readJson(rootPackagePath, argErrors);
const hasPnpmWorkspace = fs.existsSync(path.join(repoRoot, "pnpm-workspace.yaml"));
const workspacePatterns = workspaceGlobs(repoRoot, rootPackage);
const isMonorepo = workspacePatterns.length > 0 || hasPnpmWorkspace;
const packageRecords = isMonorepo
  ? discoverWorkspacePackages(repoRoot, workspacePatterns, argErrors, { preferPnpm: hasPnpmWorkspace })
  : [readPackage(repoRoot, repoRoot, argErrors)].filter(Boolean);
const releaseTags = gitTags(repoRoot);
const selectedTargets = resolveTargets({
  cwd,
  isMonorepo,
  packages: packageRecords,
  repoRoot,
  selectors: options.packages,
  errors: argErrors,
});

if (options.version && selectedTargets.length > 1) {
  argErrors.push("explicit --version is only valid for a single target package");
}

const output = {
  cwd,
  repoRoot,
  mode: isMonorepo ? "monorepo" : "single-package",
  beta: options.beta,
  dryRun: options.dryRun,
  explicitVersion: options.version ?? null,
  needsSelection: isMonorepo && options.packages.length === 0 && selectedTargets.length === 0,
  errors: argErrors,
  workingTree: readWorkingTree(repoRoot),
  workspacePatterns,
  packages: packageRecords.map(serializePackage),
  targets: selectedTargets.map(serializePackage),
  dependencyEdges: dependencyEdges(packageRecords),
  previousTags: Object.fromEntries(packageRecords.map((pkg) => [pkg.id, previousTag(pkg, isMonorepo, releaseTags)])),
  changedFiles: {},
  includedFiles: {},
  excludedFiles: {},
};

for (const pkg of packageRecords) {
  const tag = output.previousTags[pkg.id]?.tag ?? null;
  const changed = changedFiles(repoRoot, pkg, tag);
  const classified = classifyFiles(pkg, changed);
  output.changedFiles[pkg.id] = changed;
  output.includedFiles[pkg.id] = classified.included;
  output.excludedFiles[pkg.id] = classified.excluded;
}

console.log(`${JSON.stringify(output, null, 2)}\n`);
process.exit(argErrors.length > 0 ? 64 : 0);

function parseArgs(args) {
  const parsed = {
    beta: false,
    cwd: process.cwd(),
    dryRun: false,
    help: false,
    packages: [],
    version: null,
  };

  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--help" || arg === "-h") {
      parsed.help = true;
    } else if (arg === "--beta") {
      parsed.beta = true;
    } else if (arg === "--dry-run") {
      parsed.dryRun = true;
    } else if (arg === "--cwd") {
      parsed.cwd = requireValue(args, (i += 1), "--cwd");
    } else if (arg.startsWith("--cwd=")) {
      parsed.cwd = arg.slice("--cwd=".length);
    } else if (arg === "--version") {
      parsed.version = requireValue(args, (i += 1), "--version");
    } else if (arg.startsWith("--version=")) {
      parsed.version = arg.slice("--version=".length);
    } else if (arg === "--package") {
      parsed.packages.push(requireValue(args, (i += 1), "--package"));
    } else if (arg.startsWith("--package=")) {
      parsed.packages.push(arg.slice("--package=".length));
    } else if (arg.startsWith("-")) {
      throwUsage(`unknown option: ${arg}`);
    } else if (!parsed.version && isSemver(arg)) {
      parsed.version = arg;
    } else {
      parsed.packages.push(arg);
    }
  }

  return parsed;
}

function requireValue(args, index, flag) {
  const value = args[index];
  if (!value || value.startsWith("--")) throwUsage(`${flag} requires a value`);
  return value;
}

function throwUsage(message) {
  console.error(`ERROR: ${message}\n`);
  printUsage();
  process.exit(64);
}

function printUsage() {
  console.error(`Usage: plan-release.mjs [--cwd <repo>] [--beta] [--dry-run] [--version <semver>] [--package <name-or-dir>]...`);
}

function git(args, cwdArg) {
  return execFileSync("git", args, { cwd: cwdArg, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
}

function readJson(filePath, errors) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    errors.push(`${path.relative(process.cwd(), filePath)}: ${error.message}`);
    return {};
  }
}

function workspaceGlobs(root, rootPackageJson) {
  const globs = [];
  const workspaces = rootPackageJson.workspaces;
  if (Array.isArray(workspaces)) {
    globs.push(...workspaces);
  } else if (workspaces && Array.isArray(workspaces.packages)) {
    globs.push(...workspaces.packages);
  }

  const pnpmWorkspace = path.join(root, "pnpm-workspace.yaml");
  if (fs.existsSync(pnpmWorkspace)) {
    globs.push(...readPnpmWorkspaceGlobs(pnpmWorkspace));
  }

  return unique(globs.filter((glob) => typeof glob === "string" && glob));
}

function readPnpmWorkspaceGlobs(filePath) {
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
  const globs = [];
  let inPackages = false;

  for (const line of lines) {
    if (/^packages:\s*$/.test(line)) {
      inPackages = true;
      continue;
    }
    if (inPackages && /^\S/.test(line)) break;
    const match = inPackages && line.match(/^\s*-\s*['"]?([^'"]+)['"]?\s*$/);
    if (match) globs.push(match[1]);
  }

  return globs;
}

function discoverWorkspacePackages(root, globs, errors, { preferPnpm = false } = {}) {
  const pnpmPackages = preferPnpm ? discoverPnpmWorkspacePackages(root, errors) : null;
  if (pnpmPackages?.length) return pnpmPackages;

  const packageDirs = findPackageDirs(root)
    .filter((dir) => dir !== root)
    .filter((dir) => {
      if (globs.length === 0) return true;
      const rel = slash(path.relative(root, dir));
      return matchesWorkspaceGlobs(rel, globs);
    })
    .sort((a, b) => slash(path.relative(root, a)).localeCompare(slash(path.relative(root, b))));

  return packageDirs.map((dir) => readPackage(root, dir, errors)).filter(Boolean);
}

function discoverPnpmWorkspacePackages(root, errors) {
  try {
    const projects = JSON.parse(
      execFileSync("pnpm", ["--dir", root, "list", "-r", "--depth", "-1", "--json"], {
        cwd: root,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      }),
    );
    if (!Array.isArray(projects)) return null;

    return projects
      .map((project) => workspaceProjectDir(root, project.path))
      .filter((dir) => dir && dir !== root)
      .map((dir) => readPackage(root, dir, errors))
      .filter(Boolean)
      .sort((a, b) => a.dir.localeCompare(b.dir));
  } catch {
    return null;
  }
}

function workspaceProjectDir(root, projectPath) {
  if (!projectPath) return null;

  const rootReal = safeRealpath(root);
  const projectReal = safeRealpath(projectPath);
  const rel = slash(path.relative(rootReal, projectReal));
  if (rel === "" || rel.startsWith("../") || rel === "..") return null;
  return path.join(root, rel);
}

function safeRealpath(value) {
  try {
    return fs.realpathSync.native(value);
  } catch {
    return path.resolve(value);
  }
}

function findPackageDirs(root) {
  const results = [];
  walk(root);
  return results;

  function walk(dir) {
    const base = path.basename(dir);
    if (ignoredDirs.has(base)) return;
    const packagePath = path.join(dir, "package.json");
    if (fs.existsSync(packagePath)) results.push(dir);

    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      if (ignoredDirs.has(entry.name)) continue;
      walk(path.join(dir, entry.name));
    }
  }
}

function readPackage(root, dir, errors) {
  const packagePath = path.join(dir, "package.json");
  if (!fs.existsSync(packagePath)) return null;
  const manifest = readJson(packagePath, errors);
  const rel = slash(path.relative(root, dir)) || ".";
  const id = rel === "." ? manifest.name || "." : rel;
  return {
    id,
    dir: rel,
    absDir: dir,
    name: manifest.name ?? null,
    version: manifest.version ?? null,
    files: Array.isArray(manifest.files) ? manifest.files : null,
    dependencies: manifest.dependencies ?? {},
    peerDependencies: manifest.peerDependencies ?? {},
  };
}

function resolveTargets({ cwd: cwdArg, errors, isMonorepo, packages, repoRoot: root, selectors }) {
  if (!isMonorepo) return packages;
  if (selectors.length > 0) {
    const selected = [];
    for (const selector of selectors) {
      const matches = packages.filter((pkg) => packageMatches(pkg, selector));
      if (matches.length === 0) {
        errors.push(`unknown package selector: ${selector}`);
      } else if (matches.length > 1) {
        errors.push(`ambiguous package selector: ${selector}`);
      } else if (!selected.some((pkg) => pkg.id === matches[0].id)) {
        selected.push(matches[0]);
      }
    }
    return selected;
  }

  const matches = packages.filter((pkg) => {
    const rel = pkg.dir === "." ? "" : pkg.dir;
    const abs = path.join(root, rel);
    return cwdArg === abs || cwdArg.startsWith(`${abs}${path.sep}`);
  });
  return matches.length === 1 ? matches : [];
}

function packageMatches(pkg, selector) {
  const normalized = selector.replace(/\/$/, "");
  const base = path.basename(pkg.dir);
  const unscoped = pkg.name?.startsWith("@") ? pkg.name.split("/").at(-1) : pkg.name;
  return normalized === pkg.dir || normalized === base || normalized === pkg.name || normalized === unscoped;
}

function serializePackage(pkg) {
  return {
    id: pkg.id,
    dir: pkg.dir,
    name: pkg.name,
    version: pkg.version,
    files: pkg.files,
    dependencyNames: Object.keys(pkg.dependencies).sort(),
    peerDependencyNames: Object.keys(pkg.peerDependencies).sort(),
  };
}

function dependencyEdges(packages) {
  const byName = new Map(packages.filter((pkg) => pkg.name).map((pkg) => [pkg.name, pkg]));
  const edges = [];

  for (const from of packages) {
    for (const [type, deps] of [
      ["dependencies", from.dependencies],
      ["peerDependencies", from.peerDependencies],
    ]) {
      for (const [name, range] of Object.entries(deps)) {
        const to = byName.get(name);
        if (to) edges.push({ from: from.id, to: to.id, type, name, range });
      }
    }
  }

  return edges.sort((a, b) => `${a.from}:${a.to}:${a.type}`.localeCompare(`${b.from}:${b.to}:${b.type}`));
}

function previousTag(pkg, isMonorepoArg, allTags) {
  const candidates = tagPatterns(pkg, isMonorepoArg);
  const parsed = candidateTags(pkg, isMonorepoArg, allTags)
    .map((tag) => ({ tag, version: versionFromTag(tag, isMonorepoArg) }))
    .filter((entry) => entry.version)
    .sort((a, b) => compareSemverDesc(a.version, b.version));

  return {
    tag: parsed[0]?.tag ?? null,
    version: parsed[0]?.version ?? null,
    patterns: candidates,
  };
}

function tagPatterns(pkg, isMonorepoArg) {
  if (!isMonorepoArg) return ["v[0-9]*.[0-9]*.[0-9]*", "[0-9]*.[0-9]*.[0-9]*"];

  return packageTagNames(pkg).flatMap((name) => [`${name}@[0-9]*.[0-9]*.[0-9]*`, `${name}@v[0-9]*.[0-9]*.[0-9]*`]);
}

function gitTags(root) {
  try {
    return git(["tag", "--list", "--sort=-creatordate"], root)
      .split(/\r?\n/)
      .filter(Boolean);
  } catch {
    return [];
  }
}

function candidateTags(pkg, isMonorepoArg, allTags) {
  if (!isMonorepoArg) {
    return unique([
      ...allTags.filter((tag) => tag.startsWith("v") && stripLeadingV(tag)),
      ...allTags.filter((tag) => !tag.startsWith("v") && stripLeadingV(tag)),
    ]);
  }

  const matches = [];
  for (const name of packageTagNames(pkg)) {
    for (const hasLeadingV of [false, true]) {
      matches.push(
        ...allTags.filter((tag) => {
          const parsed = parsePackageTag(tag);
          return parsed?.name === name && parsed.hasLeadingV === hasLeadingV;
        }),
      );
    }
  }
  return unique(matches);
}

function packageTagNames(pkg) {
  const names = new Set([pkg.dir, path.basename(pkg.dir)]);
  if (pkg.name) {
    names.add(pkg.name);
    if (pkg.name.startsWith("@")) names.add(pkg.name.split("/").at(-1));
  }
  return [...names].filter(Boolean);
}

function parsePackageTag(tag) {
  const at = tag.lastIndexOf("@");
  if (at <= 0) return null;

  const name = tag.slice(0, at);
  const suffix = tag.slice(at + 1);
  const hasLeadingV = suffix.startsWith("v");
  const version = stripLeadingV(suffix);
  return version ? { hasLeadingV, name, version } : null;
}

function versionFromTag(tag, isMonorepoArg) {
  if (!isMonorepoArg) return stripLeadingV(tag);
  return parsePackageTag(tag)?.version ?? null;
}

function changedFiles(root, pkg, tag) {
  const args = tag ? ["diff", "--name-only", `${tag}..HEAD`, "--"] : ["ls-files", "--"];
  if (pkg.dir !== ".") args.push(pkg.dir);
  return git(args, root)
    .split(/\r?\n/)
    .filter(Boolean)
    .sort();
}

function classifyFiles(pkg, files) {
  const included = [];
  const excluded = [];

  for (const file of files) {
    const packageRel = pkg.dir === "." ? file : slash(path.relative(pkg.dir, file));
    const filesMatch = pkg.files ? matchesFilesField(packageRel, pkg.files) || packageRel === "package.json" : true;
    const production = pkg.files ? filesMatch : isProductionFile(file, packageRel);
    const entry = { path: file, reason: production ? "included" : exclusionReason(file, packageRel) };
    (production ? included : excluded).push(entry);
  }

  return { included, excluded };
}

function matchesFilesField(packageRel, filesField) {
  const rel = packageRel.replace(/^\.\//, "");
  return filesField.some((entry) => {
    const normalized = normalizeGlob(entry).replace(/\/$/, "");
    if (!hasGlob(normalized)) return rel === normalized || rel.startsWith(`${normalized}/`);
    return matchGlob(rel, normalized);
  });
}

function isProductionFile(repoRel, packageRel) {
  const rel = slash(repoRel);
  const local = slash(packageRel);
  const base = path.basename(local);

  if (/^(\.github|\.gitlab|\.circleci|\.husky)\//.test(rel)) return false;
  if (/(^|\/)(__tests__|tests?|fixtures?|mocks?)\//i.test(local)) return false;
  if (/\.(test|spec|bench|fixture)\.[cm]?[jt]sx?$/i.test(base)) return false;
  if (/^(eslint|prettier|biome|vitest|jest|commitlint|lint-staged|lefthook|husky)\.config\./.test(base)) return false;
  if (/^(\.eslintrc|\.prettierrc|\.lintstagedrc|\.npmrc|\.node-version|\.nvmrc)$/.test(base)) return false;
  if (/^(justfile|Makefile|Dockerfile)$/.test(base)) return false;
  if (/^(pnpm-lock\.yaml|bun\.lockb?|package-lock\.json|yarn\.lock)$/.test(base)) return false;
  return true;
}

function exclusionReason(repoRel, packageRel) {
  const rel = slash(repoRel);
  const local = slash(packageRel);
  const base = path.basename(local);
  if (/^(\.github|\.gitlab|\.circleci|\.husky)\//.test(rel)) return "ci";
  if (/(^|\/)(__tests__|tests?|fixtures?|mocks?)\//i.test(local) || /\.(test|spec|bench|fixture)\.[cm]?[jt]sx?$/i.test(base)) {
    return "test";
  }
  return "tooling";
}

function readWorkingTree(root) {
  const status = git(["status", "--porcelain=v1"], root)
    .split(/\r?\n/)
    .filter(Boolean);
  return { clean: status.length === 0, status };
}

function matchesWorkspaceGlobs(rel, globs) {
  const positive = [];
  const negative = [];

  for (const glob of globs) {
    const negated = glob.startsWith("!");
    const normalized = normalizeGlob(negated ? glob.slice(1) : glob).replace(/\/$/, "");
    if (!normalized) continue;
    (negated ? negative : positive).push(normalized);
  }

  const included = positive.length === 0 || positive.some((glob) => matchWorkspaceGlob(rel, glob));
  return included && !negative.some((glob) => matchWorkspaceGlob(rel, glob));
}

function matchWorkspaceGlob(rel, glob) {
  if (!hasGlob(glob)) return rel === glob || rel.startsWith(`${glob}/`);
  return matchGlob(rel, glob);
}

function normalizeGlob(glob) {
  return slash(glob).replace(/^\.\//, "").replace(/\/package\.json$/, "");
}

function matchGlob(value, glob) {
  return globToRegex(glob).test(value);
}

function globToRegex(glob) {
  let source = "^";
  for (let i = 0; i < glob.length; i += 1) {
    const char = glob[i];
    const next = glob[i + 1];
    if (char === "*" && next === "*") {
      if (glob[i + 2] === "/") {
        source += "(?:.*/)?";
        i += 2;
      } else {
        source += ".*";
        i += 1;
      }
    } else if (char === "*") {
      source += "[^/]*";
    } else if (char === "?") {
      source += "[^/]";
    } else {
      source += escapeRegex(char);
    }
  }
  source += "$";
  return new RegExp(source);
}

function hasGlob(value) {
  return /[*?[\]{}]/.test(value);
}

function escapeRegex(value) {
  return value.replace(/[|\\{}()[\]^$+*?.]/g, "\\$&");
}

function slash(value) {
  return value.split(path.sep).join("/");
}

function unique(values) {
  return [...new Set(values)];
}

function isSemver(value) {
  return /^\d+\.\d+\.\d+(?:-[0-9A-Za-z]+(?:\.[0-9A-Za-z]+)*)?(?:\+[0-9A-Za-z]+(?:\.[0-9A-Za-z]+)*)?$/.test(value);
}

function stripLeadingV(value) {
  const version = value.startsWith("v") ? value.slice(1) : value;
  return isSemver(version) ? version : null;
}

function compareSemverDesc(a, b) {
  return -compareSemver(a, b);
}

function compareSemver(a, b) {
  const left = parseSemver(a);
  const right = parseSemver(b);
  for (const key of ["major", "minor", "patch"]) {
    if (left[key] !== right[key]) return left[key] - right[key];
  }
  if (left.pre.length === 0 && right.pre.length === 0) return 0;
  if (left.pre.length === 0) return 1;
  if (right.pre.length === 0) return -1;
  const length = Math.max(left.pre.length, right.pre.length);
  for (let i = 0; i < length; i += 1) {
    const l = left.pre[i];
    const r = right.pre[i];
    if (l === undefined) return -1;
    if (r === undefined) return 1;
    if (l === r) continue;
    const lNum = /^\d+$/.test(l);
    const rNum = /^\d+$/.test(r);
    if (lNum && rNum) return Number(l) - Number(r);
    if (lNum) return -1;
    if (rNum) return 1;
    return l.localeCompare(r);
  }
  return 0;
}

function parseSemver(value) {
  const [withoutBuild] = value.split("+");
  const [core, pre = ""] = withoutBuild.split("-");
  const [major, minor, patch] = core.split(".").map(Number);
  return { major, minor, patch, pre: pre ? pre.split(".") : [] };
}
