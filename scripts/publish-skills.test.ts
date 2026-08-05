import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, test } from "bun:test";
import { fileURLToPath } from "node:url";

const repository = "PaulRBerg/agent-skills";
const sourceUrl = "https://github.com/PaulRBerg/agent-skills.git";
const scriptPath = path.join(
  path.resolve(path.dirname(fileURLToPath(import.meta.url)), ".."),
  "scripts",
  "publish-skills.ts",
);

type Fixture = {
  agentsRoot: string;
  claudeRoot: string;
  codexRoot: string;
  commandLog: string;
  env: NodeJS.ProcessEnv;
  fakeBunx: string;
  head: string;
  lockFile: string;
  processLock: string;
  remoteRoot: string;
  root: string;
  sourceRoot: string;
};

const temporaryRoots: string[] = [];

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) fs.rmSync(root, { force: true, recursive: true });
});

test("plan and check cover clean shared and restricted installs, content, modes, and filters", () => {
  const fixture = createFixture();
  const clean = runJson(fixture, "plan", "--json");
  assert.equal(clean.result.status, 0, clean.result.stderr);
  assert.equal(clean.json.clean, true);
  assert.deepEqual(clean.json.selected, ["alpha", "beta", "gamma"]);
  assert.equal(run(fixture, "check").status, 0);

  fs.appendFileSync(path.join(fixture.agentsRoot, "skills", "alpha", "SKILL.md"), "drift\n");
  fs.chmodSync(path.join(fixture.agentsRoot, "skills", "gamma", "scripts", "run.sh"), 0o644);
  fs.appendFileSync(path.join(fixture.claudeRoot, "skills", "beta", "SKILL.md"), "drift\n");

  const alpha = runJson(fixture, "plan", "--json", "--skill", "alpha");
  assert.deepEqual(alpha.json.selected, ["alpha"]);
  assert(alpha.json.drifts.some((drift) => drift.skill === "alpha" && drift.kind === "content"));
  assert(!alpha.json.drifts.some((drift) => drift.skill === "beta"));

  const gamma = runJson(fixture, "plan", "--json", "--skill", "gamma");
  assert(gamma.json.drifts.some((drift) => drift.kind === "mode"));
  assert.equal(run(fixture, "check", "--skill", "alpha").status, 1);
});

test("planner detects target layout, symlink, deletion, and stale lock metadata", () => {
  const fixture = createFixture();
  const alphaLink = path.join(fixture.claudeRoot, "skills", "alpha");
  fs.rmSync(alphaLink);
  fs.symlinkSync("../wrong", alphaLink);
  fs.cpSync(path.join(fixture.sourceRoot, "skills", "beta"), path.join(fixture.agentsRoot, "skills", "beta"), {
    recursive: true,
  });

  const lock = readLock(fixture);
  lock.skills.gone = lockEntry(fixture, "alpha", { skillPath: "skills/gone/SKILL.md" });
  lock.skills.gamma = {
    ...lock.skills.gamma,
    skillFolderHash: "0000000000000000000000000000000000000000",
    skillPath: "old/gamma/SKILL.md",
    source: "someone/else",
    sourceUrl: "https://example.com/else.git",
  };
  writeLock(fixture, lock);
  fs.cpSync(path.join(fixture.sourceRoot, "skills", "alpha"), path.join(fixture.agentsRoot, "skills", "gone"), {
    recursive: true,
  });

  const { json, result } = runJson(fixture, "plan", "--json");
  assert.equal(result.status, 0, result.stderr);
  assert(hasDrift(json, "alpha", "symlink"));
  assert(hasDrift(json, "beta", "forbidden-copy"));
  assert(hasDrift(json, "gone", "deleted"));
  assert(hasDrift(json, "gone", "deleted-install"));
  assert(hasDrift(json, "gamma", "lock-source"));
  assert(hasDrift(json, "gamma", "lock-source-url"));
  assert(hasDrift(json, "gamma", "lock-skill-path"));
  assert(hasDrift(json, "gamma", "lock-skill-folder-hash"));
  assert(json.groups.remove.includes("beta"));
  assert(json.groups.remove.includes("gone"));
});

test("planner rejects invalid source and CLI lock metadata", () => {
  const invalidTarget = createFixture();
  const skillFile = path.join(invalidTarget.sourceRoot, "skills", "beta", "SKILL.md");
  fs.writeFileSync(skillFile, skillMarkdown("beta", "invalid-target"));
  const targetResult = run(invalidTarget, "plan");
  assert.equal(targetResult.status, 2);
  assert.match(targetResult.stderr, /invalid metadata\.install-targets/);

  const malformed = createFixture();
  fs.writeFileSync(malformed.lockFile, "{not-json\n");
  const malformedResult = run(malformed, "plan");
  assert.equal(malformedResult.status, 2);
  assert.match(malformedResult.stderr, /Malformed skills CLI lock/);

  const wrongVersion = createFixture();
  writeLock(wrongVersion, { skills: {}, version: 2 });
  const versionResult = run(wrongVersion, "plan");
  assert.equal(versionResult.status, 2);
  assert.match(versionResult.stderr, /expected skills CLI lock version 3/);
});

