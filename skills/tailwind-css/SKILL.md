---
disable-model-invocation: false
name: tailwind-css
user-invocable: false
description:
  "Use for Tailwind v4 styling: add/fix classes, configure or migrate Tailwind, use tailwind-variants, or
  tw-animate-css."
---

# Tailwind CSS

Style within the installed Tailwind version, existing design tokens, component patterns, and product context.

## Authority and Defaults

- Inspect package versions, CSS entrypoints, theme declarations, shared components, class-merging utilities, and nearby
  UI before editing. Those are authoritative.
- Apply this catalog's preferences only where the project is silent. Read
  [references/coding-preferences.md](references/coding-preferences.md) for that fallback style.
- Prefer existing semantic tokens and components over arbitrary values, new colors, or one-off utilities.
- Preserve responsive states, interaction states, accessibility, and dark-mode conventions. Do not add decorative UI or
  redesign beyond the request.
- Use CSS Modules only when the behavior is materially clearer than utilities; preserve any project-specific Tailwind
  reference mechanism.

## Routing

- For v4 configuration, migration, removed utilities, variables, gradients, or CSS-first directives, read
  `references/tailwind-v4-rules.md` after confirming v4 is installed.
- For component variants or slots with `tailwind-variants`, read `references/tailwind-variants.md`.
- For `tw-animate-css`, read `references/tw-animate-css.md`.
- For Tailwind ESLint integration, read `references/eslint.md`.

Do not apply v4 syntax to an older project merely because this skill targets v4. If the installed version differs, use
its official documentation or ask whether migration is intended.

## Workflow

1. Define the visual outcome and affected states from the request and product context.
2. Reuse local tokens, spacing, typography, breakpoints, components, and variant conventions. Make the smallest
   class/config change that achieves the outcome.
3. Run the repository's relevant lint/type/build checks.
4. Render the affected screen or component at representative viewport sizes and inspect it visually, including changed
   interaction and theme states. Fix visible regressions before completion.

Completion requires code checks plus rendered inspection; textual class review alone is insufficient.
