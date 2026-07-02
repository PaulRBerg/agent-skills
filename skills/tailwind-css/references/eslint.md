# ESLint Integration

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
