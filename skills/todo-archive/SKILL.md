---
argument-hint: "[path] [--hint TEXT] [--date YYYY-MM-DD|YYYY_MM_DD] [--dry-run] [--force]"
disable-model-invocation: false
name: todo-archive
user-invocable: true
description: Use only when explicitly asked to archive/prune/compact/roll over checked tasks from TODO.md into `.ai/todos/TODO_UNTIL_YYYY_MM_DD.md`, leaving unchecked tasks.
---

# TODO Archive

Archive checked Markdown task-list items from `TODO.md` into `.ai/todos/TODO_UNTIL_YYYY_MM_DD.md`, then leave only unchecked tasks in `TODO.md`.

`TODO.md` and `.ai/` are conventionally git-ignored, so they are untracked and `git diff` shows nothing for them. Inspect changes against the filesystem, not git.

## Arguments

- `path` (optional): Repository root or any path inside the repository. Default to the current directory.
- `--hint TEXT` (optional): Archive only the section whose heading contains `TEXT` (case-insensitive substring), including its subsections. Without it, archive checked tasks from the whole file. Checked tasks outside the matched section stay in `TODO.md`.
- `--date YYYY-MM-DD|YYYY_MM_DD` (optional): Archive date. Default to today's local date.
- `--dry-run` (optional): Preview target paths and rendered content without writing.
- `--force` (optional): Overwrite the date-only archive in place instead of rolling a same-day re-run over to a timestamped file.

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

4. Report the rewritten `TODO.md`, the created archive path, the matched section (when `--hint` was given), and the archived/remaining task counts. If an archive for the date already exists, the helper rolls the new batch over to `TODO_UNTIL_YYYY_MM_DD_HHMM.md` and keeps the earlier file; report both paths. If the helper reports no checked tasks, treat it as a no-op. If `--hint` matches no heading, the helper exits non-zero and lists the available sections; relay them.

5. If useful, inspect only the touched paths. `TODO.md` and `.ai/` are git-ignored, so use the filesystem rather than `git diff`:

   ```sh
   cat TODO.md && ls .ai/todos/
   ```

## Helper Behavior

`scripts/archive_todo.py`:

- Reads only `<root>/TODO.md`.
- Writes checked tasks to `<root>/.ai/todos/TODO_UNTIL_YYYY_MM_DD.md`.
- Rewrites `<root>/TODO.md` with the tasks that were not archived.
- With `--hint`, restricts archiving to the subtree of any heading whose text contains the hint (case-insensitive); checked tasks elsewhere are left in place. Exits non-zero and lists available section headings when nothing matches.
- Preserves non-task content inside any heading subtree that still has tasks for the output.
- Drops headings whose subtree has no tasks for the output.
- Keeps nested parent items as plain bullets when only descendants match the output, avoiding orphaned subtasks without copying the wrong checkbox state.
- Leaves a minimal `# TODO` stub when every task was archived.
- Rolls a same-day re-run over to a timestamped `TODO_UNTIL_YYYY_MM_DD_HHMM.md` sibling (appending a counter on a same-minute collision) so an earlier batch is never clobbered.
- Overwrites the date-only archive in place only when `--force` is passed.
