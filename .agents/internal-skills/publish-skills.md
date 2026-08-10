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
only those paths. `$commit` owns semantic message composition; `ai-commit` owns deterministic commit and push mechanics
— the Commit Workflow Semantics group in `@sync-skills` is the canonical statement of that boundary. If the worktree is
clean but `main` is ahead, run `ai-commit push`. A `BEHIND` receipt is safe noncompletion, never successful propagation:
stop before touching global installations and report that branch reconciliation is required.

### 2. Plan Once, After the Push

```bash
bun run scripts/publish-skills.ts plan --json
```

`head` is the guarded apply SHA — no separate `git rev-parse HEAD` step. If `clean` is true and nothing was committed in
step 1, report the no-op and stop.

Append the resolved `--skill` filters only for explicit commit-range mode (see Scope).

### 3. Claim and Apply

The plan's `repos` array IS the claim set: for each entry, claim exactly its `paths` — `--recursive` for
`scope: "recursive"`, a plain file scope for `scope: "file"` — starting with the entry where `canonical: true`.
`ai-coord` holds one active claim per session, so serialize repos: claim, act, release, next.

Require `READY` before apply. Immediately before apply, run `ai-coord status` from every other listed repo root and
require no intersecting active or queued work — a conflict means wait or re-plan, never apply over contested paths. On a
dirty-settling hold or conflict, use `ai-coord wait` then re-claim; if the plan changed while waiting, release the
obsolete claim and re-plan. The CLI state lock is never a claim path or a commit path.

`repos` already omits shared-skill Claude symlinks that apply cannot mutate — after apply, confirm `~/.claude` shows no
diff for those skills.

```bash
bun run scripts/publish-skills.ts apply --expected-head <head from the plan JSON>
```

Never issue separate `bunx skills` commands or edit the CLI lock. The helper requires clean selected source paths,
`main` equal to its upstream and the expected HEAD, readable v3 lock metadata, and an exclusive process lock; it batches
at most one add per target group, removes only deleted or stale entries, verifies the result, and prints every global
path whose final state changed.

If apply fails after partial progress, preserve its completed-command list. Commit and push only its reported paths,
then re-plan and retry the remainder once with the same expected HEAD. A second failure blocks: report the failed
command, completed groups, and changed paths.

### 4. Commit Reported Global Paths

Group `Changed global paths` by repo (`~/.agents`, `~/.claude`). Commit serially, one claim at a time: first the repo
whose claim is still held from step 3, then each remaining repo with a diff. For each, claim exactly its reported skill
paths (file scope for symlinks) — a dirty-settling hold on paths apply just created is normal, wait it out — then
`$commit --push` with only those paths, then `ai-coord done`. Never claim unreported skills, unrelated dirty paths, or
the CLI state lock. Skip repos with no diff. Hold each claim until that repo's changes are committed and pushed, then
release before claiming the next.

### 5. Final Check

```bash
bun run scripts/publish-skills.ts check
```

with the same `--skill` filters, plus `just readme-skills-check`. Completion requires zero drift, both checks passing,
and every commit created here pushed (`BEHIND` = safe noncompletion, never propagation).

## Report

Report the source and global commit receipts, introduced or refreshed names, deleted names, and the final clean-check
result. Omit repositories and target groups that had no changes.
