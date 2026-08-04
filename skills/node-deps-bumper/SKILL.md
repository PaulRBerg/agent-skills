---
argument-hint: "[--dry-run] [package ...]"
disable-model-invocation: false
effort: medium
model: sonnet
name: node-deps-bumper
user-invocable: true
description: "Use for dependency updates: bump npm/pnpm/yarn/bun packages, check outdated, or run taze."
---

# Node Dependency Bumper

Use Taze to build one structured update plan, apply compatible ranged updates, and make major-version decisions as a
batch.

## Workflow

1. Resolve the skill directory and save the helper plan from the target repository:

   ```sh
   bash <skill-dir>/scripts/run-taze.sh --plan [--include package-a,package-b] > <taze-plan.json>
   ```

   The JSON plan classifies every discovered update as `apply`, `review-major`, `review`, or `skip-fixed`. The helper
   detects monorepos, includes locked versions during scans, and mirrors Bun minimum-release-age settings. If the
   repository uses package-manager age gates or Bun catalogs, read
   [references/conditional-workflows.md](references/conditional-workflows.md) for that active branch only.

2. If `--dry-run` was requested, present the plan and counts, then stop without changing manifests or lockfiles.

3. Select every ranged minor/patch update marked `apply`. Never auto-approve a major package by name. Present all
   `review-major` and unknown updates in one decision batch with current version, target version, package role when
   discoverable, and relevant migration/release notes. Apply only the majors the user selects.

4. If nothing is selected, report the no-op and stop. If the root manifest uses Bun catalogs, preview the exact selected
   catalog transitions from the accepted plan:

   ```sh
   uv run <skill-dir>/scripts/update-bun-catalogs.py \
     --root <repo> --plan <taze-plan.json> --include package-a,package-b
   ```

   The preview is read-only. Missing catalog entries, conflicting plan rows, unsupported versions, or a catalog value
   that no longer matches the plan fail before writes. The helper does not select upgrades.

5. Before the first manifest or lockfile write, discover and run the repository's standard validation suite against the
   existing dependency state. Prefer its advertised aggregate check; otherwise run every exposed dependency-resolution,
   build, test, typecheck, lint, formatting-check, codegen-check, and repository-invariant command. Use frozen or
   non-writing modes where available, and record the exact commands for the post-bump rerun. Command failures,
   dependency or peer-resolution conflicts, and actionable warnings that signal incompatibility or unsafe behavior block
   the bump; informational notices such as unavoidable deprecations do not. If the baseline has any blocking issue, stop
   with `### ⛔ Dependency bump blocked — baseline failed`, show the exact commands and diagnostics, and ask the user to
   resolve the pre-existing issues. Do not modify manifests, lockfiles, source, or configuration.

6. Write all selected Taze updates in one command:

   ```sh
   bash <skill-dir>/scripts/run-taze.sh --write --include package-a,package-b
   ```

7. For Bun catalogs, rerun `update-bun-catalogs.py` with the same plan and include set plus `--write`. It atomically
   updates every matching default/named catalog occurrence and preserves each existing `^`, `~`, or empty prefix. Then
   run `ni` so the repository's package manager updates its lockfile.

8. Inspect the manifest and lockfile diff. Rerun the exact baseline commands, plus the narrowest checks that exercise
   the updated dependencies and any required migrations. Treat every newly introduced error, type issue, check failure
   (including tests, builds, lint, formatting, codegen, and repository invariants), dependency or peer-resolution
   conflict, and actionable compatibility or safety warning as caused by the bump unless evidence shows otherwise.

9. Fix every issue caused by the bump, including required source or configuration migrations, while preserving intended
   behavior. Do not suppress diagnostics, weaken validation, or change expected behavior merely to make checks pass.
   After each fix, rerun the affected check, then rerun the complete recorded suite until it passes. If no clear safe
   fix exists within the task's authority, stop with `### ⚠️ Dependency regression decision required`. Present all such
   issues in one table with the evidence, affected locations, fix and revert options, and likely effects. Do not report
   completion until the user chooses, the fix is applied or the offending update is reverted, the lockfile is
   regenerated, and the complete suite passes.

## User-Facing Output

Present plans as `### 📦 Dependency plan` with counts and a compact table:

| ID  | Plan value | Decision | Package | Current → target | Type | Notes |
| --- | ---------- | -------- | ------- | ---------------- | ---- | ----- |

Use the plan's exact `apply`, `review-major`, `review`, and `skip-fixed` values alongside plain-language decisions.
Assign stable IDs to rows needing a choice so the user can answer once. Use `### 🔎 Dry run — no files written` for a
preview and `### ✅ No selected updates` for a no-op.

Finish applied work with `### 🏁 Dependencies updated`, a tree of changed manifests/lockfiles, and
`### 🧪 Verification`. Use `### ⚠️ Remaining review` only for non-blocking informational matters, never for an
unresolved issue caused by the bump. Keep helper JSON, package/version strings, commands, and diagnostics exact and
undecorated.

## Invariants

- Fixed versions and non-semver protocols remain unchanged unless the user explicitly asks otherwise.
- Package arguments constrain both scan and write phases.
- The same maturity-period policy applies to scan and write.
- Bun catalog preview and write use the same accepted Taze plan and selected package set; stale plans never write.
- Do not infer compatibility from SemVer alone when repository evidence, peer ranges, or release notes indicate
  otherwise.

Completion requires a clean pre-write baseline, a reviewed plan, the retained selected updates, a regenerated lockfile,
a passing rerun of the recorded suite and dependency-specific checks, and no unresolved issue caused by the bump.
Dry-run completion requires the structured plan and zero writes.
