# Conditional Workflows

Read either section below only when its triggering condition in `SKILL.md` is met.

## Minimum Release Age Mode

Use this mode for projects that configure a package-manager minimum-age policy.

Discover the effective project and user-level package-manager configuration. A configured age gate makes `@latest` a
valid initial selector and `^x.y.z` a valid retained manifest range: the installer filters out releases inside the
cooldown. Do not exact-pin solely to impose the same freshness boundary. Bun still age-gates exact requests, but they
bypass its rapid-release stability check.

The age gate is not a reproducibility mechanism. Commit the lockfile and use frozen installs in deployment. If the
deployment resolves dependencies without that lockfile, require the same effective age policy there or exact-pin for
that workflow.

Taze calls this `maturityPeriod`:

- `--maturity-period [days]` filters out package versions newer than the given number of days
- `--maturity-period-exclude <packages>` excludes packages from that filter, when supported by the installed Taze
  version

```bash
# 7-day cooldown
taze major -r --maturity-period 7
```

For Bun `minimumReleaseAge`, convert seconds to whole days using a ceiling division. Example: `604800` seconds becomes
`--maturity-period 7`. If the configured seconds are not a whole number of days, round up so Taze is not weaker than the
package manager policy.

Taze v19.13.0+ auto-infers maturity periods from pnpm and Yarn workspace config, but not from Bun `bunfig.toml`. For Bun
projects, pass `--maturity-period` explicitly.

For Bun lockfile projects, `run-taze.sh` uses Python 3.11+ through `uv` to parse the global `.bunfig.toml` under
`$XDG_CONFIG_HOME` (or `$HOME` when unset), then overlay project `[install]` keys. Multiline exclusion arrays are
supported. An explicit local `0` or empty array overrides the inherited value. See
[Bun configuration](https://bun.sh/docs/runtime/bunfig#global-vs-local). Verify the installer actually enforces
inherited age settings. If it ignores them, preserve the policy in project configuration before installing.

When the package manager config has an exclude list, pass matching Taze excludes if available:

```bash
taze major -r --maturity-period 7 --maturity-period-exclude react,webpack
```

`run-taze.sh` adds these flags itself for Bun lockfile projects and rejects extra options. Taze infers them for pnpm and
Yarn. Append the same maturity flags only to direct Taze scan and write commands. After Taze writes manifests, run the
project package manager install as usual. The package manager remains the final enforcement layer for direct and
transitive resolution.

### Approved Release-Age Exceptions

Preserve the configured age floor and existing effective exclusions. Limit an approved exception to the selected version
when supported. For pnpm, use `package@version` in the project's `minimumReleaseAgeExclude` list in
`pnpm-workspace.yaml`. Preserve inherited exclusions when the project setting replaces them. See the
[pnpm exclusion settings](https://pnpm.io/settings/dependency-resolution#minimumreleaseageexclude).

After regenerating the lockfile, run the project's ordinary frozen install without temporary age overrides. An update
command, printed configuration, or lockfile does not prove that this install succeeds. Require its final successful exit
status. If it rejects the approved release, correct the narrow exception and rerun the install. Do not lower the age
floor.

## Update Bun Catalogs

When the root `package.json` contains `catalog` / `catalogs`, use `scripts/update-bun-catalogs.py` with the saved Taze
plan. Select the packages and catalogs used for installed dependencies. Use repeatable `--catalog <name>` arguments and
`--catalog default` for the unnamed catalog. Without `--catalog`, the helper selects all catalogs for compatibility.
Preserve catalogs used for public peer ranges. A peer range describes supported consumer versions, not an installed
version.

```sh
uv run <skill-dir>/scripts/update-bun-catalogs.py \
  --root <repo> --plan <taze-plan.json> --include package-a,package-b \
  --catalog default --catalog dev
```

Skip the helper when no selected package occurs in the selected catalogs. Preview before manifest writes. After the
baseline passes, rerun the same command with `--write`. Then write direct dependencies:

```sh
bash <skill-dir>/scripts/run-taze.sh --write --direct-only --include package-a,package-b
```

The direct write uses the installed Taze public API and excludes every Bun catalog from writes. It also preserves
ordinary `peerDependencies` and fixed dependencies. It retains the wrapper's package selection, recursion, concurrency,
and Bun maturity flags. This mode requires Taze `>=21.3.0 <22` with `beforePackageWrite` support.

The API does not expose the CLI configuration resolver. The helper therefore rejects Taze configuration files, pnpm/Yarn
configuration files, and pnpm/Yarn package-manager declarations in the project or its ancestors. If it rejects
configuration, stop before writing and extend the helper to honor the effective policy. Do not bypass the failure with
native Taze catalog writes. Regenerate the lockfile after the protected write succeeds.

The catalog helper matches each selected occurrence's current value against plan rows. Different current versions can
have different targets. Duplicate matching rows must agree on one target. Unknown catalogs, missing entries, unsupported
selected ranges, ambiguous targets, and stale plans fail before writes. Unselected catalog values remain unchanged.
Fixed entries marked `skip-fixed` remain unchanged. The helper validates every transition before atomic replacement and
preserves each existing prefix. The agent owns which upgrades are accepted and whether a major migration is compatible.
Do not manually reproduce catalog transitions or weaken a helper failure.
