# tailwind-variants Best Practices

## Overview

**tailwind-variants** provides a first-class variant API for composable, type-safe component styling with automatic Tailwind class conflict resolution.

**Key Benefits:**

- Type-safe variant definitions with full TypeScript support
- Automatic class conflict resolution (via tailwind-merge)
- 37-62% faster than alternatives (v2+), 500x faster with v3
- Framework-agnostic (React, Vue, Svelte, vanilla JS)

## Installation

```bash
bun add tailwind-variants
```

**Build Options:**

```typescript
// Original build (with tailwind-merge, automatic conflict resolution)
import { tv } from "tailwind-variants";

// Lite build (~80% smaller, no conflict resolution)
import { tv } from "tailwind-variants/lite";
```

## Core API

### Basic Usage

```typescript
import { tv } from "tailwind-variants";

const button = tv({
  base: "font-medium rounded-lg transition-colors",
  variants: {
    color: {
      primary: "bg-primary text-white hover:bg-primary/90",
      secondary: "bg-secondary text-white hover:bg-secondary/90",
    },
    size: {
      sm: "text-sm px-3 py-1.5",
      md: "text-base px-4 py-2",
      lg: "text-lg px-6 py-3",
    },
  },
  defaultVariants: {
    color: "primary",
    size: "md",
  },
});

// Usage
button(); // Uses defaults
button({ color: "secondary", size: "lg" });
```

### Boolean Variants

Use boolean variants for conditional states:

```typescript
const input = tv({
  base: "w-full rounded-md border px-3 py-2",
  variants: {
    disabled: {
      true: "opacity-50 cursor-not-allowed",
    },
    error: {
      true: "border-red-500 focus:ring-red-500",
    },
  },
});

// Usage
input({ disabled: true, error: false });
```

### Compound Variants

Apply styles when multiple conditions are met:

```typescript
const button = tv({
  variants: {
    color: {
      primary: "bg-blue-500",
      secondary: "bg-gray-500",
    },
    disabled: {
      true: "opacity-50",
    },
  },
  compoundVariants: [
    {
      color: "primary",
      disabled: true,
      class: "bg-blue-300", // Override when both conditions match
    },
  ],
});
```

## Slots API (Multi-Part Components)

Use slots for complex components with multiple elements:

```typescript
const card = tv({
  slots: {
    base: "rounded-lg shadow-md bg-white overflow-hidden",
    header: "px-6 py-4 border-b border-gray-200",
    body: "px-6 py-4",
    footer: "px-6 py-4 bg-gray-50 border-t border-gray-200",
  },
  variants: {
    size: {
      sm: {
        header: "px-4 py-2 text-sm",
        body: "px-4 py-2 text-sm",
        footer: "px-4 py-2 text-sm",
      },
      lg: {
        header: "px-8 py-6 text-lg",
        body: "px-8 py-6 text-lg",
        footer: "px-8 py-6 text-lg",
      },
    },
  },
});

// Usage
const { base, header, body, footer } = card({ size: "lg" });

// In React
<div className={base()}>
  <div className={header()}>Header</div>
  <div className={body()}>Content</div>
  <div className={footer()}>Footer</div>
</div>;
```

### Compound Slots

Apply classes to multiple slots based on conditions:

```typescript
const card = tv({
  slots: {
    base: "rounded-lg",
    header: "p-4",
    body: "p-4",
  },
  variants: {
    bordered: {
      true: {},
    },
  },
  compoundSlots: [
    {
      slots: ["header", "body"], // Apply to both
      bordered: true,
      class: "border border-gray-200",
    },
  ],
});
```

### Slot Overrides

Override individual slots at runtime:

```typescript
const { base, header, body } = card();

<div className={base()}>
  <div className={header({ class: "bg-blue-50" })}>Custom styled header</div>
  <div className={body()}>Default styled body</div>
</div>;
```

## Component Composition

### Using `extend` (Recommended)

The `extend` property provides type-safe composition:

```typescript
const baseButton = tv({
  base: "font-medium rounded transition-colors",
  variants: {
    size: {
      sm: "text-sm px-3 py-1",
      md: "text-base px-4 py-2",
    },
  },
});

const primaryButton = tv({
  extend: baseButton, // Inherits all config
  base: "bg-primary text-white hover:bg-primary/90",
  variants: {
    outlined: {
      true: "bg-transparent border border-primary text-primary",
    },
  },
});

// Has both 'size' and 'outlined' variants
primaryButton({ size: "sm", outlined: true });
```

**Benefits:**

- Automatic merging of variants, slots, defaultVariants, and compoundVariants
- Full TypeScript autocomplete
- Override inherited configurations

## Responsive Variants

Use Tailwind's responsive prefixes directly in variants:

```typescript
const container = tv({
  base: "px-4 md:px-6 lg:px-8",
  variants: {
    maxWidth: {
      sm: "max-w-sm md:max-w-md",
      md: "max-w-md md:max-w-lg",
      lg: "max-w-lg md:max-w-xl lg:max-w-2xl",
    },
  },
});
```

## TypeScript Integration

### Extract Variant Types

Use `VariantProps` to generate type-safe props:

```typescript
import { type VariantProps } from "tailwind-variants";

const button = tv({
  variants: {
    color: { primary: "...", secondary: "..." },
    size: { sm: "...", md: "...", lg: "..." },
  },
});

type ButtonVariants = VariantProps<typeof button>;
// { color?: "primary" | "secondary"; size?: "sm" | "md" | "lg" }

interface ButtonProps extends ButtonVariants {
  children: React.ReactNode;
}
```

