---
argument-hint: <new-name> | <source-path> <destination-path> [--dry-run]
disable-model-invocation: true
name: repo-rename
description:
  "Preview or rename a GitHub repo, or move a local repo to another parent directory. Preserve Codex CLI and Claude Code
  continuity references."
---

# Repo Rename

If a slash or dollar invocation already added these instructions to the conversation, follow them directly. In that
case, do not invoke this skill again through a skill tool.

Preview the complete GitHub, Git, filesystem, and agent-continuity mutation set before applying a repository rename.

## Scope

- Rename the current GitHub repository, matching local folder, and `origin` URL.
- With two path arguments, move the source repository to the destination without changing its name, GitHub identity, or
  `origin`. Both paths must have the same folder name. Preview missing parent directories before creating them.
- If the repository has no Git remotes, rename only local state: the folder, continuity references, and repository
  content. The helper then skips the GitHub rename and the `origin` update. A repository with any remote still requires
  a valid GitHub context.
- Only when the repository has an active Claude Code or Codex transcript, update literal old paths in
  `~/.claude/projects`, `~/claude/projects` when present, `~/.codex/sessions`, and `~/.codex/config.toml`. This
  continuity update does not apply to repos with no active transcripts.
- Update absolute repository paths, exact name fields, whole-line repository names or headings, and GitHub URLs for the
  repository. Preserve other old-name occurrences, including imports, prose, and command selectors. Exclude VCS,
  dependencies (including Python virtual environments), caches, and generated build directories.
- For a same-name move, replace only the old absolute repository path inside repository files.
- By default, use tracked repository files and skip `PROMPT.md`, `TODO.md`, and symlinks before reading replacement
  candidates. Prune excluded directories during discovery. When the user explicitly authorizes updating notes,
  dependencies, caches, or generated files, pass `--include-local-state` to both preview and apply. This includes those
  repository text files while preserving file permissions. Symlinks, binary files, and `.git` remain excluded.
- The helper honors `CLAUDE_CONFIG_DIR` and `CODEX_HOME`, gates continuity updates on active transcripts, and replaces
  only the absolute repository path in continuity stores and Codex config.
- Do not modify other transcript stores, archives, remotes, or repositories.

## Workflow

1. Resolve the source repository from the first of two path arguments, or from the current directory for a rename. Check
   whether it has any active AI chat transcript before treating continuity preservation as in scope:
   - Claude Code: resolve `${CLAUDE_CONFIG_DIR:-~/.claude}/projects/<encoded-repo-path>` (the absolute repo path with
     every non-alphanumeric character replaced by `-`) and check it exists and is non-empty.
   - Codex: resolve `${CODEX_HOME:-~/.codex}/sessions` and check whether any session file references the exact absolute
     repo path.
   - If neither exists, state that the transcript-continuity update does not apply. In that case, the rename may still
     proceed for the GitHub, git, filesystem, and repository-content changes below.

2. Resolve the skill directory and run the helper from the repository to produce a read-only preview:

   ```sh
   uv run <skill-dir>/scripts/repo-rename.py <new-name> --dry-run
   ```

   For a same-name move, run from any directory:

   ```sh
   uv run <skill-dir>/scripts/repo-rename.py <source-path> <destination-path> --dry-run
   ```

   The helper requires a clean worktree, matching GitHub repo/folder names, a free target path, and valid GitHub
   context. For a repository with no remotes, it requires only a clean worktree and a free target path. It reports
   `"local_only": true` and a `local:<old>-><new>` confirmation token. It prints the old/new repo and paths, exact
   confirmation token, external commands, directory moves, every replacement file, occurrence counts, and rollback
   coverage. Same-name moves use `move:<absolute-source>-><absolute-destination>` tokens. Linked worktrees and external
   Git directories are unsupported.

3. If `--dry-run` was requested, return the preview and stop. A successful dry run must not change the GitHub repo,
   remote, folder, config, transcripts, or repository files.

4. Otherwise present the complete preview, then continue in the same turn. Treat the rename request as authorization for
   the listed mutations. Do not require a separate confirmation message. For a local-only preview, state that no GitHub
   or `origin` change occurs. Lead with `### 🔎 Rename preview — no changes made`. Show GitHub, folder, origin,
   replacement counts, and rollback coverage in compact tables, then the exhaustive file/count list. If any preview fact
   changes, regenerate the preview and continue within the requested scope.

   For same-name moves, show the unchanged GitHub identity and `origin`, plus each parent directory to create.

5. Pass the preview's exact token to the helper internally:

   ```sh
   uv run <skill-dir>/scripts/repo-rename.py <new-name> --apply --confirm '<owner/old->owner/new>'
   ```

   For a same-name move, pass both paths and the preview's `move:` token instead.

   The helper re-runs preflight, backs up every replacement file, applies deterministic literal replacements, and
   attempts rollback on failure. Never bypass its clean-tree, target-path, or token checks.

6. Verify the helper's result with `gh repo view`, `git remote get-url origin`, `pwd -P`, and a search for remaining old
   absolute-path/name references in the listed scope. Report intentional remaining matches separately.

   If a moved Python environment retains old paths, reinstall its locked packages with the project's existing package
   manager. Then rerun the repository's test command.

## Completion

Dry-run completion is a full mutation preview with zero writes. Apply completion requires the new GitHub identity,
folder, remote, and listed continuity references to agree, plus a clean verification report. If rollback is incomplete,
stop and lead with `### ⛔ Rollback incomplete`, then report every failed rollback action before doing anything else. On
success, use `### ✅ Repository renamed` and a surface/expected/verified table. Keep helper JSON, confirmation tokens,
commands, paths, URLs, and rollback errors exact and undecorated.