test("apply batches one remove and one add per target group, then verifies clean state", () => {
  const fixture = createFixture();
  driftInstalledSkill(fixture, "alpha", "agents");
  driftInstalledSkill(fixture, "beta", "claude");
  driftInstalledSkill(fixture, "gamma", "agents");
  const lock = readLock(fixture);
  lock.skills.gone = lockEntry(fixture, "alpha", { skillPath: "skills/gone/SKILL.md" });
  writeLock(fixture, lock);

  const result = run(fixture, "apply", "--expected-head", fixture.head);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Apply completed and verified/);
  const commands = readCommandLog(fixture);
  assert.equal(commands.length, 4);
  assert.deepEqual(commands.map(commandKind), ["remove", "shared", "claude", "codex"]);
  assert.equal(run(fixture, "check").status, 0);
  assert.equal(readLock(fixture).skills.gone, undefined);
});

test("apply guards reject dirty sources, HEAD and upstream mismatches, malformed locks, and concurrent runs", () => {
  const dirty = createFixture();
  fs.appendFileSync(path.join(dirty.sourceRoot, "skills", "alpha", "SKILL.md"), "dirty\n");
  const dirtyResult = run(dirty, "apply", "--expected-head", dirty.head, "--skill", "alpha");
  assert.equal(dirtyResult.status, 1);
  assert.match(dirtyResult.stderr, /selected source paths are dirty/);

  const headMismatch = createFixture();
  const oldHead = headMismatch.head;
  commitFixtureFile(headMismatch, "README.md", "new head\n", true);
  const headResult = run(headMismatch, "apply", "--expected-head", oldHead, "--skill", "alpha");
  assert.equal(headResult.status, 1);
  assert.match(headResult.stderr, /HEAD is .* expected/);

  const upstreamMismatch = createFixture();
  commitFixtureFile(upstreamMismatch, "README.md", "ahead\n", false);
  const upstreamResult = run(
    upstreamMismatch,
    "apply",
    "--expected-head",
    git(upstreamMismatch.sourceRoot, "rev-parse", "HEAD"),
    "--skill",
    "alpha",
  );
  assert.equal(upstreamResult.status, 1);
  assert.match(upstreamResult.stderr, /does not equal upstream/);

  const malformed = createFixture();
  fs.writeFileSync(malformed.lockFile, "{");
  const malformedResult = run(malformed, "apply", "--expected-head", malformed.head);
  assert.equal(malformedResult.status, 1);
  assert.match(malformedResult.stderr, /Malformed skills CLI lock/);

  const concurrent = createFixture();
  fs.mkdirSync(concurrent.processLock);
  const concurrentResult = run(concurrent, "apply", "--expected-head", concurrent.head);
  assert.equal(concurrentResult.status, 2);
  assert.match(concurrentResult.stderr, /another publish-skills apply holds/);
  assert.equal(readCommandLog(concurrent).length, 0);
});

test("partial apply failures report completed groups and exact changed global paths", () => {
  const fixture = createFixture();
  driftInstalledSkill(fixture, "alpha", "agents");
  driftInstalledSkill(fixture, "beta", "claude");
  fixture.env.FAKE_SKILLS_FAIL_MATCH = "--skill beta";

  const result = run(fixture, "apply", "--expected-head", fixture.head, "--skill", "alpha", "--skill", "beta");
  assert.equal(result.status, 1);
  assert.match(result.stdout, /Apply stopped with partial progress/);
  assert.match(result.stdout, /shared: alpha/);
  assert.match(result.stdout, new RegExp(escapeRegExp(path.join(fixture.agentsRoot, "skills", "alpha"))));
  const plan = runJson(fixture, "plan", "--json", "--skill", "alpha", "--skill", "beta").json;
  assert(!plan.driftSkills.includes("alpha"));
  assert(plan.driftSkills.includes("beta"));
});

