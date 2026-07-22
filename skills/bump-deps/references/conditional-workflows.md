# Conditional Workflows

Both sections below are gated: read only when the triggering condition in `SKILL.md` is met.

## Minimum Release Age Mode

Use this mode for projects that configure a package-manager minimum-age policy.

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

When the package manager config has an exclude list, pass matching Taze excludes if available:

```bash
taze major -r --maturity-period 7 --maturity-period-exclude react,webpack
```

Append the same maturity flags to every Taze scan and write command in the workflow. After Taze writes manifests, run
the project package manager install as usual; the package manager remains the final enforcement layer for direct and
transitive resolution.

## Update Bun Catalogs

When the root `package.json` contains `workspaces.catalog` or `workspaces.catalogs`, use
`scripts/update-bun-catalogs.py` with the saved Taze plan and agent-accepted include set. Preview before manifest
writes; after the Taze write, rerun the same command with `--write` before regenerating the lockfile.

The helper owns default/named catalog discovery, multiple occurrences, prefix preservation, stale-plan validation, and
atomic replacement. The agent owns which upgrades are accepted and whether a major migration is compatible. Do not
manually reproduce the catalog transition or weaken a helper failure.

Use `Edit` to apply the version changes directly to the root `package.json`.
