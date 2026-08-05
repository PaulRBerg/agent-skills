---
name: publish-skills
description:
  Commit and push attributable catalog changes, reconcile deterministic source-owned installation drift in one guarded
  batch, then commit and push only the global skill paths that actually changed.
---

# Publish Skills

Publish current catalog content and repair every selected source-owned global installation and CLI-lock drift.

## Scope

Default to every candidate reported by `scripts/publish-skills.mjs`. Do not reconstruct a last-published Git boundary
and do not use the current transcript as one.

When the user explicitly names a commit range, resolve the range to commits reachable from the current branch, collect
only paths matching `skills/<name>/...`, validate and de-duplicate the kebab-case names, and pass each name as a
repeated `--skill <name>` filter in every planner, apply, and final-check command. Treat a rename as the old and new
names. Stop if the range or ownership is ambiguous.

## Workflow

### 1. Plan Current Drift

From the source repository, run:

```bash
node scripts/publish-skills.mjs plan --json
```

Append the resolved `--skill` filters only for explicit commit-range mode. The planner is read-only and reports content,
executable-bit, target-layout, symlink, deletion, and CLI metadata drift. If it reports no drift and there are no
attributable source changes to commit, report the no-op and stop.

### 2. Commit and Push Source Changes

If attributable source changes are uncommitted, invoke `$commit --push` from the source repository without `--all` and
include only those paths. `$commit` owns semantic message composition; its `ai-commit` backend owns deterministic
transaction, commit, and push mechanics. If the worktree is clean but `main` is ahead, run `ai-commit push`. A `BEHIND`
receipt means the source is not propagated: stop before changing global installations and report that branch
reconciliation is required. Otherwise, do not change global installations until the source commit and push succeed.

Re-run the same planner command after the push. Record the exact source HEAD:

```bash
git rev-parse HEAD
```

### 3. Apply Once

Run one guarded apply with the recorded full SHA and the same optional filters:

```bash
node scripts/publish-skills.mjs apply --expected-head <full-sha>
```

Do not issue separate `bunx skills` commands or edit the CLI lock. The helper requires clean selected source paths,
`main` equal to its upstream and the expected HEAD, readable v3 lock metadata, and an exclusive process lock. It removes
only deleted or stale entries and target-restriction changes, batches at most one add per target group, verifies the
result, and prints every global path whose final state changed.

If apply fails after partial progress, preserve its completed-command list. Commit and push only its reported repository
paths, then re-plan and retry the remaining drift once with the same expected HEAD. A second failure blocks publication;
report the failed command, completed groups, and changed paths.

### 4. Commit and Push Reported Global Paths

Group `Changed global paths` under `~/.agents` and `~/.claude`. In each repository with a reported diff, invoke
`$commit --push` and pass only the reported skill paths as session-modified paths, including deletions. Never include
unreported skills, unrelated dirty paths, or the CLI state lock. Skip repositories with no reported diff.

### 5. Require a Clean Final Check

Run the same selected scope through:

```bash
node scripts/publish-skills.mjs check
```

Completion requires zero selected content, mode, target, deletion, symlink, or lock-metadata drift and successful pushes
for every source or global repository commit created by this workflow. `BEHIND` is safe noncompletion, not successful
propagation.

## Report

Report the source and global commit receipts, introduced or refreshed names, deleted names, and the final clean-check
result. Omit repositories and target groups that had no changes.
