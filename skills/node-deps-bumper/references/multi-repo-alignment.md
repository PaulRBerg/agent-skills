# Multi-Repository Alignment

Use when the user names two or more repositories or asks to sync or align dependency versions across repositories. Each
repository keeps its own package manager, range style, age policy, and validation suite. Alignment only chooses shared
targets. Treat an argument that resolves to a directory containing `package.json` as a repository root.

## 1. Plan Every Repository

Save one plan per repository:

```sh
bash <skill-dir>/scripts/run-taze.sh --plan <repo> > <repo-plan.json>
```

The helper applies that repository's own minimum-release-age policy, so each row's `available` is the newest version
that repository admits. Plans list only packages with updates. Also record every direct dependency and catalog entry
from each manifest so packages that are already current still count as shared.

## 2. Respect Holds

Search each repository for documented holds: `overrides`, `resolutions`, or `pnpm.overrides` entries, exact pins, and
guidance, changelog, or code comments explaining a pin or version cap. A hold caps that repository's ceiling or excludes
the package. Never override it silently. Report every hold with its source and effect on the shared target.

## 3. Choose One Shared Target

For every package declared directly in two or more repositories:

- A repository's ceiling is its plan row's `available`, or its current version when its plan has no row.
- The shared target is the highest version every repository's plan allows: the lowest ceiling.
- Never move a repository below its current version. When one repository is already ahead of another's ceiling, leave
  the package unaligned and report why.
- Fixed versions and non-semver protocols stay unchanged unless the user asks otherwise.

Packages found in only one repository follow that repository's plan exactly as in the single-repository workflow.

## 4. Review Majors Once

Present every shared target that crosses a major version in any repository, plus every `review` or unknown row, in one
cross-repository decision batch: package, per-repository current → target, package role, and migration notes. Reuse
explicit approval of those transitions. Ask only about unresolved rows. Never infer major approval from a package name.
A declined major leaves that package unchanged everywhere.

## 5. Apply Per Repository

Run the single-repository baseline (Workflow step 5) in each repository before its first write. Then, from each root:

- Bun catalogs: change catalog definitions, never `catalog:` references in workspace manifests. When the shared target
  differs from the plan's `available`, set that row's `available` to the target in a copy of the saved plan, then run
  `update-bun-catalogs.py` preview and `--write` with that copy.
- Other direct dependencies: when the target equals the plan's `available`, write with
  `run-taze.sh --write --include <packages>`. Otherwise install the target through the repository's package manager in
  the same dependency section with the existing range prefix (for example `bun add -d pkg@^x.y.z`).
- Regenerate the lockfile with the repository's package manager.

## 6. Validate and Commit Per Repository

Rerun each repository's recorded baseline plus the narrowest checks that exercise the updated packages, fixing or
escalating regressions exactly as in Workflow steps 8-9. Commit each repository separately with only its manifest,
catalog, lockfile, and required migration changes, following that repository's commit conventions.

## Report

Use one table: package, shared target, per-repository current → new, and notes. List unaligned packages, declined
majors, and holds with their sources, then per-repository verification and commit receipts.
