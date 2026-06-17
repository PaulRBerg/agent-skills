---
argument-hint: '[path] [--dry-run]'
disable-model-invocation: false
name: agent-context-polish
user-invocable: true
description: 'Use to polish repo-local AGENTS.md and project .agents/skills context: sync installed skills, refresh docs, and remove verbose or trivially inferable agent instructions.'
---

# Agent Context Polish

## Objective

Tighten repo-local agent context after syncing the sources that maintain it. The output is not more documentation; it is less noise: keep guidance, preferences, workflow traps, and non-obvious repo rules that an agent cannot cheaply infer from the tree.

## Arguments

- `path`: Optional repo-relative subtree. Restrict the final polish scan to matching `AGENTS.md` files and `.agents/skills` installs below that path. Forward to `md-docs` when supported.
- `--dry-run`: Preview sub-skill and polish changes without writing files. Forward to sub-skills when supported.
- Default: operate on the whole git repository.

## Required Skills

This skill requires sibling skills installed next to it:

- `../agent-skills-update/SKILL.md`
- `../md-docs/SKILL.md`

Before doing any repo work, verify both files exist. If either is missing, stop and prompt the user to install it with `npx skills`, then rerun this skill. Do not emulate the missing skill manually.

Use this wording:

```text
Missing required skill: <name>. Install it with `npx skills add PaulRBerg/agent-skills` or the skill source that provides `<name>`, then rerun `agent-context-polish`.
```

Read each sibling `SKILL.md` once and follow its instructions inline as if invoked with the arguments described below. `$agent-skills-update` and `$md-docs` are skill invocations, not shell commands.

## Guard Rails

Run these checks before touching files:

```sh
cwd="$(pwd -P)"
case "$cwd" in
  /) printf 'abort: refusing to run at the filesystem root\n' >&2; exit 1 ;;
  "$HOME/.agents"|"$HOME/.agents/"*|"$HOME/.claude"|"$HOME/.claude/"*)
    printf 'abort: refusing to run under ~/.agents or ~/.claude\n' >&2; exit 1 ;;
esac
repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  printf 'abort: not inside a git repository\n' >&2; exit 1; }
case "$repo_root" in
  /|"$HOME") printf 'abort: unsupported repo root: %s\n' "$repo_root" >&2; exit 1 ;;
  "$HOME/.agents"|"$HOME/.agents/"*|"$HOME/.claude"|"$HOME/.claude/"*)
    printf 'abort: repo root is under ~/.agents or ~/.claude\n' >&2; exit 1 ;;
esac
```

Snapshot `git status --short` before running sub-skills or broad generators. Ignore unrelated pre-existing changes, but do not overwrite them blindly.

## Scope

Stay inside the current git repository. Never scan or write global skill installs under `~/.agents`, `~/.claude`, or `.claude/skills`.

Targets:

- Every repo-local `AGENTS.md`, recursively.
- Every repo-local `.agents/skills/<name>/SKILL.md`, recursively.

Context:

- Other files under repo-local `.agents/skills/<name>/` may be read only to validate a `SKILL.md` claim.
- Do not treat catalog trees such as `skills/<name>/` as installed project skills.
- Do not rewrite `references/`, `scripts/`, `assets/`, or `examples/` unless a minimal edit is required to repair a reference broken by the `SKILL.md` polish.

Discovery should respect git ignores for `AGENTS.md` and deliberately include hidden `.agents/skills` installs:

```sh
git -C "$repo_root" ls-files --cached --others --exclude-standard -- '**/AGENTS.md' 'AGENTS.md'
fd --glob --full-path --hidden --no-ignore --follow --type f \
   --exclude .git --exclude .claude --exclude node_modules --exclude vendor \
   --exclude dist --exclude build --exclude out --exclude target \
   --exclude .next --exclude .venv --exclude coverage \
   '**/.agents/skills/*/SKILL.md' "$repo_root"
```

