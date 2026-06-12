---
disable-model-invocation: true
name: tailwind-css
user-invocable: true
description: 'Use for Tailwind v4 styling: add/fix classes, configure or migrate Tailwind, use tailwind-variants, or tw-animate-css.'
---

# Tailwind CSS v4

Expert guidance for Tailwind CSS v4, CSS-first configuration, modern utility patterns, and type-safe component styling with tailwind-variants.

## CSS-First Configuration

Tailwind CSS v4 eliminates `tailwind.config.ts` in favor of CSS-only configuration. All configuration lives in CSS files using special directives.

**Core Directives:**

- `@import "tailwindcss"` - Entry point that loads Tailwind
- `@theme { }` - Define or extend design tokens
- `@theme static { }` - Define tokens that should not generate utilities
- `@utility` - Create custom utilities
- `@custom-variant` - Define custom variants

**Minimal Example:**

```css
@import "tailwindcss";

@theme {
  --color-brand: oklch(0.72 0.11 178);
  --font-display: "Inter", sans-serif;
  --spacing-edge: 1.5rem;
}
```

All theme tokens defined with `@theme` automatically become available as utility classes. For example, `--color-brand` can be used as `bg-brand`, `text-brand`, `border-brand`, etc.

## ESLint Integration

Use `eslint-plugin-better-tailwindcss` for Tailwind CSS v4 class validation and style enforcement.

**Correctness Rules (errors):**

- `no-conflicting-classes` - Detect classes that override each other
- `no-unknown-classes` - Flag classes not registered with Tailwind

**Stylistic Rules (warnings):**

- `enforce-canonical-classes` - Use standard v4 class names
- `enforce-shorthand-classes` - Use abbreviated class versions
- `no-deprecated-classes` - Remove outdated class names
- `no-duplicate-classes` - Eliminate redundant declarations
- `no-unnecessary-whitespace` - Clean up extra spacing

**Examples:**

```typescript
// ❌ Bad: separate padding
<div className="px-6 py-6">

// ✅ Good: shorthand
<div className="p-6">
```

```typescript
// ❌ Bad: separate width/height
<div className="w-6 h-6">

// ✅ Good: size utility
<div className="size-6">
```

Run the project's ESLint check after modifying Tailwind classes to validate all changes across the codebase.

## Coding Preferences

For detailed coding patterns covering layout, spacing, typography, colors, borders, gradients, arbitrary values, class merging, image sizing, z-index, and dark mode, see [references/coding-preferences.md](references/coding-preferences.md).

## CSS Modules

Use CSS Modules only as a last resort for complex CSS that cannot be easily written with Tailwind classes.

All `.module.css` files must include `@reference "#tailwind";` at the top to enable Tailwind utilities and theme tokens inside the module.

**Example:**

```css
/* component.module.css */
@reference "#tailwind";

.component {
  /* Complex CSS that can't be expressed with Tailwind utilities */
  /* Can still use Tailwind utilities and theme tokens */
}
```

## Common Tasks

### Adding a Component with Variants

1. Read `references/tailwind-variants.md` for patterns
2. Check the project's `@theme` configuration for available tokens
3. Use `tv()` from `tailwind-variants` for type-safe variants

**Example:**

```typescript
import { tv } from "tailwind-variants";

const button = tv({
  base: "rounded-lg px-4 py-2 font-medium",
  variants: {
    color: {
      primary: "bg-blue-600 text-white",
      secondary: "bg-gray-600 text-white",
    },
    size: {
      sm: "text-sm",
      md: "text-base",
      lg: "text-lg",
    },
  },
});
```

### Debugging Styles

1. Check `references/tailwind-v4-rules.md` for breaking changes
2. Verify gradient syntax (`bg-linear-*`, not `bg-gradient-*`)
3. Verify CSS variable syntax (`bg-my-color`, not `bg-[--var-my-color]`)
4. Check if arbitrary value exists in the project's `@theme` configuration

### Working with Colors

1. Check the project's `@theme` configuration first to see available colors
2. Use semantic color names when available
3. Use opacity modifiers for transparency (`/20`, `/50`, etc.)
4. Avoid arbitrary colors unless absolutely necessary

**Example:**

```typescript
// ✅ Good: theme token with opacity
<div className="bg-brand/20 text-brand">

// ❌ Avoid: arbitrary hex
<div className="bg-[#4f46e5]/20 text-[#4f46e5]">
```

### Adding Animations

1. Read `references/tw-animate-css.md` for available animations
2. Combine a base class (`animate-in` or `animate-out`) with effect classes
3. Note decimal spacing gotcha: use `[0.625rem]` syntax, not `2.5`

**Example:**

```typescript
// Enter: fade + slide up
<div className="fade-in slide-in-from-bottom-4 duration-300 animate-in">

// Exit: fade + slide down
<div className="fade-out slide-out-to-bottom-4 duration-200 animate-out">
```

## Quick Reference Table

| Aspect             | Pattern                                           |
| ------------------ | ------------------------------------------------- |
| Configuration      | CSS-only: `@theme`, `@utility`, `@custom-variant` |
| Gradients          | `bg-linear-*`, `bg-radial`, `bg-conic`            |
| Opacity            | Modifier syntax: `bg-black/50`                    |
| Line Height        | Modifier syntax: `text-base/7`                    |
| Font Features      | `font-features-zero`, `font-features-ss01`, etc.  |
| CSS Variables      | `bg-my-color` (auto-created from `@theme`)        |
| CSS Modules        | `@reference "#tailwind";` at top                  |
| Class Merging      | `cn()` for conditionals; plain string for static  |
| Viewport           | `min-h-dvh` (not `min-h-screen`)                  |
| Component Variants | `references/tailwind-variants.md`                 |
| Animations         | `references/tw-animate-css.md`                    |
| V4 Rules           | `references/tailwind-v4-rules.md`                 |

## Reference Documentation

- **Tailwind v4 Rules & Best Practices:** `references/tailwind-v4-rules.md` — Breaking changes, removed/renamed utilities, layout rules, typography, gradients, CSS variables, new v4 features, common pitfalls
- **tailwind-variants Patterns:** `references/tailwind-variants.md` — Component variants, slots API, composition, TypeScript integration, responsive variants
- **tw-animate-css Reference:** `references/tw-animate-css.md` — Enter/exit animations, slide/fade/zoom utilities, spacing gotchas
