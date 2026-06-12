---
argument-hint: '[skill-name ...] [--dry-run]'
disable-model-invocation: true
name: update-skills
user-invocable: true
description: Use ONLY to check or update the project-scoped agent skills installed under .agents/skills so they match the current state of the repo. Do not trigger for creating, finding, or installing skills, or for README/AGENTS.md updates.
---

# Update Project Skills

## Overview

Re-sync the project-scoped agent skills installed under a repo's `.agents/skills/` directories with the current state of that repo. Installed skills drift as the code evolves: documented commands get renamed, paths move, flags change, conventions shift. This skill verifies each skill's factual claims against the repo and fixes what no longer holds — verify, don't rewrite. It never restyles a skill, reorders its sections, or invents new ones.

By default the skill operates **recursively** across the whole repository: every `.agents/skills/<name>/SKILL.md` at any depth is a target, including nested `.agents/skills` directories inside monorepo packages. Each skill is verified against its own project root — the directory holding its `.agents/` dir — not the repo root blindly. Factual fixes are applied by default; pass `--dry-run` to preview instead.

## Scope

This is the central rule for everything this skill reads and writes.

| Location                                                             | Treatment                                                  |
| -------------------------------------------------------------------- | ---------------------------------------------------------- |
| `<dir>/.agents/skills/<name>/` physically inside the repo, any depth | Target: verified and updated                               |
| `~/.agents/skills/` (global install)                                 | Never scanned, never written                               |
| `~/.claude/skills/` (global install)                                 | Never scanned, never written                               |
| Any `.claude/skills/` directory                                      | Never enumerated; real (non-symlink) ones silently ignored |
| `skills/` catalog trees (skill-development repos)                    | Not an install; never a target                             |

A repo that *develops* skills under `skills/` is out of scope by construction: the discovery glob only matches `.agents/skills`. Running this skill inside such a repo finds zero targets — the expected result, not an error.

## Arguments

- `skill-name ...` (positional): Restrict the run to the named skills, matched against the skill directory name. A name matching several nested locations selects all of them. Zero matches → report `✗ <name> not found` and list the discovered skill names.
- `--dry-run`: Preview every planned change without writing files.

If the user passes other flags, fall back to default mode and surface a one-line note about the unrecognized flag in the final report.

## Guard Rails

Run these checks before anything else. Any abort stops the whole workflow — there is no fallback directory.

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

Never run from `/`, from anywhere under `~/.agents` or `~/.claude`, or outside a git repository; never accept a repo whose toplevel is `/`, `$HOME`, or under those dot-dirs. These rules keep the skill from ever treating a global skill install as a project.

## Discovery

Enumerate targets from `$repo_root`. `--no-ignore` matters because repos often gitignore `.agents/`; `--hidden` reaches the dot-dir; `--follow` traverses `.agents/skills` dirs symlinked across packages; `--exclude .claude` enforces Scope mechanically.

```sh
fd --glob --full-path --hidden --no-ignore --follow --type f \
   --exclude .git --exclude .claude --exclude node_modules --exclude vendor \
   --exclude dist --exclude build --exclude out --exclude target \
   --exclude .next --exclude .venv --exclude coverage \
   '**/.agents/skills/*/SKILL.md' "$repo_root" \
| while IFS= read -r p; do
    dir="$(cd "${p%/SKILL.md}" 2>/dev/null && pwd -P)" || continue
    case "$dir" in
      "$HOME/.agents/"*|"$HOME/.claude/"*) continue ;;
      "$repo_root"/*) printf '%s/SKILL.md\n' "$dir" ;;
    esac
  done \
| awk '!seen[$0]++'
```

The realpath stage (`cd … && pwd -P`) drops anything resolving under `~/.agents` or `~/.claude` or escaping the repo — a skill dir symlinked to a global install is skipped with `⊘` — and the `awk` stage dedupes aliases of the same physical directory.

Zero results → report `⊘ No project skills found under .agents/skills in this repo.` and stop successfully.

## Per-Skill Project Root

Each discovered skill belongs to the package that installed it:

```sh
project_root="${skill_md%/.agents/skills/*}"
```

Verify claims against `project_root` first; fall back to `$repo_root` for repo-wide claims (default branch, CI files, root configs). Never verify a nested package's skill against another package's manifests. Command sources are the `justfile` and `package.json` scripts (plus lock files for package-manager detection).

## Workflow

When to use: the user asks to update, refresh, re-sync, check, or verify the project skills installed under `.agents/skills`. Trigger phrases include "update the project skills", "sync .agents/skills", "check the installed skills against the repo".

Steps:

1. Parse arguments (names, `--dry-run`).
2. Run guard rails; abort on any violation.
3. Enumerate targets via the discovery pipeline; apply name filters.
4. Resolve each skill's project root.
5. Check skill-internal integrity (frontmatter, bundled-file references).
6. Extract verifiable claims from the skill's docs.
7. Verify each claim against the repo; fix discrepancies.
8. Flag obsolete skills; collect suggested additions.
9. Apply updates (or emit the dry-run preview), format changed files, report.

Inputs: every discovered `SKILL.md` plus its `references/`, `scripts/`, `assets/`, and `examples/`; each project root's `justfile`, `package.json`, lock files, and configs. Outputs: corrected skill files in place, plus the grouped report.

Recognised flags: `skill-name ...`, `--dry-run`.

See [references/update-skills.md](references/update-skills.md).

## Update Rules

- Factual corrections only: paths, commands, flags, env vars, symbols, versions, conventions. Never restyle, reorder, or rewrite for taste.
- Smallest span per fix: change the word, line, or table cell that is wrong; leave the rest byte-identical.
- Preserve each skill's voice and structure. The repo is the source of truth for facts; the skill is the source of truth for how it presents them.
- Obsolete skill (its entire subject is gone from the repo): flag `⚠`, suggest the user shelve or delete it; never delete it or hollow out its content.
- External or unverifiable claims (URLs, third-party API behavior, `~/...` paths outside the repo): leave untouched.
- New repo features near a skill's scope: report-only under "Suggested Additions" unless the skill's stated scope unambiguously covers them.

## Safety Defaults

- Never auto-commit. Rely on git for recovery; do not create `*.backup` files.
- Never write outside the discovered target set, and never write under `$HOME`.
- When a run would modify more than a handful of skills, list the planned targets first — treat it as an implicit `--dry-run` preview — before writing.
- Format changed files with the host repo's own formatter only if one is discoverable; never install tooling.

## Reporting

Use the standard symbols:

- `✓` updated or verified up to date
- `⊘` skipped (symlink to a global install, zero targets, no formatter)
- `⚠` advisory (obsolete skill, missing bundled file, unrecognized flag)
- `✗` failure (unparseable frontmatter left unfixed, named skill not found)

Group results per `.agents/skills` location, using its path relative to the repo root as a sub-header; indent per-skill deltas under each skill line so a single skill's changes scan without re-reading the header. End with a one-line tally, e.g. `Checked 4 skills: 2 updated, 1 up to date, 1 obsolete.` For `--dry-run`, open the report with `## Planned Changes` and show per-skill bullet diffs instead of applied results.

## Additional Resources

- **`references/update-skills.md`** — Complete verify-and-fix workflow: argument parsing, guard rails, discovery and fallbacks, claim extraction, per-claim verification commands, obsolete-skill handling, formatting, and report templates.
