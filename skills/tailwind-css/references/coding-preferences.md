# Tailwind Engineering Preferences

> Fallbacks only: local design-system and component conventions win.

## Choices that prevent churn

- Use this ladder:
  - utility composition
  - local component/template extraction
  - `@theme` token for a shared product value
  - `@utility` for a repeated low-level behavior
  - scoped CSS for a stable primitive or markup the app does not control

  Do not abstract a genuine one-off.

- Use `gap` for flex/grid spacing, `size-*` for equal dimensions, and `min-h-dvh` for full-height mobile layouts. Prefer
  top/left margins, parent padding for trailing space, and the container/spacing scales before arbitrary pixels.
- Prefer `text-{size}/{line-height}` to a separate `leading-*` utility. Use theme color and sizing scales before
  arbitrary values. Promote recurring values to tokens.
- Use `cn` for reusable class constants, conditional classes, caller overrides, and conflict resolution. Keep static
  `className` strings direct. Size images with utilities and define stacking levels as `@theme` tokens rather than
  arbitrary `z-[…]` values.
- Put light styles first and append `dark:` overrides.

## Markup outside application control

Scope CMS, rich-text, and third-party output under a dedicated wrapper or the local typography integration. Target its
final DOM, including nested, focus, hover, and motion states. Use `@apply` only as a narrow adapter in such custom or
transformed markup. Ordinary component styling remains utility composition.
