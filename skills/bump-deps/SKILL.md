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

1. Resolve the skill directory and run the helper from the target repository:

   ```sh
   bash <skill-dir>/scripts/run-taze.sh --plan [--include package-a,package-b]
   ```

   The JSON plan classifies every discovered update as `apply`, `review-major`, `review`, or `skip-fixed`. The helper
   detects monorepos, includes locked versions during scans, and mirrors Bun minimum-release-age settings. If the
   repository uses package-manager age gates or Bun catalogs, read
   [references/conditional-workflows.md](references/conditional-workflows.md) for that active branch only.

2. If `--dry-run` was requested, present the plan and counts, then stop without changing manifests or lockfiles.

3. Select every ranged minor/patch update marked `apply`. Never auto-approve a major package by name. Present all
   `review-major` and unknown updates in one decision batch with current version, target version, package role when
   discoverable, and relevant migration/release notes. Apply only the majors the user selects.

4. If nothing is selected, report the no-op and stop. Otherwise write all selected updates in one command:

   ```sh
   bash <skill-dir>/scripts/run-taze.sh --write --include package-a,package-b
   ```

5. Update matching root Bun catalog entries when present, preserving their existing range prefixes. Then run `ni` so the
   repository's package manager updates its lockfile.

6. Inspect the manifest and lockfile diff. Run the narrowest package-manager or repository checks that exercise updated
   dependencies, with extra attention to approved major migrations.

## Invariants

- Fixed versions and non-semver protocols remain unchanged unless the user explicitly asks otherwise.
- Package arguments constrain both scan and write phases.
- The same maturity-period policy applies to scan and write.
- Do not infer compatibility from SemVer alone when repository evidence, peer ranges, or release notes indicate
  otherwise.

Completion requires a reviewed plan, one manifest write for the selected set, a regenerated lockfile, and validation
evidence; dry-run completion requires the structured plan and zero writes.