function createFixture(): Fixture {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "publish-skills-test-"));
  temporaryRoots.push(root);
  const fixture: Fixture = {
    agentsRoot: path.join(root, ".agents"),
    claudeRoot: path.join(root, ".claude"),
    codexRoot: path.join(root, ".codex"),
    commandLog: path.join(root, "commands.jsonl"),
    env: {},
    fakeBunx: "",
    head: "",
    lockFile: path.join(root, "state", ".skill-lock.json"),
    processLock: path.join(root, "state", "publish.lock"),
    remoteRoot: path.join(root, "remote.git"),
    root,
    sourceRoot: path.join(root, "source"),
  };
  for (const directory of [
    fixture.agentsRoot,
    fixture.claudeRoot,
    fixture.codexRoot,
    path.dirname(fixture.lockFile),
    fixture.sourceRoot,
  ]) {
    fs.mkdirSync(directory, { recursive: true });
  }

  writeSkill(fixture.sourceRoot, "alpha", "shared");
  writeSkill(fixture.sourceRoot, "beta", "claude-code");
  writeSkill(fixture.sourceRoot, "gamma", "codex");
  git(fixture.sourceRoot, "init", "-b", "main");
  git(fixture.sourceRoot, "config", "user.email", "tests@example.com");
  git(fixture.sourceRoot, "config", "user.name", "Tests");
  git(fixture.sourceRoot, "add", "skills");
  git(fixture.sourceRoot, "commit", "-m", "fixture");
  git(root, "init", "--bare", fixture.remoteRoot);
  git(fixture.sourceRoot, "remote", "add", "origin", fixture.remoteRoot);
  git(fixture.sourceRoot, "push", "-u", "origin", "main");
  fixture.head = git(fixture.sourceRoot, "rev-parse", "HEAD");

  installFixtureSkill(fixture, "alpha", "shared");
  installFixtureSkill(fixture, "beta", "claude-code");
  installFixtureSkill(fixture, "gamma", "codex");
  writeLock(fixture, {
    dismissed: {},
    skills: {
      alpha: lockEntry(fixture, "alpha"),
      beta: lockEntry(fixture, "beta"),
      gamma: lockEntry(fixture, "gamma"),
    },
    version: 3,
  });

  fixture.fakeBunx = path.join(root, "fake-bunx.ts");
  fs.writeFileSync(fixture.fakeBunx, fakeBunxSource());
  fs.chmodSync(fixture.fakeBunx, 0o755);
  fixture.env = {
    ...process.env,
    FAKE_SKILLS_LOG: fixture.commandLog,
    PUBLISH_SKILLS_AGENTS_ROOT: fixture.agentsRoot,
    PUBLISH_SKILLS_BUNX: fixture.fakeBunx,
    PUBLISH_SKILLS_CLAUDE_ROOT: fixture.claudeRoot,
    PUBLISH_SKILLS_CODEX_ROOT: fixture.codexRoot,
    PUBLISH_SKILLS_LOCK_FILE: fixture.lockFile,
    PUBLISH_SKILLS_PROCESS_LOCK: fixture.processLock,
    PUBLISH_SKILLS_SOURCE_ROOT: fixture.sourceRoot,
  };
  return fixture;
}

function writeSkill(sourceRoot, name, target) {
  const root = path.join(sourceRoot, "skills", name);
  fs.mkdirSync(path.join(root, "scripts"), { recursive: true });
  fs.writeFileSync(path.join(root, "SKILL.md"), skillMarkdown(name, target));
  const script = path.join(root, "scripts", "run.sh");
  fs.writeFileSync(script, `#!/bin/sh\necho ${name}\n`);
  fs.chmodSync(script, 0o755);
}

function skillMarkdown(name, target) {
  const metadata = target === "shared" ? "" : `metadata:\n  install-targets: ${target}\n`;
  return `---\n${metadata}name: ${name}\ndescription: ${name}\n---\n\n# ${name}\n`;
}

function installFixtureSkill(fixture, name, target) {
  const source = path.join(fixture.sourceRoot, "skills", name);
  const agents = path.join(fixture.agentsRoot, "skills", name);
  const claude = path.join(fixture.claudeRoot, "skills", name);
  fs.mkdirSync(path.dirname(agents), { recursive: true });
  fs.mkdirSync(path.dirname(claude), { recursive: true });
  if (target === "shared" || target === "codex") fs.cpSync(source, agents, { recursive: true });
  if (target === "shared") fs.symlinkSync(path.relative(path.dirname(claude), agents), claude);
  if (target === "claude-code") fs.cpSync(source, claude, { recursive: true });
}

function lockEntry(fixture, sourceName, overrides = {}) {
  return {
    installedAt: "2026-01-01T00:00:00.000Z",
    skillFolderHash: git(fixture.sourceRoot, "rev-parse", `HEAD:skills/${sourceName}`),
    skillPath: `skills/${sourceName}/SKILL.md`,
    source: repository,
    sourceType: "github",
    sourceUrl,
    updatedAt: "2026-01-01T00:00:00.000Z",
    ...overrides,
  };
}

