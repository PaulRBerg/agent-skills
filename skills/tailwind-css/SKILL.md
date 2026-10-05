---
name: tailwind-css
user-invocable: false
description:
  "Use for Tailwind v4 styling: add/fix classes, configure or migrate Tailwind, use tailwind-variants, or
  tw-animate-css."
---

# Tailwind CSS

Follow the installed Tailwind version and the repository's tokens, components, class-merging utility, CSS entrypoint,
and nearby UI. They take precedence over this skill. Do not add packages, change integration, or migrate versions
without a request and local need.

## Routing

- Apply [coding preferences](references/coding-preferences.md) only where the project is silent.
- For v4 configuration, migration, directives, or generated classes, use [v4 rules](references/tailwind-v4-rules.md) and
  the matching official docs.
- Read [tailwind-variants](references/tailwind-variants.md), [tw-animate-css](references/tw-animate-css.md), or
  [ESLint](references/eslint.md) only when that integration exists locally or the request adds it.

Do not apply v4 syntax to an older installation. Preserve responsive, interaction, accessible, and dark-mode behavior.
Do not redesign beyond the request.

## Completion

Define the intended visual and state change. Reuse local conventions. Keep classes statically discoverable.

If source registration or generated mappings change, run the real Tailwind build and confirm the expected utilities. Run
required repository checks and inspect the changed states at one representative viewport for a small style edit. Broaden
to narrow/wide viewports, themes, and interactions when responsive rules or shared styling changed. When markup is
transformed by JavaScript or a component library, inspect the final DOM too. Textual class review alone is insufficient.
Repeat checks only when subsequent edits affect their evidence.

Finish with `### 🎨 Tailwind — ✅ styling updated` (or `### 🎨 Tailwind — 🔎 inspected, no files written`) and
code-check and rendered-inspection evidence. Use prose for one inspected state and a compact table for several. Add
`### ⚠️ Remaining` only when needed. Keep source UI copy and diagnostics undecorated.
