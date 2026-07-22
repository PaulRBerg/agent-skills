---
argument-hint: "[--dry-run] [package ...]"
disable-model-invocation: false
effort: medium
model: sonnet
name: bump-deps
user-invocable: true
description: "Use for dependency updates: bump npm/pnpm/yarn/bun packages, check outdated, or run taze."
---

# Bump Dependencies

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

5. Write all selected Taze updates in one command:

   ```sh
   bash <skill-dir>/scripts/run-taze.sh --write --include package-a,package-b
   ```

6. For Bun catalogs, rerun `update-bun-catalogs.py` with the same plan and include set plus `--write`. It atomically
   updates every matching default/named catalog occurrence and preserves each existing `^`, `~`, or empty prefix. Then
   run `ni` so the repository's package manager updates its lockfile.

7. Inspect the manifest and lockfile diff. Run the narrowest package-manager or repository checks that exercise updated
   dependencies, with extra attention to approved major migrations. Also run whatever lint command the repository
   exposes (package script, task runner recipe, or equivalent) — dependency bumps can introduce new lint violations even
   when tests and the build stay green (e.g. an updated linter or plugin adding rules, or a typing change surfacing
   stricter checks).

8. Fix newly flagged lint errors, but judge each one before changing code: a bump can introduce a rule the user may not
   want enabled at all, not just a violation to fix. If a new error's fix is unclear, or fixing it would fight the
   rule's intent rather than follow it, stop with `### ⚠️ Dependency lint decision required`. Show the rule, affected
   locations, likely effects, and the explicit `fix`, `suppress`, or `disable` choices in one table instead of guessing.

## User-Facing Output

Present plans as `### 📦 Dependency plan` with counts and a compact table:

| ID  | Plan value | Decision | Package | Current → target | Type | Notes |
| --- | ---------- | -------- | ------- | ---------------- | ---- | ----- |

Use the plan's exact `apply`, `review-major`, `review`, and `skip-fixed` values alongside plain-language decisions.
Assign stable IDs to rows needing a choice so the user can answer once. Use `### 🔎 Dry run — no files written` for a
preview and `### ✅ No selected updates` for a no-op.

Finish applied work with `### 🏁 Dependencies updated`, a tree of changed manifests/lockfiles, `### 🧪 Verification`,
and `### ⚠️ Remaining review` only when non-empty. Keep helper JSON, package/version strings, commands, and diagnostics
exact and undecorated.

## Invariants

- Fixed versions and non-semver protocols remain unchanged unless the user explicitly asks otherwise.
- Package arguments constrain both scan and write phases.
- The same maturity-period policy applies to scan and write.
- Bun catalog preview and write use the same accepted Taze plan and selected package set; stale plans never write.
- Do not infer compatibility from SemVer alone when repository evidence, peer ranges, or release notes indicate
  otherwise.

Completion requires a reviewed plan, one manifest write for the selected set, a regenerated lockfile, and validation
evidence; dry-run completion requires the structured plan and zero writes.
