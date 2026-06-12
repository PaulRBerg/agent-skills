---
argument-hint: '[packages...] [version] [--beta] [--dry-run]'
disable-model-invocation: true
effort: high
model: opus
name: bump-release
user-invocable: true
description: 'Use for release versioning: bump/cut/tag a release, bump version, create a release, changelog updates, or version tagging.'
---

# Bump Release

Support for both regular and beta releases, in single-package repos and monorepos. A single invocation may release one package or several.

## Parameters

- `packages`: Optional list of monorepo package directories or names to release (e.g., `evm evm-safe`). Omit in single-package repos
- `version`: Optional explicit version to use (e.g., `2.0.0`). When provided, skips automatic version inference. Only valid when releasing a single package
- `--beta`: Create a beta release with `-beta.X` suffix
- `--dry-run`: Preview the release without making any changes (no file modifications, commits, or tags)

## Repo Layout and Package Selection

The user may run this skill in a single-package repo or in a monorepo (e.g., `~/projects/prb-effect`, whose workspaces include `evm`, `evm-safe`, `next`, `solana`, `xstate`), and may request releases for multiple packages at once (e.g., "bump evm and evm-safe").

1. **Detect the layout** - A monorepo declares `workspaces` in the root `package.json` or ships a `pnpm-workspace.yaml`. Otherwise, treat the repo root as the only package
2. **Resolve target packages** - In a monorepo, determine which packages to release in this order of precedence:
   - Explicit `packages` arguments or package names in the user's prompt
   - The current working directory, when it sits inside exactly one workspace package
3. **Ask when unclear** - If the target set is ambiguous or unstated, do not guess. Use `AskUserQuestion` (multiSelect: true) listing the workspace packages so the user confirms which to release
4. **Scope everything per package** - All file paths (`CHANGELOG.md`, `package.json`) are relative to each package directory. Compute changelog diffs against that package's own previous tag, restricted to files under the package directory. The `justfile` usually lives at the repo root

## Dependent Packages

Bumping one workspace package can force releases of others. After deciding the new version for each requested package:

1. **Scan for dependents** - Check every other workspace package's `dependencies` and `peerDependencies` for the bumped package (e.g., `@prb/effect-evm-safe` declares `"@prb/effect-evm": "^2.0.0"`)
2. **Range check** - If the new version still satisfies the declared range, the dependent needs no release. If it falls outside the range (e.g., `evm` goes `2.x` → `3.0.0` against `^2.0.0`), update the range and release the dependent too — even when the user did not name it
3. **Pick the dependent's bump level** - A widened `dependencies` range is usually a patch. Raising a `peerDependencies` major is breaking for the dependent's consumers and usually warrants a major; confirm with `AskUserQuestion` when in doubt
4. **Surface the cascade** - Include cascaded dependents in the release-plan confirmation so the user sees the full package set before any file is modified
5. **Release in dependency order** - Process dependencies before dependents so each updated range points at an already-tagged version

## Steps

Run these steps once per target package, in dependency order:

1. Update the package's `CHANGELOG.md` file with all changes since its last version release (**skip this step for beta releases**).
2. Bump the version in the package's `package.json`:
   - **Regular release**: Follow semantic versioning (e.g., 1.2.3)
   - **Beta release**: Add `-beta.X` suffix (e.g., 1.2.3-beta.1)
3. **Format files** - If a `justfile` exists in the repository, run `just full-write` to ensure `CHANGELOG.md` and `package.json` are properly formatted
4. Commit the changes:
   - **Single-package repo**: "docs: release <version>"
   - **Monorepo**: "docs: release <package> <version>" — one commit per package
5. Create a new git tag:
   - **Single-package repo**: `git tag -a v<version> -m "<version>"`
   - **Monorepo**: follow the repo's existing tag convention (inspect `git tag`); default to `<package-dir>@<version>` (e.g., `evm@1.3.1`) when none exists. One tag per package, pointing at that package's release commit

**Note**: When `--dry-run` flag is provided, display what would be done without making any actual changes to files, creating commits, or tags.

## Process

1. **Check for arguments** - Determine which `packages` were named (monorepo), if `version` was provided, if this is a beta release (`--beta`), and/or dry-run (`--dry-run`)

2. **Resolve target packages** - Follow "Repo Layout and Package Selection" above. In a monorepo, ask via `AskUserQuestion` unless the user's prompt or working directory makes the target packages unambiguous

3. **Check for clean working tree** - Run `git status --porcelain` to verify there are no uncommitted changes unrelated to this release. If there are, run the `commit` skill to commit them before proceeding

4. **Write Changelog** - For each target package, examine diffs between the current branch and that package's previous tag (scoped to the package directory in a monorepo) to write its Changelog. Then find
   relevant PRs by looking at the commit history and add them to each changelog (when available). If `package.json` contains
   a `files` field, only include changes within those specified files/directories. If no `files` field exists, include all
   changes except test changes, CI/CD workflows, and development tooling

