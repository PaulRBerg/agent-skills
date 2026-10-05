# tailwind-variants

Use only if `tailwind-variants` is already installed or requested. Keep every emitted Tailwind class static and match
the installed package version.

- Use a normal `tv` recipe for one element and `slots` for multipart components. Slot variants return per-slot classes.
- Use `compoundVariants` or `compoundSlots` only for state combinations that cannot be expressed by one variant axis.
- Compose related recipes with `extend` instead of duplicating bases, slots, variants, or defaults.
- Keep one-off configuration local. `createTV` is for an intentionally shared configuration. Mutating `defaultConfig`
  changes every recipe in the process. Extend merge groups only for local custom tokens and prefer `extend`/`override`
  over replacing configuration.
- The lite entrypoint does not provide merge resolution. Do not select it when callers depend on conflict resolution.

Consult the [API](https://www.tailwind-variants.org/docs/api-reference),
[slots](https://www.tailwind-variants.org/docs/slots), [extending](https://www.tailwind-variants.org/docs/extending),
and [configuration](https://www.tailwind-variants.org/docs/configuration) for the installed version.