### Required Variants

Make specific variants required:

```typescript
type ButtonVariants = VariantProps<typeof button>;

interface ButtonProps
  extends Omit<ButtonVariants, "size">,
    Required<Pick<ButtonVariants, "size">> {
  children: React.ReactNode;
}
```

### Type Inference

Use `as const` for proper type inference:

```typescript
const sizes = {
  sm: "text-sm px-3 py-1",
  md: "text-base px-4 py-2",
} as const;

const button = tv({
  variants: { size: sizes },
});
```

## Configuration

### Local Configuration

```typescript
const button = tv(
  {
    base: "rounded",
  },
  {
    twMerge: true, // Enable tailwind-merge
    twMergeConfig: {}, // Custom tailwind-merge config
  }
);
```

### Global Configuration

```typescript
import { defaultConfig } from "tailwind-variants";

defaultConfig.twMerge = false;
```

### Custom Instance with `createTV`

Create isolated configurations:

```typescript
import { createTV } from "tailwind-variants";

const tv = createTV({
  twMerge: true,
  twMergeConfig: {
    theme: {
      colors: ["primary", "secondary"],
      spacing: ["edge", "section"],
    },
  },
});
```

## Common Patterns

### Button Component

```typescript
const button = tv({
  base: "inline-flex items-center justify-center font-medium transition-colors focus-visible:outline-hidden disabled:pointer-events-none disabled:opacity-50",
  variants: {
    variant: {
      default: "bg-primary text-white hover:bg-primary/90",
      destructive: "bg-red-500 text-white hover:bg-red-600",
      outline: "border border-input bg-surface-950 hover:bg-accent",
      ghost: "hover:bg-accent hover:text-accent-foreground",
    },
    size: {
      default: "h-10 px-4 py-2",
      sm: "h-9 rounded-md px-3",
      lg: "h-11 rounded-md px-8",
      icon: "size-10",
    },
  },
  defaultVariants: {
    variant: "default",
    size: "default",
  },
});
```

### Card Component with Slots

```typescript
const card = tv({
  slots: {
    base: "rounded-lg border bg-card text-card-foreground shadow-sm",
    header: "flex flex-col gap-1.5 p-6",
    title: "text-2xl font-semibold leading-none tracking-tight",
    description: "text-sm text-muted-foreground",
    content: "p-6 pt-0",
    footer: "flex items-center p-6 pt-0",
  },
});

const { base, header, title, description, content, footer } = card();
```

### Input Component with States

```typescript
const input = tv({
  base: "flex w-full rounded-md border bg-surface-950 px-3 py-2 text-sm transition-colors",
  variants: {
    variant: {
      default: "border-input focus-visible:outline-hidden focus-visible:ring-3 focus-visible:ring-ring",
      error: "border-red-500 focus-visible:ring-red-500",
    },
    disabled: {
      true: "cursor-not-allowed opacity-50",
    },
  },
  defaultVariants: {
    variant: "default",
  },
});
```

## Best Practices

01. **Use `extend` for composition** - Provides type safety and automatic config merging
02. **Leverage compound variants** - Handle complex conditional styling declaratively
03. **Use slots for multi-part components** - Enables granular control
04. **Define default variants** - Reduces repetition in usage
05. **Use boolean variants for states** - Clean API for conditional states
06. **Keep original build unless bundle size critical** - Automatic conflict resolution prevents bugs
07. **Use `VariantProps` for TypeScript** - Extract types automatically
08. **Use `as const` for external definitions** - Ensures proper type inference
09. **Apply responsive prefixes in variants** - Leverage Tailwind's responsive utilities
10. **Reuse `tv()` instances** - Define once, use multiple times

## API Reference

```typescript
tv(options, config);
```

**Options:**

| Property           | Type                                                               | Description                  |
| ------------------ | ------------------------------------------------------------------ | ---------------------------- |
| `base`             | ClassValue                                                         | Base component styles        |
| `slots`            | Record\<string, ClassValue>                                        | Named style sections         |
| `variants`         | Record\<string, Record\<string, ClassValue>>                       | Conditional style variants   |
| `defaultVariants`  | Record\<string, any>                                               | Default variant values       |
| `compoundVariants` | Array\<{ \[variant\]: value, class: ClassValue }>                  | Multi-condition combinations |
| `compoundSlots`    | Array\<{ slots: string[], \[variant\]: value, class: ClassValue }> | Multi-slot combinations      |
| `extend`           | TVReturnType                                                       | Extend another component     |

**Config:**

| Property        | Type          | Default | Description                      |
| --------------- | ------------- | ------- | -------------------------------- |
| `twMerge`       | boolean       | true    | Enable class conflict resolution |
| `twMergeConfig` | TwMergeConfig | {}      | Custom tailwind-merge config     |

## Performance Considerations

- **v3 is 500x faster** when using tailwind-merge (vs v2)
- **Reuse `tv()` instances** - Define components once
- **Avoid dynamic class generation** - Pre-define variants
- **Use compound variants judiciously** - Each adds complexity
- **Consider lite build** - ~80% smaller if conflict resolution not needed

## Troubleshooting

**Classes not applying:**

- Ensure `twMerge` is enabled and `tailwind-merge` is installed
- Check class specificity and order
- Verify Tailwind config includes component files

**TypeScript errors:**

- Use `as const` for external variant definitions
- Use `VariantProps` to extract types
- Verify all variant keys match between definition and usage

**Conflict resolution issues:**

- Use original build (not lite) for automatic merging
- Configure `twMergeConfig` for custom theme values
- Ensure `tailwind-merge` is installed
