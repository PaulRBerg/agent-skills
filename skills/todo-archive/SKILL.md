---
argument-hint: "[path] [--hint TEXT] [--date YYYY-MM-DD|YYYY_MM_DD] [--dry-run] [--force]"
disable-model-invocation: true
effort: low
model: sonnet
name: todo-archive
user-invocable: true
description: Archive checked TODO.md tasks into `.ai/todos/TODO_UNTIL_YYYY_MM_DD.md`, leaving unchecked tasks.
---

# TODO Archive

`TODO.md` and `.ai/` are conventionally git-ignored, so they are untracked and `git diff` shows nothing for them.
Inspect changes against the filesystem, not git.

## Arguments

- `path` (optional): Repository root or any path inside the repository. Default to the current directory.
- `--hint TEXT` (optional): Archive only the section whose heading contains `TEXT` (case-insensitive substring),
  including its subsections. Without it, archive checked tasks from the whole file. Checked tasks outside the matched
  section stay in `TODO.md`.
- `--date YYYY-MM-DD|YYYY_MM_DD` (optional): Archive date. Default to today's local date.
- `--dry-run` (optional): Preview target paths and rendered content without writing.
- `--force` (optional): Overwrite the date-only archive in place instead of rolling a same-day re-run over to a
  timestamped file.

## Workflow

1. Resolve the repository root:

   ```sh
   git rev-parse --show-toplevel
   ```

   If the command fails, use the provided `path` or current directory as the root.

2. Verify `TODO.md` exists at the root. If it is missing, stop and report the path checked.

3. Run the helper from this skill directory:

   ```sh
   uv run python scripts/archive_todo.py --root "$repo_root"
   ```

   Pass through `--hint`, `--date`, `--dry-run`, or `--force` when the user requested them.

4. Report the rewritten `TODO.md`, the created archive path, the matched section (when `--hint` was given), and the
   archived/remaining task counts. If an archive for the date already exists, the helper rolls the new batch over to
   `TODO_UNTIL_YYYY_MM_DD_HHMM.md` and keeps the earlier file; report both paths. If the helper reports no checked
   tasks, treat it as a no-op. If `--hint` matches no heading, the helper exits non-zero and lists the available
   sections; relay them. When `--force` replaces an existing archive, say `⚠️ Overwrote existing archive`, never
   `Created`.

5. If useful, inspect only the touched paths. `TODO.md` and `.ai/` are git-ignored, so use the filesystem rather than
   `git diff`:

   ```sh
   cat TODO.md && ls .ai/todos/
   ```

## Helper Behavior

`scripts/archive_todo.py` reads only `<root>/TODO.md`, writes archived tasks to
`<root>/.ai/todos/TODO_UNTIL_YYYY_MM_DD.md`, and rewrites `<root>/TODO.md` with the remaining tasks (a minimal `# TODO`
stub if everything was archived). With `--hint`, it restricts archiving to the matched heading's subtree and exits
non-zero listing available headings when nothing matches. It rolls a same-day re-run over to a timestamped
`TODO_UNTIL_YYYY_MM_DD_HHMM.md` sibling instead of clobbering the earlier archive, unless `--force` is passed.

## Completion

Completion evidence is the helper's archive path plus archived and remaining task counts; a no-checked-task result is a
successful no-op. Dry-run completion requires rendered paths/content with no filesystem changes. Finish with
`### 📦 Archived <count> checked tasks` and concise TODO/archive/scope/count rows; use
`### ✅ Nothing to archive — no files written` for a no-op and `### 🔎 Archive preview — no files written` for dry runs.
Do not repeat full dry-run document content in the final message or add decoration to TODO/archive files, paths,
commands, or helper diagnostics.
