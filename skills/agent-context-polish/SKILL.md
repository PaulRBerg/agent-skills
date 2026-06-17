---
argument-hint: '[path] [--dry-run]'
disable-model-invocation: false
name: agent-context-polish
user-invocable: true
description: 'Use to polish repo-local AGENTS.md context, companion CLAUDE.md symlinks, and project .agents/skills context: sync installed skills, prune noisy guidance, and restructure nested agent context when it improves analysis.'
---

# Agent Context Polish

## Objective

Tighten repo-local agent context after syncing the sources that maintain it. The output is not more documentation; it is less noise and better placement: keep guidance, preferences, workflow traps, and non-obvious repo rules that an agent cannot cheaply infer from the tree. Prefer a small, scoped `AGENTS.md` hierarchy over one broad root file when locality makes instructions more accurate, and keep companion `CLAUDE.md` symlinks colocated with the `AGENTS.md` files they mirror.

## Arguments

- `path`: Optional repo-relative subtree. Restrict the final polish scan to matching `AGENTS.md` files, companion `CLAUDE.md` symlinks, and `.agents/skills` installs below that path. Forward to `md-docs` when supported.
- `--dry-run`: Preview sub-skill and polish changes without writing files. Forward to sub-skills when supported.
- Default: operate on the whole git repository.

## Required Skills

This skill requires sibling skills installed next to it:

- `../agent-skills-update/SKILL.md`
- `../md-docs/SKILL.md`

Before doing any repo work, verify both files exist. If either is missing, stop and prompt the user to install it with `bunx skills`, then rerun this skill. Do not emulate the missing skill manually.

Use this wording:

```text
Missing required skill: <name>. Install it with `bunx skills add PaulRBerg/agent-skills` or the skill source that provides `<name>`, then rerun `agent-context-polish`.
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
- New repo-local nested `AGENTS.md` files when a subtree needs distinct agent guidance.
- Repo-local `CLAUDE.md` symlinks that mirror an `AGENTS.md` target and need to move, be created, or be removed with it.
- Every repo-local `.agents/skills/<name>/SKILL.md`, recursively.

Context:

- Other files under repo-local `.agents/skills/<name>/` may be read only to validate a `SKILL.md` claim.
- Other files in candidate subtrees may be read to decide whether `AGENTS.md` guidance belongs in a parent, child, or new nested file.
- Treat `CLAUDE.md` as an alias only when it is a symlink. Do not merge, rewrite, or relocate regular `CLAUDE.md` files as context sources.
- Do not treat catalog trees such as `skills/<name>/` as installed project skills.
- Do not rewrite `references/`, `scripts/`, `assets/`, or `examples/` unless a minimal edit is required to repair a reference broken by the `SKILL.md` polish.

Discovery should respect git ignores for `AGENTS.md` and `CLAUDE.md`, filter `CLAUDE.md` results to symlinks with `test -L`, and deliberately include hidden `.agents/skills` installs:

```sh
git -C "$repo_root" ls-files --cached --others --exclude-standard -- '**/AGENTS.md' 'AGENTS.md'
git -C "$repo_root" ls-files --cached --others --exclude-standard -- '**/CLAUDE.md' 'CLAUDE.md'
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
3. Verify the required sibling skills are installed. Stop on the first missing skill and prompt for `bunx skills` installation.
4. Run `$agent-skills-update` first:
   - Forward `--dry-run` when present.
   - Do not forward `path`; `agent-skills-update` owns repo-local `.agents/skills` discovery.
   - If it reports no project skills, continue to `md-docs`.
5. Run `$md-docs update-all` next:
   - Forward `path` and `--dry-run` when present.
   - If the installed `md-docs` does not expose `update-all`, report the fallback and run `$md-docs update-readme` followed by `$md-docs update-agents` with the same supported arguments.
   - If `md-docs` is missing or either documented workflow fails, stop before the polish pass.
6. Rediscover targets and companion `CLAUDE.md` symlinks after the sub-skills finish.
7. Polish the `AGENTS.md` hierarchy, companion `CLAUDE.md` symlinks, and repo-local project-skill `SKILL.md` files:
   - For `AGENTS.md`, prune, relocate, add, or remove files only when the structure becomes more accurate for agents analyzing the codebase.
   - For `CLAUDE.md`, relocate, add, or remove only symlinks that mirror moved, added, or deleted `AGENTS.md` files. Preserve or adjust relative link targets so the symlink remains valid from its new directory.
   - For project-skill `SKILL.md`, edit only discovered repo-local `.agents/skills/<name>/SKILL.md` files.
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

## AGENTS.md and CLAUDE.md Placement

Treat each `AGENTS.md` as scoped instructions for its directory tree. Treat `CLAUDE.md` only as a compatibility symlink to the same scoped instructions.

- Move subtree-specific guidance from a parent `AGENTS.md` to the deepest common ancestor where it applies.
- Promote duplicated nested guidance to the nearest shared parent only when it genuinely applies across those children.
- Add a nested `AGENTS.md` when a subtree has distinct commands, generated files, ownership boundaries, safety rules, data-handling rules, or review constraints that would be noisy or misleading at the repo root.
- Prune a root or nested `AGENTS.md` when it repeats parent guidance, describes obvious layout, or carries stale context. Delete an `AGENTS.md` only when no useful guidance remains after pruning.
- Keep parent files for global defaults and child files for local deltas, exceptions, and sharper constraints.
- Do not create nested `AGENTS.md` files merely to document directories, package names, or facts an agent can discover from manifests and filenames.
- When relocating an `AGENTS.md`, relocate any sibling `CLAUDE.md` symlink that points to that `AGENTS.md`, adjusting the link target if needed.
- When adding a nested `AGENTS.md` below a tree that maintains sibling `CLAUDE.md` symlinks, add the same companion symlink for the new file.
- When deleting an `AGENTS.md`, delete only a sibling `CLAUDE.md` symlink that points to that deleted file. Leave regular `CLAUDE.md` files and symlinks to other targets untouched.
- After moving, adding, or deleting `AGENTS.md` files or companion `CLAUDE.md` symlinks, rediscover targets and ensure no useful local constraint was orphaned.

## Verification

After edits:

- Run the host repo's Markdown formatter/checker if discoverable. Prefer `just mdformat-write` then `just mdformat-check` when those recipes exist.
- Run skill metadata checks when `SKILL.md` frontmatter or `agents/openai.yaml` files changed. Prefer `just skill-invocation-check` when present.
- For `--dry-run`, skip writes and report the exact files and sections that would change.
- If no formatter/checker exists, report the skip.

## Report

Use this order:

### Scope

List counts and relative paths for existing, added, removed, and relocated `AGENTS.md` files and companion `CLAUDE.md` symlinks plus project-skill `SKILL.md` targets. Say explicitly when no `.agents/skills` installs exist.

### Sync

Summarize `$agent-skills-update` and `$md-docs` outcomes, including any `update-all` fallback.

### Polish

List each changed file, content relocated between `AGENTS.md` files, new or removed `AGENTS.md` files, companion `CLAUDE.md` symlinks moved, added, or removed, and noise removed or compressed. For `--dry-run`, list planned changes instead.

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
- A planned `CLAUDE.md` write would modify a regular file instead of a symlink.
- A planned write would touch global skills or files outside `$repo_root`.