5. **Follow format** - Consult `references/common-changelog.md` for the Common Changelog specification

6. **Check version** - Get the current version from each target package's `package.json`

7. **Bump version** - If `version` argument provided (single-package release only), use it directly. Otherwise, for each target package, if unchanged since its last release, increment per Semantic Versioning rules:

   - **For regular releases**:

     - **PATCH** (x.x.X) - Bug fixes, documentation updates
     - **MINOR** (x.X.x) - New features, backward-compatible changes
     - **MAJOR** (X.x.x) - Breaking changes

   - **For beta releases** (`--beta` flag):

     - If current version has no beta suffix: Add `-beta.1` to the version
     - If current version already has beta suffix: Increment beta number (e.g., `-beta.1` → `-beta.2`)
     - If moving from beta to release: Remove beta suffix and use the base version

   - **When unsure** — If the changes are ambiguous (e.g., a new feature that may also break consumers, or a mix of fixes and features), use `AskUserQuestion` to let the user decide the semver level:

     - header: "Version"
     - question: "Changes include both `<summary>`. Which release level?"
     - options: list the plausible semver levels with their resulting version (e.g., "1.3.0 (minor)", "2.0.0 (major)")
     - multiSelect: false

     Use the user's choice and skip step 9 for that package

8. **Cascade to dependents** - Follow "Dependent Packages" above: scan the other workspace packages for `dependencies`/`peerDependencies` ranges that the new versions no longer satisfy, and add those packages to the release plan with updated ranges

9. **Confirm version** - When the version was confidently inferred (no explicit `version` argument), use `AskUserQuestion` to confirm before proceeding:

   - header: "Version"
   - question: "Release `<current>` → `<inferred>`?"
   - options:
     - The inferred version label (e.g., "1.3.0 (minor)") — mark as "(Recommended)"
     - One alternative that is one semver level higher (e.g., "2.0.0 (major)")
     - One alternative that is one semver level lower when possible (e.g., "1.2.4 (patch)")
   - multiSelect: false

   For multi-package releases, confirm the full plan instead: one question per package (max 4 per `AskUserQuestion` call), covering requested packages and cascaded dependents alike, so the user sees every package that will be released. If the user picks an alternative, use that version for the remaining steps. Skip this step when `--dry-run` is active (show the inferred versions in the preview instead)

## Beta Release Logic

When `--beta` flag is provided in the $ARGUMENTS

1. **Check for explicit version** - If `version` provided:
   - If version already has beta suffix → use as-is
   - If version has no beta suffix → append `-beta.1`
2. **Otherwise, parse current version** from `package.json` and **determine beta version**:
   - If current version is `1.2.3`: Create `1.2.4-beta.1` (increment patch + beta.1)
   - If current version is `1.2.3-beta.1`: Create `1.2.3-beta.2` (increment beta number)
   - If current version is `1.2.3-beta.5`: Create `1.2.3-beta.6` (increment beta number)
3. **Skip CHANGELOG.md update** - Beta releases don't update the changelog
4. **Commit and tag** with beta version (e.g., `v1.2.4-beta.1`)

## Output

For regular releases only, generate changelog entries in `CHANGELOG.md` following the format and writing guidelines in `references/common-changelog.md`. Use the `Changed`, `Added`, `Removed`, `Fixed` categories (in that order). Every entry must begin with a present-tense verb in imperative mood.

## Inclusion Criteria

For regular releases only (changelog generation is skipped for beta releases):

- **Files field constraint** - If `package.json` contains a `files` field, only include changes to files/directories specified in that array. All other codebase changes should be excluded from the CHANGELOG
- **Production changes only** - When no `files` field exists, exclude test changes, CI/CD workflows, and development tooling
- **Reference pull requests** - Link to PRs when available for context
- **Net changes only** - Examine diffs between the current branch and the previous tag to identify changes
- **Only dependencies and peerDependencies changes** - Exclude changes to devDependencies

## Examples

### Regular Release

```bash
# Create a regular patch/minor/major release
/bump-release

# Preview what a regular release would do
/bump-release --dry-run
```

### Beta Release

```bash
# Create a beta release with -beta.X suffix
/bump-release --beta

# Preview what a beta release would do
/bump-release --beta --dry-run
```

### Monorepo

```bash
# Release two workspace packages (e.g., in ~/projects/prb-effect)
/bump-release evm evm-safe

# Release one workspace package; cascade to dependents if their ranges break
/bump-release evm

# Preview a multi-package release
/bump-release evm evm-safe --dry-run
```

### Explicit Version

```bash
# Specify exact version
/bump-release 2.0.0

# Specify exact beta version
/bump-release 2.0.0-beta.1

# Combine with flags
/bump-release 2.0.0 --dry-run
```

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

- `references/common-changelog.md` — Read when generating changelog entries: Common Changelog format and writing guidelines
