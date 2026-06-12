---
argument-hint: "[path] [--date YYYY-MM-DD|YYYY_MM_DD] [--dry-run] [--force]"
disable-model-invocation: true
name: todo-archive
user-invocable: true
description: Use only when explicitly asked to archive/prune/compact/roll over checked tasks from TODO.md into `.ai/todos/TODO_UNTIL_YYYY_MM_DD.md`, leaving unchecked tasks.
---

# TODO Archive

Archive checked Markdown task-list items from `TODO.md` into `.ai/todos/TODO_UNTIL_YYYY_MM_DD.md`, then leave only unchecked tasks in `TODO.md`.

## Arguments

- `path` (optional): Repository root or any path inside the repository. Default to the current directory.
- `--date YYYY-MM-DD|YYYY_MM_DD` (optional): Archive date. Default to today's local date.
- `--dry-run` (optional): Preview target paths and rendered content without writing.
- `--force` (optional): Allow overwriting an existing archive file.

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

   Pass through `--date`, `--dry-run`, or `--force` when the user requested them.

4. Report the rewritten `TODO.md`, the created archive path, and the checked/unchecked task counts. If the helper reports no checked tasks, treat it as a no-op.

5. If useful, inspect only the touched paths:

   ```sh
   git diff -- TODO.md .ai/todos/
   ```

## Helper Behavior

`scripts/archive_todo.py`:

- Reads only `<root>/TODO.md`.
- Writes checked tasks to `<root>/.ai/todos/TODO_UNTIL_YYYY_MM_DD.md`.
- Rewrites `<root>/TODO.md` with unchecked tasks.
- Preserves non-task content inside any heading subtree that still has tasks for the output.
- Drops headings whose subtree has no tasks for the output.
- Keeps nested parent items as plain bullets when only descendants match the output, avoiding orphaned subtasks without copying the wrong checkbox state.
- Leaves a minimal `# TODO` stub when every task was archived.
- Refuses to overwrite an existing archive unless `--force` is passed.
