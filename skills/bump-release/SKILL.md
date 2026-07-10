---
argument-hint: "[packages...] [version] [--beta] [--dry-run]"
disable-model-invocation: true
effort: high
model: sonnet
name: bump-release
user-invocable: true
description: "Cut a release: bump versions, write changelogs, commit, tag."
---

# Bump Release

Release one package or several packages with version bumps, changelog entries, commits, and tags. Supports
single-package repos, workspace monorepos, regular releases, beta releases, and dry runs.

## Arguments

- `packages`: Optional monorepo package names or directories, such as `evm` or `evm-safe`. Omit in single-package repos.
- `version`: Optional explicit semver, such as `2.0.0`. Only valid for one target package.
- `--beta`: Create or advance a `-beta.X` prerelease.
- `--dry-run`: Preview the release plan without modifying files, committing, or tagging.

## Fast Planner

Run the bundled planner before manual inspection. It is read-only and gives one JSON fact base for package discovery,
previous tags, scoped changed files, dependency edges, and dirty-tree status. In pnpm workspaces, it uses
`pnpm list -r --depth -1 --json` when available and falls back to local workspace-glob discovery. Bun and npm-style
`package.json` workspaces use the local glob discovery, including negative workspace patterns.

Resolve `<skill-dir>` from the loaded `SKILL.md` path:

```sh
node "<skill-dir>/scripts/plan-release.mjs" [--cwd <repo>] [--beta] [--dry-run] [--version <semver>] [--package <name-or-dir>]...
```

Map user arguments directly:

- Pass every package selector as `--package <selector>`.
- Pass an explicit version as `--version <semver>`.
- Pass `--beta` and `--dry-run` when requested.

If the helper exits `2`, stop: the cwd is not a git repo or has no root `package.json`. If it exits `64`, read the JSON
`errors` when present, report the invalid arguments, and stop.

## Workflow

1.  **Run the planner** - Use the JSON output as the source of truth for `mode`, `packages`, `targets`, `previousTags`,
    `changedFiles`, `includedFiles`, `excludedFiles`, `dependencyEdges`, `needsSelection`, and `workingTree`.
2.  **Require a clean tree** - If `workingTree.clean` is false, stop and show the short status. Do not invoke the
    `commit` skill or commit unrelated work unless the user explicitly asks.
3.  **Resolve targets** - If `needsSelection` is true, ask the user which workspace packages to release. If package
    selectors are unknown or ambiguous, stop and ask for exact package names or directories.
4.  **Reject invalid version scope** - If an explicit `version` was supplied for more than one target package, stop.
    Explicit versions are single-package only.
5.  **Plan versions** - Determine a candidate version for each target package. For explicit versions, beta suffixing,
    and prerelease transitions, follow the Version Examples table below. For a regular release from a stable version
    with no explicit version, inspect relevant net changes and choose patch, minor, or major by Semantic Versioning.
6.  **Skip no-op releases** - For regular releases, if a target has no `includedFiles` and no dependency-range cascade,
    report that there are no relevant release changes and do not bump it.
7.  **Cascade dependents** - Use `dependencyEdges` to find workspace packages whose `dependencies` or `peerDependencies`
    point at bumped packages. Check ranges with a structured semver parser or package manager API when available, not ad
    hoc string comparison. If the new version is outside the declared range, update the range and add the dependent to
    the release plan. Treat dependency range widening as patch by default; treat peer dependency major changes as major
    unless the user confirms otherwise.
8.  **Confirm inferred versions** - For non-dry-run regular releases without explicit versions, ask the user to confirm
    inferred versions. For multi-package releases, include requested packages and cascaded dependents in the same
    release-plan confirmation when the agent UI allows it.
9.  **Preview dry runs** - For `--dry-run`, print the package order, current versions, planned versions,
    changelog/tag/commit actions, dependency range updates, and skipped files. Stop before edits.
10. **Write changelogs** - For regular releases only, read `references/common-changelog.md` after the final package set
    is known and apply it as the authoritative changelog contract. Bound diff inspection per target using its
    `includedFiles` against `previousTags[package].tag`.
11. **Edit release files** - Update each target package's `CHANGELOG.md` and `package.json`. For beta releases, skip
    `CHANGELOG.md`. Update any cascaded dependent ranges before committing the dependent release.
12. **Format once** - After all release edits, run formatting once. If a `justfile` exists, inspect `just --list` and
    prefer the narrowest relevant write recipe; use broad recipes such as `just full-write` only when no narrower
    established recipe covers the touched files. Without a suitable recipe, use the repo's established formatter
    commands or leave formatting unchanged.
13. **Commit and tag in dependency order** - Process dependencies before dependents. Use one commit and one annotated
    tag per package:
    - Single-package commit: `docs: release <version>`
    - Monorepo commit: `docs: release <package> <version>`
    - Single-package tag: `v<version>` unless existing tags use bare semver.
    - Monorepo tag: follow existing tag patterns from `previousTags`; default to `<package-dir>@<version>`.

## Script Reference

| Script                     | Purpose                                           |
| -------------------------- | ------------------------------------------------- |
| `scripts/plan-release.mjs` | Read-only release discovery and scoped diff facts |

Planner output fields to use:

- `packages` and `targets`: package identity, directory, name, version, `files`, dependency names, and peer dependency
  names.
- `previousTags`: per-package previous release tag and tag patterns used.
- `changedFiles`, `includedFiles`, `excludedFiles`: scoped file lists for changelog and no-op decisions.
- `dependencyEdges`: workspace dependency and peer dependency relationships.
- `workingTree`: dirty-tree status that must be clean before release edits.
- `needsSelection` and `errors`: package-selection or argument problems to resolve before proceeding.

## Version Examples

| Current Version | Release Type   | New Version     |
| --------------- | -------------- | --------------- |
| `1.2.3`         | Regular        | `1.2.4` (patch) |
| `1.2.3`         | Beta           | `1.2.4-beta.1`  |
| `1.2.3-beta.1`  | Beta           | `1.2.3-beta.2`  |
| `1.2.3-beta.5`  | Regular        | `1.2.3`         |
| `1.2.3`         | `2.0.0`        | `2.0.0`         |
| `1.2.3`         | `2.0.0` + Beta | `2.0.0-beta.1`  |

## Resources

- `references/common-changelog.md` - Read only for regular releases after the final stable release package set is known.

## Completion

Dry-run completion is a planner-backed package/version/action preview with zero writes. Release completion requires the
planned manifests and changelogs, repository formatting, one commit and annotated tag per target in dependency order,
and a final report of created commits/tags and skipped packages.