For each discovered skill path, resolve the physical directory with `cd "${p%/SKILL.md}" && pwd -P`; skip it if the real path escapes `$repo_root` or lands under `~/.agents` or `~/.claude`.

## Workflow

1. Parse `path` and `--dry-run`.
2. Run the guard rails and resolve `$repo_root`.
3. Verify the required sibling skills are installed. Stop on the first missing skill and prompt for `npx skills` installation.
4. Run `$agent-skills-update` first:
   - Forward `--dry-run` when present.
   - Do not forward `path`; `agent-skills-update` owns repo-local `.agents/skills` discovery.
   - If it reports no project skills, continue to `md-docs`.
5. Run `$md-docs update-all` next:
   - Forward `path` and `--dry-run` when present.
   - If the installed `md-docs` does not expose `update-all`, report the fallback and run `$md-docs update-readme` followed by `$md-docs update-agents` with the same supported arguments.
   - If `md-docs` is missing or either documented workflow fails, stop before the polish pass.
6. Rediscover targets after the sub-skills finish.
7. Polish only the discovered `AGENTS.md` and repo-local project-skill `SKILL.md` files.
8. Verify changed Markdown with the narrowest formatter/checker available. Prefer `just` recipes when present and inspect unclear recipes before running them.
9. Report sub-skill outcomes, polish changes, verification, skipped checks, and residual risks.

## Polish Rules

Remove or compress content that is easy to infer by inspecting the repo. Prefer deletion over replacing noise with a new summary.

Flag these as noise:

- Exact folder/file layout dumps, directory trees, or long "Structure" sections that add no convention or warning.
- Lists of installed skills or available skills. Agents load those automatically.
- Generic textbook facts or tool definitions, such as what React, TypeScript, Python, npm, git, or Markdown are.
- Repeated manifest data, package script inventories, or command lists copied verbatim without preference, ordering, side effects, or failure guidance.
- File-by-file descriptions that merely restate names an agent can discover with `rg --files`.
- Tutorial prose for common tooling where the repo does not have a special constraint.
- Historical authoring notes, implementation logs, or commentary about why a context file was created.

Keep or sharpen these:

- Coding preferences, review standards, naming rules, and architectural constraints.
- Commands with non-obvious ordering, side effects, environment requirements, or "use this instead of that" guidance.
- Safety, privacy, financial, credential, deployment, and data-handling rules.
- Generated-file warnings, ownership boundaries, and files that must not be edited directly.
- Repo-specific speed traps, flaky checks, migration constraints, and external system quirks.
- Skill invocation rules that are not discoverable from the filesystem alone.

When a section mixes useful guidance with obvious inventory, keep the useful guidance and remove only the inventory. Do not broaden scope or rewrite for style once the noise is gone.

## Verification

After edits:

- Run the host repo's Markdown formatter/checker if discoverable. Prefer `just mdformat-write` then `just mdformat-check` when those recipes exist.
- Run skill metadata checks when `SKILL.md` frontmatter or `agents/openai.yaml` files changed. Prefer `just skill-invocation-check` when present.
- For `--dry-run`, skip writes and report the exact files and sections that would change.
- If no formatter/checker exists, report the skip.

## Report

Use this order:

### Scope

List counts and relative paths for `AGENTS.md` and project-skill `SKILL.md` targets. Say explicitly when no `.agents/skills` installs exist.

### Sync

Summarize `$agent-skills-update` and `$md-docs` outcomes, including any `update-all` fallback.

### Polish

List each changed file and the noise removed or compressed. For `--dry-run`, list planned changes instead.

### Verification

Name exact commands run and outcomes. Name skipped checks and why.

### Residual Risks

Write `None.` when none remain. Otherwise use one terse line per risk.

## Stop Conditions

Stop without editing when:

- A required sibling skill is missing.
- The current directory is not inside a supported git repository.
- A sub-skill fails or requires user confirmation.
- A target has unparseable frontmatter and the minimal safe fix is not obvious.
- A planned write would touch global skills or files outside `$repo_root`.
