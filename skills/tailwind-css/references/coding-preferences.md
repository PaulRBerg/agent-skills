# Coding Preferences

> When to read: when writing or fixing Tailwind utility class usage, layout, spacing, typography, color, border, gradient, arbitrary value, class-merging, image-sizing, z-index, or dark-mode patterns.

## Layout and Spacing

**Use `gap` for flex/grid spacing, not `space-x`/`space-y`:**

The `gap` utilities handle wrapping correctly, while `space-*` utilities break when flex/grid items wrap to multiple lines.

```typescript
// ✅ Good: gap handles wrapping
<div className="flex gap-4">

// ❌ Bad: breaks when items wrap
<div className="flex space-x-4">
```

**Prefer `size-*` over separate `w-*`/`h-*` for equal dimensions:**

```typescript
// ✅ Good: concise
<div className="size-16">

// ❌ Bad: redundant
<div className="w-16 h-16">
```

**Always use `min-h-dvh` instead of `min-h-screen`:**

Dynamic viewport height (`dvh`) accounts for mobile browser chrome, while `vh` units ignore it and cause layout issues on mobile Safari.

```typescript
// ✅ Good: works on mobile Safari
<main className="min-h-dvh">

// ❌ Bad: buggy on mobile Safari
<main className="min-h-screen">
```

**Prefer top/left margins over bottom/right:**

Consistent directionality improves layout predictability.

```typescript
// ✅ Good: top margin
<div className="mt-4">

// ❌ Avoid: bottom margin (unless needed)
<div className="mb-4">
```

**Use padding on parent containers instead of bottom margins on last child:**

Padding provides consistent spacing without needing `:last-child` selectors.

```typescript
// ✅ Good: padding on parent
<section className="pb-8">
  <div>Item 1</div>
  <div>Item 2</div>
  <div>Item 3</div>
</section>

// ❌ Avoid: margin on children
<section>
  <div className="mb-4">Item 1</div>
  <div className="mb-4">Item 2</div>
  <div>Item 3</div>
</section>
```

**For max-widths, prefer container scale over pixel values:**

```typescript
// ✅ Good: semantic container size
<div className="max-w-2xl">

// ❌ Avoid: arbitrary pixel value
<div className="max-w-[672px]">
```

## Typography

**Avoid `leading-*` classes; use line height modifiers:**

Tailwind v4 supports inline line height modifiers with the `text-{size}/{leading}` syntax.

```typescript
// ✅ Good: combined size and line height
<p className="text-base/7">

// ❌ Bad: separate utilities
<p className="text-base leading-7">
```

**Font Size Reference:**

| Class       | Size |
| ----------- | ---- |
| `text-xs`   | 12px |
| `text-sm`   | 14px |
| `text-base` | 16px |
| `text-lg`   | 18px |
| `text-xl`   | 20px |

## Colors and Opacity

**Use opacity modifier syntax, not separate opacity utilities:**

All `*-opacity-*` utilities were removed in Tailwind v4. Use the modifier syntax instead.

```typescript
// ✅ Good: opacity modifier
<div className="bg-red-500/60">

// ❌ Bad: removed in v4
<div className="bg-red-500 bg-opacity-60">
```

**Prefer design tokens over arbitrary hex values:**

Check the project's `@theme` configuration before using arbitrary color values.

```typescript
// ✅ Good: theme token
<div className="bg-brand">

// ❌ Avoid: arbitrary hex (check theme first)
<div className="bg-[#4f46e5]">
```

## Border Radius

Tailwind v4 renamed border radius utilities:

| v3           | v4 (equivalent) | Size |
| ------------ | --------------- | ---- |
| `rounded-sm` | `rounded-xs`    | 2px  |
| `rounded`    | `rounded-sm`    | 4px  |
| `rounded-md` | `rounded`       | 6px  |
| `rounded-lg` | `rounded-md`    | 8px  |

Use the v4 names when writing new code.

## Gradients

Tailwind v4 renamed gradient utilities and added new gradient types.

**Use `bg-linear-*`, not `bg-gradient-*`:**

```typescript
// ✅ Good: v4 syntax
<div className="bg-linear-to-r from-blue-500 to-purple-500">

// ❌ Bad: removed in v4
<div className="bg-gradient-to-r from-blue-500 to-purple-500">
```

**New gradient types:**

- `bg-radial` - Radial gradients
- `bg-conic` - Conic gradients

**Example:**

```typescript
<div className="bg-radial from-blue-500 to-purple-500">
<div className="bg-conic from-red-500 via-yellow-500 to-green-500">
```

## Arbitrary Values

**Always prefer Tailwind's predefined scale:**

Check the project's `@theme` configuration for available tokens before using arbitrary values.

```typescript
// ✅ Good: predefined scale
<div className="ml-4">

// ❌ Avoid: arbitrary pixel value
<div className="ml-[16px]">
```

**General rule:** Prefer sizing scale over pixel values. Three similar lines of code is better than a premature abstraction.

## Class Merging

The common pattern is a `cn` utility combining `clsx` + `tailwind-merge`.

**Use `cn` for:**

- Static constants: `const CARD_BASE = cn("fixed classes")`
- Conditional classes: `cn("base", condition && "conditional")`
- Dynamic merging: `cn(baseClasses, className)`
- Conflict resolution: `cn("p-4", "p-6")` → `"p-6"`

**Do NOT use `cn` for:**

- Static strings in `className` attributes: `className="fixed classes"`

**Examples:**

```typescript
// ✅ Good: static string in className
<button className="rounded-lg px-4 py-2 font-medium bg-blue-600">

// ✅ Good: static constant with cn
const CARD_BASE = cn("rounded-lg border border-gray-300 p-4");
<div className={CARD_BASE} />

// ✅ Good: conditional with cn
<button className={cn(
  "rounded-lg px-4 py-2 font-medium",
  isActive ? "bg-blue-600" : "bg-gray-700",
  disabled && "opacity-50"
)} />

// ❌ Bad: unnecessary cn for static className attribute
<button className={cn("rounded-lg px-4 py-2 font-medium")} />
```

## Image Sizing

Use Tailwind size classes instead of pixel values for `Image` components.

```typescript
// ✅ Good: Tailwind units
<Image src={src} alt={alt} className="size-16" />
<Image src={src} alt={alt} className="w-24 h-auto" />

// ❌ Bad: pixel values
<Image src={src} alt={alt} width={64} height={64} />
```

## Z-Index

Define z-index values as CSS custom properties in `@theme`, then reference them with the `z-(--z-*)` syntax.

**Never use arbitrary z-index numbers:**

```typescript
// ✅ Good: theme z-index value
<div className="z-(--z-modal)">
<div className="z-(--z-sticky)">

// ❌ Bad: arbitrary z-index numbers
<div className="z-[100]">
<div className="z-[9999]">
```

**Define z-index tokens in CSS:**

```css
@theme {
  --z-base: 0;
  --z-sticky: 10;
  --z-modal: 100;
  --z-tooltip: 1000;
}
```

## Dark Mode

Use the plain `dark:` variant for dark mode styles.

**Pattern:**

Write light mode styles first, then add dark mode overrides.

```typescript
// ✅ Good: light mode first, then dark override
<div className="bg-white text-gray-900 dark:bg-gray-900 dark:text-white">

// ❌ Avoid: dark mode first (less readable)
<div className="dark:bg-gray-900 dark:text-white bg-white text-gray-900">
```
