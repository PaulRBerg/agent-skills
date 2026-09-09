---
argument-hint: "[path] [--hint TEXT] [--date YYYY-MM-DD|YYYY_MM_DD] [--dry-run]"
disable-model-invocation: true
effort: low
model: sonnet
name: todo-archive
description: Archive checked TODO.md tasks into `.ai/todos/YYYY-MM/DD.md`, leaving unchecked tasks.
---

# TODO Archive

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

## Arguments

- `path` (optional): Repository root or any path inside the repository. Default to the current directory.
- `--hint TEXT` (optional): Archive only the section whose heading contains `TEXT` (case-insensitive substring),
  including its subsections. Without it, archive checked tasks from the whole file. Checked tasks outside the matched
  section stay in `TODO.md`.
- `--date YYYY-MM-DD|YYYY_MM_DD` (optional): Archive date. Default to today's local date.
- `--dry-run` (optional): Preview target paths and rendered content without writing.

## Workflow

1. Resolve the supplied `path`, or the current directory when omitted. For an existing file, use its containing
   directory; for a directory, use that directory. Store the absolute directory as `start_dir`, then resolve its Git
   root:

   ```sh
   git -C "$start_dir" rev-parse --show-toplevel
   ```

   Store the result as `repo_root`. When `start_dir` is outside a Git repository, use it as `repo_root`. Stop on a
   missing input path or another resolution error.

2. Verify `TODO.md` exists at the root. If it is missing, stop and report the path checked.

3. Resolve the skill directory and run its helper:

   ```sh
   uv run python "<skill-dir>/scripts/archive_todo.py" --root "$repo_root"
   ```

   Pass through `--hint`, `--date`, or `--dry-run` when the user requested them.

4. Report the rewritten `TODO.md`, the created or merged archive path, the matched section (when `--hint` was given),
   and the archived/remaining task counts. If an archive for the date already exists, the helper appends the new batch
   to it, retaining one matching top-level heading. If the helper reports no checked tasks, treat it as a no-op. If
   `--hint` matches no heading, the helper exits non-zero and lists the available sections; relay them.

5. If useful, inspect only `<repo_root>/TODO.md` and the exact archive path returned by the helper. Verify their
   contents directly; when Git ignores them, compare filesystem snapshots rather than relying on `git diff`.

## Helper Behavior

`scripts/archive_todo.py` reads only `<root>/TODO.md`, writes archived tasks to `<root>/.ai/todos/YYYY-MM/DD.md`, and
rewrites `<root>/TODO.md` with the remaining tasks. It preserves task-free sections and prose verbatim (a minimal
`# TODO` stub only if everything was archived). With `--hint`, it restricts archiving to the matched heading's subtree
and exits non-zero listing available headings when nothing matches. A same-day re-run appends its batch to that day's
file, removing the new leading H1 only when it exactly matches the existing archive's leading H1.

## Completion

Completion evidence is the helper's archive path plus archived and remaining task counts; a no-checked-task result is a
successful no-op. Dry-run completion requires rendered paths/content with no filesystem changes, but the final message
still follows the one-line formats below. Report the result as a single line (append a `(scope: "<hint>")` segment only
when `--hint` was given), with the archive path relative to the repository root:

- Success: `📦 Archived <n> → <archive path> (created|merged) · <remaining> remaining`
- No-op: `✅ Nothing to archive · <remaining> remaining`
- Dry run: `🔎 Would archive <n> → <archive path> (would create|would merge) · <remaining> would remain`

Do not repeat full dry-run document content in the final message or add decoration to TODO/archive files, paths,
commands, or helper diagnostics.
