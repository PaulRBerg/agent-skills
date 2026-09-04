---
name: publish-skills
description:
  Commit and push attributable catalog changes, reconcile deterministic source-owned installation drift in one guarded
  batch, then commit and push only the global skill paths that actually changed.
---

# Publish Skills

Publish current catalog content and repair every selected source-owned global installation and CLI-lock drift.

## Scope

Default to every candidate reported by `scripts/publish-skills.ts`. Do not reconstruct a last-published Git boundary and
do not use the current transcript as one.

When the user explicitly names a commit range, resolve it to commits reachable from the current branch, collect only
paths matching `skills/<name>/...`, validate and de-duplicate the kebab-case names, and pass each as a repeated
`--skill <name>` filter on every planner, apply, and check command. A rename contributes both the old and new names
(details: `@skill-lifecycle`). Stop if the range or ownership is ambiguous.

## Workflow

### 1. Commit and Push Source

If attributable source changes are uncommitted, run `$commit --push` from the source repository without `--all`, passing
only those paths. If the worktree is clean but `main` is ahead, run `ai-commit push`. On a `BEHIND` receipt, stop before
touching global installations and report that branch reconciliation is required.

Keep this work under the source-repository claim through its commit and push, then run `ai-coord done` for that claim
before acquiring the target bundle.

### 2. Plan Once, After the Push

```bash
bun run scripts/publish-skills.ts plan --json
```

Require planner JSON `version: 2`; retain its `repos` records and `canonical` field unchanged. `head` is the guarded
apply SHA — no separate `git rev-parse HEAD` step. If `clean` is true and nothing was committed in step 1, report the
no-op and stop.

Append the resolved `--skill` filters only for explicit commit-range mode (see Scope).

### 3. Acquire Every Target, Then Apply

The plan's `repos` array IS the claim set. Resolve every reported `root` to its canonical physical string with
`cd <root> && pwd -P`, then combine each root with its reported `paths` entries. Submit every resulting absolute path in
one `ai-coord bundle start 'publish catalog skills'` command: use repeated `--recursive '<absolute-dir>'` arguments for
`scope: "recursive"` and plain `'<absolute-file>'` arguments for `scope: "file"`. `canonical` is informational; include
every reported repository. Do not acquire roots with separate `ai-coord start` calls.

Require `READY` for the complete bundle before apply. On blocked, dirty-settling, or unknown coverage, run
`ai-coord wait`, then resubmit the full bundle after each wake; a wake is not authorization. Re-plan only if the source
`HEAD` or the planned mutation paths changed while waiting, then submit the complete updated bundle. Do not apply over
contested paths. The CLI process/state lock is outside repository coordination and commits: never claim or commit it.

`repos` already omits shared-skill Claude symlinks that apply cannot mutate — after apply, confirm `~/.claude` shows no
diff for those skills.

```bash
bun run scripts/publish-skills.ts apply --expected-head <head from the plan JSON>
```

Never issue separate `bunx skills` commands or edit the CLI lock. The helper requires clean selected source paths,
`main` equal to its upstream and the expected HEAD, readable v3 lock metadata, and an exclusive process lock; it batches
at most one add per target group, removes only deleted or stale entries, verifies the result, and prints every global
path whose final state changed.

If apply fails after partial progress, preserve its completed-command list and retain the complete target bundle. Commit
and push only its reported paths, then re-plan and retry the remainder once with the same expected HEAD. A second
failure blocks: report the failed command, completed groups, and changed paths.

### 4. Commit Reported Global Paths and Release Claims

Group `Changed global paths` by reported repo root. Retain the complete bundle acquired in step 3 through every target
commit and push; never perform a post-apply `start`. For each repo with reported changed paths, commit and push only
those paths. For a repo with no reported diff, confirm its planned paths have no diff. Once every target's changes are
pushed or verified absent, run `ai-coord done` once to release the entire bundle; it does not release only the current
repository. Never claim unreported skills, unrelated dirty paths, or the CLI process/state lock. A dirty-settling result
on a reported publisher-written path is a regression, not expected waiting: preserve the bundle and stop with the
evidence.

### 5. Final Check

```bash
bun run scripts/publish-skills.ts check
```

with the same `--skill` filters, plus `just readme-skills-check`. Completion requires zero drift, both checks passing,
and every commit created here pushed.

## Report

Report the source and global commit receipts, introduced or refreshed names, deleted names, and the final clean-check
result. Omit repositories and target groups that had no changes.
