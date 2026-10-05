---
name: publish-skills
description:
  Commit and push attributable catalog changes. Reconcile deterministic source-owned installation drift in one guarded
  batch. Then commit and push only the global skill paths that actually changed.
---

# Publish Skills

Publish current catalog content. Repair every selected source-owned global installation and CLI-lock drift.

## Scope

For standalone publication, default to every candidate reported by `scripts/publish-skills.ts`. Do not reconstruct a
last-published Git boundary or use the current transcript as one. Instead, use the first applicable scoped mode:

- When the user explicitly names a commit range, select that range.
- For publication triggered by continuous skill maintenance, retain the exact source commit receipts for the completed
  repairs. Select only skill paths attributable to that maintenance. If only the installation is stale, use the commit
  containing the verified source correction. In that case, restrict selection to the affected skill.

For either scoped mode, resolve the selected commits as reachable from the current branch. Collect paths matching
`skills/<name>/...`. Validate the kebab-case names. Remove duplicate names. Pass each name as a repeated
`--skill <name>` filter on every planner, apply, and check command.

A rename contributes both the old and new names. See `@skill-lifecycle` for details. If the selection or ownership is
ambiguous, stop. An empty scoped selection is a no-op. Never replace it with full-drift publication.

## Workflow

### 1. Commit and Push Source

If attributable source changes are uncommitted, run `$commit --push` from the source repository without `--all`. Pass
only those paths. If selected source changes are already committed, run `ai-commit push` to verify propagation, even
when unrelated paths are dirty.

On a `BEHIND` receipt, resolve the branch state before touching global installations:

- If the working tree and index are clean and no other Git operation is in progress, fetch. Verify those conditions.
  Then run a conflict-free `git pull --rebase --no-autostash`. Rerun `ai-commit push`.
- If that rebase encounters conflicts, abort only that rebase. Ask before resolving the conflicts.
- If the tree or index is dirty, stop. Report that branch reconciliation is required.

Never autostash.

Keep this work under the source-repository claim through its commit and push. Then run `ai-coord done` for that claim
before step 2 acquires the target claims.

### 2. Plan, Claim, and Apply

Append the resolved `--skill` filters for either scoped mode (see Scope):

```bash
just publish-skills [--skill <name>]...
```

The recipe plans once with `scripts/publish-skills.ts plan --json`. It requires planner JSON `version: 2`. It prints the
plan `head`, which is the guarded apply SHA. When `clean` is true, the recipe exits 0 without claiming. In that case,
skip to step 4. Report any source commit or the no-op.

Otherwise, the plan's `repos` array is the claim set. The recipe resolves every reported `root` to its canonical
physical path. It keeps every reported `paths` entry. It chooses the acquisition by the number of distinct canonical
roots:

- One root: `ai-coord start 'publish catalog skills'` from that root with repository-relative scopes.
- Two or more roots: one `ai-coord bundle start 'publish catalog skills'` with absolute scopes.
- No roots: no repository claim. A non-clean plan may only need CLI metadata cleanup under the helper's process lock.

`scope: "recursive"` entries become `--recursive` arguments. `scope: "file"` entries become plain paths. The recipe runs
`apply --expected-head <head>` with the same filters only after acquisition exits 0 and reports `READY`.

When it prints `Target claims are not READY`, run `ai-coord wait`. Then rerun `just publish-skills` after each wake. A
wake is not authorization. The rerun creates a new plan and resubmits the complete claim set. Do not apply over
contested paths.

The CLI process/state lock is outside repository coordination and commits. Never claim or commit it.

`repos` already omits shared-skill Claude symlinks that apply cannot mutate. After apply, confirm `~/.claude` shows no
diff for those skills.

Never issue separate `bunx skills` commands or edit the CLI lock. The helper pins the `skills` CLI version. It requires
clean selected source paths, `main` equal to its upstream and the expected HEAD, readable v3 lock metadata, and an
exclusive process lock.

The helper batches at most one add per target group. It removes only deleted or stale entries. It verifies the result.
It prints every global path whose final state changed.

If apply fails after partial progress, preserve its completed-command list. Retain all target claims. Commit and push
only its reported paths. Then create a new plan. Retry the remainder once under the held claims with
`bun run scripts/publish-skills.ts apply --expected-head <printed plan head>` plus the same filters.

A second failure blocks the work. Report the failed command, completed groups, and changed paths.

### 3. Commit Reported Global Paths and Release Claims

Group `Changed global paths` by reported repo root. Before preparation, expand reported directories to individual
repository-relative file paths for attributable publication changes, including new and deleted files. Exclude unrelated
files within those directories.

Retain all claims acquired in step 2 through every target commit and push. Never perform a post-apply `start`. For each
repo with reported changed paths, commit and push only those file paths. For a repo with no reported diff, confirm its
planned paths have no diff.

Once every target's changes are pushed or verified absent, run `ai-coord done` once from a claimed target repository, if
claims were acquired. For a bundle, this releases every target, not only the current repository. Never claim unreported
skills, unrelated dirty paths, or the CLI process/state lock.

A dirty-settling result on a reported publisher-written path is a regression, not expected waiting. In that case,
preserve the claims. Stop with the evidence.

### 4. Final Check

```bash
bun run scripts/publish-skills.ts check
```

Use the same `--skill` filters for this command. Also run `just skill-check`. Completion requires zero drift, both
checks passing, and every commit created here pushed.

## Report

Report the source and global commit receipts, introduced or refreshed names, deleted names, and the final clean-check
result. Omit repositories and target groups that had no changes.