function driftInstalledSkill(fixture, name, target) {
  const root = target === "claude" ? fixture.claudeRoot : fixture.agentsRoot;
  fs.appendFileSync(path.join(root, "skills", name, "SKILL.md"), "drift\n");
}

function commitFixtureFile(fixture, relativePath, content, push) {
  fs.writeFileSync(path.join(fixture.sourceRoot, relativePath), content);
  git(fixture.sourceRoot, "add", relativePath);
  git(fixture.sourceRoot, "commit", "-m", relativePath);
  if (push) git(fixture.sourceRoot, "push");
}

function run(fixture, ...args) {
  return spawnSync(process.execPath, [scriptPath, ...args], {
    encoding: "utf8",
    env: fixture.env,
  });
}

function runJson(fixture, ...args) {
  const result = run(fixture, ...args);
  return { json: result.stdout ? JSON.parse(result.stdout) : null, result };
}

function readLock(fixture) {
  return JSON.parse(fs.readFileSync(fixture.lockFile, "utf8"));
}

function writeLock(fixture, data) {
  fs.writeFileSync(fixture.lockFile, `${JSON.stringify(data, null, 2)}\n`);
}

function readCommandLog(fixture) {
  if (!fs.existsSync(fixture.commandLog)) return [];
  return fs
    .readFileSync(fixture.commandLog, "utf8")
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function commandKind(args) {
  if (args[1] === "remove") return "remove";
  const agents = args.slice(args.indexOf("--agent") + 1, args.indexOf("--skill"));
  if (agents.length === 2) return "shared";
  return agents[0] === "claude-code" ? "claude" : "codex";
}

function hasDrift(plan, skill, kind) {
  return plan.drifts.some((drift) => drift.skill === skill && drift.kind === kind);
}

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function fakeBunxSource() {
  return `#!/usr/bin/env bun
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
fs.appendFileSync(process.env.FAKE_SKILLS_LOG, JSON.stringify(args) + "\\n");
if (process.env.FAKE_SKILLS_FAIL_MATCH && args.join(" ").includes(process.env.FAKE_SKILLS_FAIL_MATCH)) process.exit(9);
const skillIndex = args.indexOf("--skill");
const end = args.indexOf("--yes", skillIndex);
const names = args.slice(skillIndex + 1, end);
const lockPath = process.env.PUBLISH_SKILLS_LOCK_FILE;
const lock = JSON.parse(fs.readFileSync(lockPath, "utf8"));
const roots = {
  agents: process.env.PUBLISH_SKILLS_AGENTS_ROOT,
  claude: process.env.PUBLISH_SKILLS_CLAUDE_ROOT,
  codex: process.env.PUBLISH_SKILLS_CODEX_ROOT,
};
const sourceRoot = process.env.PUBLISH_SKILLS_SOURCE_ROOT;

if (args[1] === "remove") {
  for (const name of names) {
    for (const root of Object.values(roots)) fs.rmSync(path.join(root, "skills", name), { force: true, recursive: true });
    delete lock.skills[name];
  }
} else {
  const agentIndex = args.indexOf("--agent");
  const agents = args.slice(agentIndex + 1, skillIndex);
  for (const name of names) {
    const source = path.join(sourceRoot, "skills", name);
    const canonical = path.join(roots.agents, "skills", name);
    const claude = path.join(roots.claude, "skills", name);
    const codex = path.join(roots.codex, "skills", name);
    fs.mkdirSync(path.dirname(canonical), { recursive: true });
    fs.mkdirSync(path.dirname(claude), { recursive: true });
    fs.rmSync(codex, { force: true, recursive: true });
    if (agents.includes("codex")) {
      fs.rmSync(canonical, { force: true, recursive: true });
      fs.cpSync(source, canonical, { recursive: true });
    }
    if (agents.includes("claude-code")) {
      fs.rmSync(claude, { force: true, recursive: true });
      if (agents.includes("codex")) fs.symlinkSync(path.relative(path.dirname(claude), canonical), claude);
      else fs.cpSync(source, claude, { recursive: true });
    }
    lock.skills[name] = {
      installedAt: lock.skills[name]?.installedAt ?? "2026-01-01T00:00:00.000Z",
      skillFolderHash: execFileSync("git", ["rev-parse", "HEAD:skills/" + name], { cwd: sourceRoot, encoding: "utf8" }).trim(),
      skillPath: "skills/" + name + "/SKILL.md",
      source: "${repository}",
      sourceType: "github",
      sourceUrl: "${sourceUrl}",
      updatedAt: "2026-01-02T00:00:00.000Z",
    };
  }
}
fs.writeFileSync(lockPath, JSON.stringify(lock, null, 2) + "\\n");
`;
}
