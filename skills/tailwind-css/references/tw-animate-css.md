# tw-animate-css Reference

Tailwind CSS v4 compatible animation library.

**Package:** `tw-animate-css` ([npm](https://www.npmjs.com/package/tw-animate-css) | [GitHub](https://github.com/Wombosvideo/tw-animate-css))

## Setup

Import in the project's main CSS file:

```css
@import "tw-animate-css";
```

## Animation Base Classes

Enter/exit animations require a base class:

| Base Class    | Purpose                |
| ------------- | ---------------------- |
| `animate-in`  | Enable enter animation |
| `animate-out` | Enable exit animation  |

## Animation Utilities

### Fade

| Class      | Effect          |
| ---------- | --------------- |
| `fade-in`  | Fade in (enter) |
| `fade-out` | Fade out (exit) |

### Zoom

| Class      | Effect          |
| ---------- | --------------- |
| `zoom-in`  | Zoom in (enter) |
| `zoom-out` | Zoom out (exit) |

### Spin

| Class      | Effect          |
| ---------- | --------------- |
| `spin-in`  | Spin in (enter) |
| `spin-out` | Spin out (exit) |

### Slide

Slide utilities accept a direction and distance value.

**Directions:**

- `in-from-top`, `in-from-bottom`, `in-from-left`, `in-from-right`
- `out-to-top`, `out-to-bottom`, `out-to-left`, `out-to-right`

**Value Types:**

| Type              | Example                           | Notes                              |
| ----------------- | --------------------------------- | ---------------------------------- |
| Integer spacing   | `slide-in-from-bottom-4`          | Uses Tailwind spacing scale (1-96) |
| Arbitrary length  | `slide-in-from-bottom-[0.625rem]` | Any CSS length value               |
| Arbitrary percent | `slide-in-from-bottom-[10%]`      | Percentage of element              |

**Spacing Scale Gotcha:**

Decimal spacing values like `2.5` are NOT supported directly:

```html
<!-- ❌ Invalid: decimal spacing not recognized -->
<div class="slide-in-from-bottom-2.5 animate-in">

<!-- ✅ Valid: use arbitrary value instead -->
<div class="slide-in-from-bottom-[0.625rem] animate-in">

<!-- ✅ Valid: use integer spacing -->
<div class="slide-in-from-bottom-2 animate-in">
```

**Spacing to rem conversion:** `value × 0.25rem`

- `2` = `0.5rem` (8px)
- `2.5` = `0.625rem` (10px) → use `[0.625rem]`
- `3` = `0.75rem` (12px)

## Duration and Timing

Control animation timing with standard Tailwind utilities:

| Utility        | Purpose            |
| -------------- | ------------------ |
| `duration-150` | Animation duration |
| `delay-100`    | Animation delay    |
| `ease-in`      | Timing function    |
| `ease-out`     | Timing function    |
| `ease-in-out`  | Timing function    |

## Fill Mode

Control element visibility after animation:

| Class                 | Effect                         |
| --------------------- | ------------------------------ |
| `fill-mode-none`      | No fill mode                   |
| `fill-mode-forwards`  | Keep end state                 |
| `fill-mode-backwards` | Apply start state immediately  |
| `fill-mode-both`      | Combine forwards and backwards |

## Common Patterns

### Enter Animation

```html
<div class="fade-in slide-in-from-bottom-4 duration-300 animate-in">
  Content slides up and fades in
</div>
```

### Exit Animation

```html
<div class="fade-out slide-out-to-bottom-4 duration-200 animate-out">
  Content slides down and fades out
</div>
```

### Responsive Animation

```html
<div class="lg:fade-in lg:slide-in-from-bottom-[0.625rem] lg:animate-in">
  Only animates on lg+ breakpoints
</div>
```

### Combined Animation

```html
<div class="fade-in zoom-in-50 slide-in-from-bottom-8 duration-500 animate-in">
  Multiple effects combined
</div>
```

## Quick Reference

| Want to...                     | Use                                 |
| ------------------------------ | ----------------------------------- |
| Fade in                        | `fade-in animate-in`                |
| Slide up on enter              | `slide-in-from-bottom-4 animate-in` |
| Slide down on exit             | `slide-out-to-bottom-4 animate-out` |
| Use decimal spacing (e.g. 2.5) | `slide-in-from-bottom-[0.625rem]`   |
| Control duration               | `duration-300`                      |
| Add delay                      | `delay-150`                         |
| Keep end state                 | `fill-mode-forwards`                |
