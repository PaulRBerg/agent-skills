---
argument-hint: "[--subject <line>] [--base <branch>]"
disable-model-invocation: true
effort: high
name: git-squash
user-invocable: true
description: "Squash a feature branch into one commit via soft reset to the merge base, ready for a clean PR."
---

# Git Squash

Squash the current feature branch into one commit representing its net change relative to the resolved default branch.

## Arguments

- `--subject <line>`: require this exact first line in the replacement commit message.
- `--base <branch>`: override default-branch detection.

Without `--subject`, the agent writes a conventional-commit subject from the surviving net diff. Without `--base`, the
helper resolves `origin/HEAD`, then origin's advertised head, then local/remote `main`, `master`, or `trunk`.

## Plan Interface

Resolve the helper from this `SKILL.md`. Write its JSON to a scratch path outside the repository so preflight remains
clean:

```sh
uv run "<skill-dir>/scripts/git-squash.py" plan \
  [--cwd <repo>] [--base <branch>] [--subject <line>] > <plan.json>
```

`plan` is read-only. It verifies the Git worktree, attached branch, clean tree/index, resolved base, non-default current
branch, merge base, positive ahead count, and remote facts. It returns `schemaVersion: 1`, the immutable original HEAD,
merge base, base ref, commits in chronological order, unique authors, tree/remote state, and rollback facts. A failed
precondition exits without changing history.

Show the branch, base ref, merge base, commits replaced, tree state, remote state, and rollback HEAD in a compact plain
table before mutation.

## Agent-Owned Commit Message

Inspect the plan's commits and the net diff from `mergeBase..originalHead`. The net diff is authoritative; intermediate
commits supply intent and attribution only. Inspect targeted hunks when the summary is ambiguous.

Use `--subject` exactly when supplied. Otherwise choose the conventional type from the surviving outcome: `feat`, `fix`,
`refactor`, `docs`, `test`, `build`, `ci`, `chore(deps)`, `style`, `perf`, `ai`, or `chore`. Do not call the change
`chore` merely because it is a squash.

Keep the subject imperative, lowercase after the prefix, specific, and without a trailing period. Add at most five body
bullets for distinct surviving outcomes; omit the body when the subject is sufficient. Do not dump paths or statistics.
Append one `Co-authored-by: Name <email>` trailer for each plan author other than the current Git user. The agent owns
all semantic wording and must ensure every statement is supported by the net diff.

Write the final message to a scratch file outside the repository.

## Apply Interface

```sh
uv run "<skill-dir>/scripts/git-squash.py" apply \
  --plan <plan.json> --message-file <message.txt>
```

`apply` binds the rewrite to the plan's original HEAD, branch, merge base, base ref, clean state, rollback index, and
optional subject. It revalidates them immediately before mutation; a stale plan fails without changing history. It
soft-resets to the merge base, verifies the staged net diff is non-empty, and commits from the message file.

If any operation fails after mutation but before the replacement commit completes, the helper restores the original HEAD
and exact index. It leaves working-tree files untouched. Do not reproduce the reset/rollback sequence manually or
continue after a helper failure without inspecting its diagnostic and current Git state.

## Report

On success, report the plan's replaced count, resolved base ref, new hash, and subject. If the branch exists on origin,
state the exact next action `git push --force-with-lease`; do not run it unless explicitly requested.

Lead with `### ✅ Squashed — <old count> commits → 1`. Keep preflight facts, hashes, commands, errors, and rollback
wording plain and exact.
