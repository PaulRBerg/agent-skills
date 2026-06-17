---
argument-hint: <polish|create> [path] [skill-name ...] [--root-only] [--preserve] [--minimal] [--thorough|--full] [--dry-run] [--force]
disable-model-invocation: false
name: agents-context
user-invocable: true
description: "Use to create or polish repo agent context files: README.md, AGENTS.md/CLAUDE.md symlinks, and existing project .agents/skills/SKILL.md files. Updates existing skills only; do not create skills."
---

# Agents Context

Create and polish repo-local context as one system: human README files, agent-facing AGENTS.md files with companion CLAUDE.md symlinks, and existing project-installed skills under `.agents/skills`.

## Workflows

Choose exactly one workflow.

| User intent                                                             | Workflow | Reference                                                    |
| ----------------------------------------------------------------------- | -------- | ------------------------------------------------------------ |
| Update, refresh, sync, check, prune, polish, or repair existing context | `polish` | `references/brain-polish.md`                                 |
| Create or initialize missing README.md / AGENTS.md files                | `create` | `references/create-docs.md`                                  |
| Create, scaffold, install, or discover a skill                          | Stop     | Refer to `skills/create-skill` or the user's skill installer |

Default to `polish` when the user asks for updates or invokes the skill without a clear creation request. Default to `create` only when the user says create, init, initialize, generate, or new for README.md / AGENTS.md context files.

## Arguments

- `path`: Optional repo-relative subtree. Restrict README.md, AGENTS.md, CLAUDE.md, package-root, and project-skill discovery to that subtree.
- `skill-name ...`: Optional filters for the existing project-installed skills under `.agents/skills/<name>/`. Applies only to project skill targets during `polish`.
- `--root-only`: Restrict README.md / AGENTS.md creation or polishing to the repository root. Does not select nested `.agents/skills` targets.
- `--dry-run`: Preview planned writes and diffs without changing files.
- `--preserve`: For `polish`, keep user-authored prose and structure where still accurate; fix only drift and obvious noise.
- `--minimal`: Use the smallest useful README.md / AGENTS.md output.
- `--thorough` / `--full`: Perform deeper analysis and include more detail when it adds durable context.
- `--force`: For `create`, overwrite existing README.md or AGENTS.md targets without prompting. Never applies to skills.

Surface unrecognized flags in the final report and continue with defaults when safe.

## Scope

Stay inside the current git repository. Never scan or write global installs under `~/.agents`, `~/.claude`, or any `.claude/skills` directory.

This skill may write:

- Existing `README.md` files during `polish`.
- Missing `README.md` files during `create`.
- Existing `AGENTS.md` files during `polish`.
- Missing `AGENTS.md` files during `create`.
- Sibling `CLAUDE.md` symlinks that mirror an `AGENTS.md` in the same directory.
- Existing repo-local `.agents/skills/<name>/SKILL.md` files, plus their existing `references/`, `scripts/`, `assets/`, and `examples/` files when a minimal factual fix requires it.

This skill must not:

- Create, install, delete, or rename skills. For new skills, refer to `skills/create-skill`.
- Treat catalog directories such as `skills/<name>/` as project-installed skills.
- Process `CLAUDE.md` as a source file unless it is a symlink to sibling `AGENTS.md`.
- Rewrite regular `CLAUDE.md` files.
- Edit `CONTRIBUTING.md`; only recommend merging it into sibling `AGENTS.md`.
- Auto-commit.

## Guard Rails

Run these checks before discovery or writes:

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

Snapshot `git status --short` before broad edits. Ignore unrelated pre-existing changes, but do not overwrite them blindly.

## Discovery

Use git-aware discovery for documentation targets:

```sh
git -C "$repo_root" ls-files --cached --others --exclude-standard -- \
  '**/README.md' 'README.md' '**/AGENTS.md' 'AGENTS.md'
```

For creation, package roots are the repo root plus directories containing a manifest:

```sh
git -C "$repo_root" ls-files --cached --others --exclude-standard -- \
  '**/package.json' 'package.json' '**/Cargo.toml' 'Cargo.toml' \
  '**/pyproject.toml' 'pyproject.toml' '**/setup.py' 'setup.py' \
  '**/go.mod' 'go.mod' '**/foundry.toml' 'foundry.toml' \
  '**/Gemfile' 'Gemfile' '**/composer.json' 'composer.json'
```

Discover existing project-installed skills deliberately, including ignored `.agents/` directories:

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

Skip VCS, dependency, and build output directories: `.git`, `node_modules`, `vendor`, `.venv`, `target`, `dist`, `build`, `out`, `.next`, and `coverage`. Skip hidden dot-directories unless they contain a manifest or are the explicit `.agents/skills` target set.

## Core Rules

- Treat `README.md` as human-facing. Keep description, badges, links, references, acknowledgments, license, and a pointer to AGENTS.md. Keep CLI commands only for short operator-run setup guides such as dotfiles, infra, homelab, or explicit user requests.
- Treat `AGENTS.md` as agent-facing. Keep commands, non-obvious architecture, style rules, safety constraints, generated-file ownership, workflow traps, and contribution guidance.
- Treat `CLAUDE.md` as a compatibility symlink. Create or refresh it with `(cd "$dir" && ln -sf AGENTS.md CLAUDE.md)` only when it is missing or already a symlink.
- Treat project-installed `SKILL.md` files as existing workflows to verify, not content to redesign. Fix factual drift with the smallest edit that restores truth.
- Prefer deletion over summarizing noise. Remove content that can be cheaply inferred from manifests, filenames, or standard tool behavior.
- Keep parent `AGENTS.md` files for global defaults and nested `AGENTS.md` files for local deltas.
- During `polish`, recommend missing nested AGENTS.md files only when a subtree has distinct commands, safety rules, generated files, ownership boundaries, or data-handling constraints. Create them only through the `create` workflow.

## Verification

After writes:

1. Run the host repo's Markdown formatter/checker. Prefer `just mdformat-write`, then `just mdformat-check` when present.
2. If any `SKILL.md` frontmatter or `agents/openai.yaml` changed in a skills catalog, run the host repo's invocation metadata check. Prefer `just skill-invocation-check` when present.
3. Verify changed `CLAUDE.md` symlinks resolve to sibling `AGENTS.md`.
4. For `--dry-run`, skip writes and verification that depends on changed files; report the exact commands that would run.

## Reporting

Report in this order:

1. **Scope**: target counts and relative paths for README.md, AGENTS.md, CLAUDE.md symlinks, and existing project skills.
2. **Changes**: grouped by relative directory or `.agents/skills` location.
3. **Advisories**: CONTRIBUTING.md merge notes, obsolete skills, skipped regular CLAUDE.md files, unrecognized flags.
4. **Verification**: exact commands run and outcomes.
5. **Residual Risks**: `None.` or one terse line per risk.

Use `✓` for updated/created/verified, `⊘` for skipped, `⚠` for advisories, and `✗` for failures.

## References

- Read `references/brain-polish.md` for the unified update/polish workflow.
- Read `references/create-docs.md` for README.md and AGENTS.md creation.
