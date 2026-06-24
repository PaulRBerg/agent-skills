---
argument-hint: '[--dry-run] [package ...]'
disable-model-invocation: false
effort: high
model: opus
name: bump-deps
user-invocable: true
description: 'Use for dependency updates: update/bump deps, npm/pnpm/yarn/bun package upgrades, outdated checks, package.json updates, or taze.'
---

# Bump Dependencies Skill

Update Node.js dependencies using taze CLI with smart prompting: auto-apply MINOR/PATCH updates, prompt for MAJOR updates individually, skip fixed-version packages.

For projects with a package-manager minimum-age policy, pass Taze's maturity-period flags so Taze filters out too-new releases before writing manifests.

When package names are provided as arguments (e.g. `/bump-deps react typescript`), scope all taze commands to only those packages using `--include`.

When `--dry-run` is passed (e.g. `/bump-deps --dry-run` or `/bump-deps --dry-run react`), scan for updates and present a summary table **without applying any changes**. See [Dry Run Mode](#dry-run-mode) below.

## Prerequisites

Before choosing commands, check whether the target project has either:

- `bun.lock` or `bun.lockb` plus `bunfig.toml` with `[install].minimumReleaseAge`
- pnpm or Yarn minimum-age settings

If present, follow [Minimum Release Age Mode](#minimum-release-age-mode).

The scan command in [Step 1](#step-1-scan-for-updates) also verifies that taze is installed.

### Helper invocation

This skill ships a `run-taze.sh` helper. Resolve `<skill-dir>` from the loaded `SKILL.md` path, then invoke the helper by its absolute path and run it from the target repository cwd so taze scans that project. Never `cd` into the skill directory; a bare `scripts/run-taze.sh` resolves against the target repo, which does not contain the helper.

```bash
bash "<skill-dir>/scripts/run-taze.sh"
```

Every `<skill-dir>/scripts/run-taze.sh` invocation below uses this form. Run the helper once now to confirm taze is installed. If exit code is 1, stop and inform the user that taze must be installed:

- Global install: `npm install -g taze`
- One-time: `bunx taze`

## Minimum Release Age Mode

Use this mode for projects that configure a package-manager minimum-age policy.

Taze calls this `maturityPeriod`:

- `--maturity-period [days]` filters out package versions newer than the given number of days
- `--maturity-period-exclude <packages>` excludes packages from that filter, when supported by the installed Taze version

```bash
# 7-day cooldown
taze major -r --maturity-period 7
```

For Bun `minimumReleaseAge`, convert seconds to whole days using a ceiling division. Example: `604800` seconds becomes `--maturity-period 7`. If the configured seconds are not a whole number of days, round up so Taze is not weaker than the package manager policy.

Taze v19.13.0+ auto-infers maturity periods from pnpm and Yarn workspace config, but not from Bun `bunfig.toml`. For Bun projects, pass `--maturity-period` explicitly.

When the package manager config has an exclude list, pass matching Taze excludes if available:

```bash
taze major -r --maturity-period 7 --maturity-period-exclude react,webpack
```

Append the same maturity flags to every Taze scan and write command in the workflow. After Taze writes manifests, run the project package manager install as usual; the package manager remains the final enforcement layer for direct and transitive resolution.

## Update Workflow

### Step 1: Scan for Updates

Run the helper once to discover available updates. It auto-detects monorepo projects (`workspaces` in package.json or `pnpm-workspace.yaml`) and enables recursive mode automatically.

```bash
bash "<skill-dir>/scripts/run-taze.sh"
```

### Step 2: Parse and Categorize Updates

From the taze output, categorize each package update:

| Category  | Version Change                              | Action        |
| --------- | ------------------------------------------- | ------------- |
| **Fixed** | No `^` or `~` prefix (e.g., `"1.0.0"`)      | Skip entirely |
| **PATCH** | `x.y.z` → `x.y.Z` (e.g., `1.0.0` → `1.0.1`) | Auto-apply    |
| **MINOR** | `x.y.z` → `x.Y.0` (e.g., `1.0.0` → `1.1.0`) | Auto-apply    |
| **MAJOR** | `x.y.z` → `X.0.0` (e.g., `1.0.0` → `2.0.0`) | Prompt user   |

If package arguments were provided, filter to only those packages.

#### Dry Run Mode

If `--dry-run` was passed, **stop here** — do not apply any updates. Instead, present a single markdown table summarizing all available updates and exit. The table must include every discovered package (including fixed-version packages, shown as skipped):

```
| Package | Current | Available | Type | Action |
|---------|---------|-----------|------|--------|
| @types/node | ^20.0.0 | ^22.0.0 | major | prompt |
| typescript | ^5.3.0 | ^5.4.0 | minor | auto-apply |
| eslint | ^8.56.0 | ^8.57.0 | patch | auto-apply |
| lucide-react | ^3.0.0 | ^4.0.0 | major | auto-apply |
| lodash | 4.17.21 | 4.18.0 | minor | skip (fixed) |
```

**Column definitions:**

- **Package** — package name
- **Current** — version string as it appears in package.json (with range prefix)
- **Available** — new version string (preserving range prefix)
- **Type** — `major`, `minor`, or `patch`
- **Action** — what the normal (non-dry-run) workflow would do:
  - `auto-apply` — MINOR/PATCH updates and auto-approved major packages (e.g. `lucide-react`)
  - `prompt` — MAJOR updates that would be prompted to the user
  - `skip (fixed)` — fixed-version packages that would be skipped

Sort the table by action priority: `prompt` first, then `auto-apply`, then `skip (fixed)`. Within each group, sort alphabetically by package name.

After presenting the table, print a one-line summary: `N updates available (M major, P minor, Q patch, F fixed-skipped)` and stop. Do **not** proceed to Step 3 or beyond.

**Identifying fixed versions:** In package.json, fixed versions have no range prefix:

- Fixed: `"lodash": "4.17.21"` → skip
- Ranged: `"lodash": "^4.17.21"` → process

### Step 3: Select Updates to Apply

Automatically select all MINOR/PATCH updates for application. Also automatically select approved major packages:

- `lucide-react` (icon library with frequent major bumps, backward-compatible in practice)

Do not prompt for fixed-version packages. Do not prompt for auto-approved major packages.

If no packages are selected, stop without running install.

Report the packages selected for automatic update.

### Step 4: Prompt for MAJOR Updates

For each remaining major update, use `AskUserQuestion` to ask the user individually:

```
Package: <package-name>
Current: <current-version>
Available: <new-version>

Update to major version?
```

**Question format:**

- header: Package name (max 12 chars, truncate if needed)
- options: "Yes, update" / "No, skip"
- multiSelect: false

Collect all approved major updates.

### Step 5: Apply Selected Updates

After collecting user approvals, apply all selected packages in one write command. Include MINOR/PATCH updates, auto-approved major packages, and user-approved major packages:

```bash
bash "<skill-dir>/scripts/run-taze.sh" --write --include <pkg1>,<pkg2>,<pkg3>
```

The helper keeps the same monorepo and maturity-period flags used by the scan. It omits `--include-locked` in write mode so fixed-version packages remain untouched.

### Step 6: Update Bun Catalogs

After applying all updates, check the **root** `package.json` for Bun workspace catalogs. Bun monorepos can centralize dependency versions using `catalog` and `catalogs` fields inside the `workspaces` object:

```json
{
  "workspaces": {
    "packages": ["packages/*"],
    "catalog": {
      "react": "^19.0.0"
    },
    "catalogs": {
      "testing": {
        "jest": "^30.0.0"
      }
    }
  }
}
```

Workspace packages reference these with `"react": "catalog:"` (default catalog) or `"jest": "catalog:testing"` (named catalog).

**Skip this step** if neither `workspaces.catalog` nor `workspaces.catalogs` exists in the root `package.json`.

For each package that was updated in Step 5:

1. Check if it appears in `workspaces.catalog` — if so, update the version there
2. Check each named catalog in `workspaces.catalogs` — if the package appears, update the version there

Preserve the existing range prefix (`^`, `~`, or none) from the catalog entry. For example, if the catalog has `"react": "^19.0.0"` and taze bumped react to `19.1.0`, update the catalog to `"react": "^19.1.0"`.

Use `Edit` to apply the version changes directly to the root `package.json`.

### Step 7: Install Dependencies

After all updates are applied, run `ni` to install dependencies. It auto-detects the package manager.

## Taze Output Interpretation

Taze displays updates grouped by type. Example output:

```
@types/node  ^20.0.0  →  ^22.0.0   (major)
typescript   ^5.3.0   →  ^5.4.0    (minor)
eslint       ^8.56.0  →  ^8.57.0   (patch)
```

The rightmost column indicates update type (major/minor/patch).

Packages shown with `--include-locked` that have no `^` or `~` are fixed versions—skip these entirely.

## Script Reference

| Script                | Purpose                                              |
| --------------------- | ---------------------------------------------------- |
| `scripts/run-taze.sh` | Run taze in non-interactive mode, check installation |

Supported script arguments:

- `--include <packages>` — pass a comma-separated package list or taze include regex
- `--concurrency <n>` — pass through taze's request concurrency setting
- `--write` — write selected updates; requires `--include`; scan mode is the default

## Important Notes

- Fixed-version dependencies (no `^` or `~`) indicate intentional pinning—never modify these
- MAJOR updates may contain breaking changes—prompt the user unless the package is explicitly auto-approved
- MINOR/PATCH updates are backward-compatible by semver convention—safe to auto-apply
- The `--include` flag accepts comma-separated package names or regex patterns
- Monorepo detection is automatic—no flag needed when using the `run-taze.sh` helper
- Bun catalogs (`workspaces.catalog` / `workspaces.catalogs`) are the source of truth for workspace packages using the `catalog:` protocol—always update catalog entries alongside regular deps
