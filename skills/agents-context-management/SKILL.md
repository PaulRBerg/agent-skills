---
argument-hint: <polish|create> [path] [skill-name ...] [--root-only] [--preserve] [--minimal] [--thorough|--full] [--dry-run] [--force]
disable-model-invocation: true
name: agents-context-management
user-invocable: true
description: "Create or polish repo agent context: README.md, AGENTS.md/CLAUDE.md, and installed project skills."
---

# Agents Context Management

Create or polish repo-local context as one coherent system: human-facing README.md files, agent-facing AGENTS.md files with companion CLAUDE.md symlinks, and existing project-installed skills under `.agents/skills`.

Success means every selected target is grounded in repository evidence, respects its audience and scope, and passes the narrowest repository-defined validation. Stop after reporting completed or planned changes, validation, and any blockers.

## Choose a Workflow

Choose exactly one workflow and read only its reference.

| User intent                                                     | Workflow                     | Reference                                 |
| --------------------------------------------------------------- | ---------------------------- | ----------------------------------------- |
| Update, refresh, sync, prune, polish, repair, or fix context    | `polish`                     | `references/brain-polish.md`              |
| Create, initialize, generate, or regenerate context files       | `create`                     | `references/create-docs.md`               |
| Audit, check, review, inspect, or suggest changes without edits | `polish` in `--dry-run` mode | `references/brain-polish.md`              |
| Create or scaffold a skill                                      | Stop                         | Refer to `skills/create-skill`            |
| Install, discover, remove, or rename a skill                    | Stop                         | Use a dedicated skill-management workflow |

If the intent is unclear, select `polish` in `--dry-run` mode and report the smallest useful planned change set.

## Authority

- Explicit create, update, polish, repair, fix, or equivalent intent authorizes in-scope local writes. Inspection-only intent and `--dry-run` do not.
- Require explicit confirmation before deleting README.md, AGENTS.md, or CLAUDE.md entries. `--force` authorizes documented overwrites, not deletions.
- Treat a broad write request as authorization for the requested scope. Otherwise, preview a change set larger than a handful of files and stop before writing.
- Do not expand from documentation work into source changes, skill creation, external writes, or commits.

## Arguments

- `path`: Optional repo-relative subtree. Restrict documentation, package-root, and project-skill discovery to that subtree.
- `skill-name ...`: Optional filters for existing `.agents/skills/<name>/` targets during `polish`.
- `--root-only`: Select only root README.md, AGENTS.md, and CLAUDE.md targets. Exclude project skills unless explicitly selected by `skill-name`.
- `--dry-run`: Report planned writes and concise diffs without changing files.
- `--preserve`: During `polish`, keep accurate user-authored prose and structure; fix only drift and obvious noise.
- `--minimal`: Produce the smallest context that still meets the completion bar.
- `--thorough` / `--full`: Perform deeper analysis only where it adds durable, repository-specific context.
- `--force`: During `create`, regenerate existing README.md or AGENTS.md targets without prompting. Never applies to skills or deletions.

If `--minimal` and `--thorough` / `--full` are both present, make no writes and ask the user to choose. Report unrecognized flags; continue only when they cannot change scope, safety, or write behavior.

## Repository Guard Rail

Run before discovery or writes:

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

Snapshot `git status --short` before broad edits. Preserve unrelated pre-existing changes and re-check expected paths after generators or broad commands.

## Shared Constraints

- Stay inside the current git repository. Never scan or write global installs under `~/.agents`, `~/.claude`, or `.claude/skills`.
- Treat README.md as human-facing and AGENTS.md as agent-facing. Put stable shared rules in parent AGENTS.md files and local deltas in nested files.
- Treat CLAUDE.md only as a compatibility symlink to sibling AGENTS.md. Never rewrite a regular CLAUDE.md file.
- Treat existing project-installed `.agents/skills/<name>/` files as factual workflows to verify, not redesign.
- Never treat catalog `skills/<name>/` directories as project-installed skills.
- Never create, install, delete, or rename skills; edit CONTRIBUTING.md; or auto-commit.
- Remove content only when repository evidence shows it is stale, misplaced, duplicated, or cheaply inferable. Do not invent descriptions, links, commands, conventions, or ownership rules.

## Discovery and Tool Routing

- Use git-aware discovery for tracked and untracked README.md, AGENTS.md, manifests, and repository configuration.
- Deliberately inspect ignored `.agents/skills/*/SKILL.md` targets when project skills are in scope. Canonicalize candidates and accept only paths beneath `repo_root`.
- Exclude VCS, dependency, environment, and build-output directories. Skip hidden directories except explicit manifests and `.agents/skills` targets.
- Prefer `fd` for filesystem discovery. If unavailable, use `find` with the same exclusions and path checks. An empty or suspiciously narrow result warrants one meaningful fallback before concluding that no target exists.
- Read independent manifests and targets in parallel when practical; synthesize their evidence before writing.

## Completion and Report

After writes, run repository-defined Markdown formatting or checks when present. If skill frontmatter or `agents/openai.yaml` changed in a catalog, run its invocation metadata check. Verify changed CLAUDE.md symlinks resolve to sibling AGENTS.md. In `--dry-run`, report commands that would depend on planned files instead of running them.

Report only:

1. **Mode and Scope**: workflow, dry-run status, target counts, and relative paths.
2. **Changes**: completed or planned changes grouped by directory.
3. **Validation**: exact commands and outcomes, including justified skips.
4. **Blockers and Risks**: conflicts, advisories, unrecognized flags, or `None.`

Omit empty detail and stop once the selected targets meet the completion bar.

## References

- `polish`: read `references/brain-polish.md`.
- `create`: read `references/create-docs.md`.
