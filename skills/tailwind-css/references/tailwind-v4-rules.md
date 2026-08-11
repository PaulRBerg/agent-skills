# Tailwind CSS v4 Rules

> When to read: for v4 configuration, migration, source detection, CSS-first directives, compatibility, or removed and
> renamed behavior.

## Authority and Sources

- The installed Tailwind packages, build integration, browser targets, CSS entrypoints, and local `@theme` declarations
  are authoritative. Do not upgrade Tailwind or replace the project's integration unless the request includes that
  change.
- Use the official documentation matching the installed minor version when behavior may have changed. This reference is
  a verification checklist and compact offline fallback, not an exhaustive copy of the docs.
- For an explicit v3-to-v4 migration, prefer the official upgrade tool when the project satisfies its requirements, then
  inspect its diff and run the project's build and rendered checks.

## Migration and Compatibility Checklist

Before migrating or reviewing a migration, verify:

- The project's browser matrix against [v4 compatibility](https://tailwindcss.com/docs/compatibility). Tailwind v4
  depends on modern CSS features; projects supporting older browsers may need to remain on v3.
- The active integration: `@tailwindcss/vite`, `@tailwindcss/postcss`, `@tailwindcss/cli`, or another documented
  adapter. Preserve the existing integration unless changing it is part of the task.
- The CSS entrypoint uses `@import "tailwindcss"` instead of the removed `@tailwind` directives.
- Prefix syntax, important-modifier placement, variant stacking order, arbitrary CSS-variable syntax, and source
  detection against the current [upgrade guide](https://tailwindcss.com/docs/upgrade-guide).
- Default border and ring behavior, hover behavior on touch devices, and individual transform properties where the
  existing UI depends on v3 defaults.

Common removed or renamed utilities include:

| v3                                     | v4                                    |
| -------------------------------------- | ------------------------------------- |
| `bg-opacity-*`, `text-opacity-*`, etc. | Color opacity modifiers (`/50`)       |
| `flex-shrink-*`, `flex-grow-*`         | `shrink-*`, `grow-*`                  |
| `overflow-ellipsis`                    | `text-ellipsis`                       |
| `bg-gradient-*`                        | `bg-linear-*`                         |
| `outline-none`                         | `outline-hidden`                      |
| `shadow-sm`, `shadow`                  | `shadow-xs`, `shadow-sm`              |
| `blur-sm`, `blur`                      | `blur-xs`, `blur-sm`                  |
| `rounded-sm`, `rounded`, `rounded-md`  | `rounded-xs`, `rounded-sm`, `rounded` |

Use the upgrade guide for the complete version-specific list rather than extrapolating from this table.

## CSS-First Configuration

- Define a value with `@theme` when it should create Tailwind utilities or variants. Use `:root` for ordinary CSS
  variables that should not expand Tailwind's API. See [theme variables](https://tailwindcss.com/docs/theme).
- Inspect the project's `--breakpoint-*` variables before using responsive variants. Do not assume the default
  breakpoint set is present.
- Register a reusable low-level or variant-aware primitive with `@utility`. Keep utilities composable and limited to one
  behavior. See [custom styles](https://tailwindcss.com/docs/adding-custom-styles).
- Use scoped plain CSS for stable component APIs or markup the application does not control. `@apply` remains valid as a
  narrow adapter for cases such as third-party markup; it is not the default architecture for application styling.
- In CSS Modules or component `<style>` blocks, preserve the project's `@reference` mechanism when Tailwind theme
  values, custom utilities, custom variants, `@apply`, or `@variant` need access to the main stylesheet context.
- Tailwind v4 is not designed to be combined with Sass, Less, or Stylus. Confirm the current compatibility guidance
  before changing a preprocessor-based project.

## Source Detection

Tailwind scans source files as plain text rather than parsing the host language:

- Write complete class names in source. Map props or state values to complete static strings instead of interpolating
  class fragments such as `bg-${color}-600`.
- Use `@source` for external or otherwise ignored sources. In monorepos, use the import `source()` base path when the
  build working directory would make automatic detection ambiguous.
- Use `@source inline()` only when a required utility cannot be represented as a complete static string in a scanned
  source file. Prefer explicit mappings over broad safelists.
- After changing source registration or generated class mappings, run the real Tailwind build and verify that the
  expected utilities are present.

See [detecting classes in source files](https://tailwindcss.com/docs/detecting-classes-in-source-files) for current
syntax and exclusions.

## Official Documentation Map

- [Upgrade guide](https://tailwindcss.com/docs/upgrade-guide)
- [Compatibility](https://tailwindcss.com/docs/compatibility)
- [Functions and directives](https://tailwindcss.com/docs/functions-and-directives)
- [Theme variables](https://tailwindcss.com/docs/theme)
- [Adding custom styles](https://tailwindcss.com/docs/adding-custom-styles)
- [Responsive design](https://tailwindcss.com/docs/responsive-design)
- [Detecting classes in source files](https://tailwindcss.com/docs/detecting-classes-in-source-files)
