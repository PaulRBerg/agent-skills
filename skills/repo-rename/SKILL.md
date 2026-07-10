---
argument-hint: <new-name> [--dry-run]
disable-model-invocation: true
name: repo-rename
user-invocable: true
description: 'Preview or rename a GitHub upstream repo and matching local project folder, update origin, and preserve Codex CLI and Claude Code continuity references.'
---

# Repo Rename

Preview the complete GitHub, Git, filesystem, and agent-continuity mutation set before applying a repository rename.

## Scope

- Rename the current GitHub repository, matching local folder, and `origin` URL.
- Update literal old paths in `~/.claude/projects`, `~/claude/projects` when present, `~/.codex/sessions`, and `~/.codex/config.toml`.
- Update constrained literal old-name references inside the repository, excluding VCS, dependencies, and generated build directories.
- Do not modify other transcript stores, archives, remotes, or repositories.

## Workflow

1. Resolve the skill directory and run the helper from the repository to produce a read-only preview:

   ```sh
   uv run <skill-dir>/scripts/repo-rename.py <new-name> --dry-run
   ```

   The helper requires a clean worktree, matching GitHub repo/folder names, a free target path, and valid GitHub context. It prints the old/new repo and paths, exact confirmation token, external commands, directory moves, every replacement file, occurrence counts, and rollback coverage.

2. If `--dry-run` was requested, return the preview and stop. A successful dry run must not change the GitHub repo, remote, folder, config, transcripts, or repository files.

3. Otherwise present the complete preview and require explicit confirmation in a subsequent user message. Explain that the confirmation authorizes the GitHub rename, local folder move, origin update, and every listed continuity/repository replacement. If any preview fact changes, regenerate it and ask again.

4. After confirmation, pass the preview's exact token:

   ```sh
   uv run <skill-dir>/scripts/repo-rename.py <new-name> --apply --confirm '<owner/old->owner/new>'
   ```

   The helper re-runs preflight, backs up every replacement file, applies deterministic literal replacements, and attempts rollback on failure. Never bypass its clean-tree, target-path, or confirmation checks.

5. Verify the helper's result with `gh repo view`, `git remote get-url origin`, `pwd -P`, and a search for remaining old absolute-path/name references in the listed scope. Report intentional remaining matches separately.

## Completion

Dry-run completion is a full mutation preview with zero writes. Apply completion requires the new GitHub identity, folder, remote, and listed continuity references to agree, plus a clean verification report. If rollback is incomplete, stop and report every failed rollback action before doing anything else.
