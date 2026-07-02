# Conditional Workflows

Both sections below are gated: read only when the triggering condition in `SKILL.md` is met.

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

## Update Bun Catalogs

After applying all updates (Step 5), check the **root** `package.json` for Bun workspace catalogs. Bun monorepos can centralize dependency versions using `catalog` and `catalogs` fields inside the `workspaces` object:

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

For each package that was updated in Step 5:

1. Check if it appears in `workspaces.catalog` — if so, update the version there
2. Check each named catalog in `workspaces.catalogs` — if the package appears, update the version there

Preserve the existing range prefix (`^`, `~`, or none) from the catalog entry. For example, if the catalog has `"react": "^19.0.0"` and taze bumped react to `19.1.0`, update the catalog to `"react": "^19.1.0"`.

Use `Edit` to apply the version changes directly to the root `package.json`.
