# Tailwind CSS v4 Rules

> Read for v4 configuration, migration, source detection, or CSS directives. The installed packages, integration,
> browser matrix, CSS entrypoint, and local `@theme` declarations are authoritative.

## Configuration and migration

Keep the existing Vite, PostCSS, CLI, or other documented adapter. For an explicit migration, check the browser matrix
first: v4 relies on modern CSS and may require remaining on v3. Use the official upgrade tool when applicable, inspect
its diff, and verify the build and rendered UI. Do not copy v3 directives, utility rename lists, or JavaScript config
assumptions into v4 without checking the [upgrade guide](https://tailwindcss.com/docs/upgrade-guide).

In v4, use `@import "tailwindcss"`. In v4, use `@theme` only for values that should create utilities or variants, and
ordinary CSS variables otherwise. Preserve `@reference` in CSS modules or separately processed component styles when
they need main-sheet theme values, custom utilities, variants, `@apply`, or `@variant`. Prefer scoped CSS or variables
there. Follow [coding preferences](coding-preferences.md#markup-outside-application-control) for the narrow `@apply`
case.

## Source detection

Tailwind scans plain text. Map state or props to complete class strings. Interpolation such as `bg-${color}-600` is not
discoverable. Register ignored/external paths with `@source`, set an unambiguous monorepo base with import `source()`,
and use `@source inline()` only when no scanned static mapping can express a required utility.

## Current documentation

- [Upgrade guide](https://tailwindcss.com/docs/upgrade-guide)
- [Compatibility](https://tailwindcss.com/docs/compatibility)
- [Functions and directives](https://tailwindcss.com/docs/functions-and-directives)
- [Detecting classes](https://tailwindcss.com/docs/detecting-classes-in-source-files)
