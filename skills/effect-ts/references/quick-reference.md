# Quick Reference

> When to read: pull this in for a quick lookup of basic Effect constructors, composition combinators, error-handling
> combinators, or the error taxonomy table.

## Creating Effects

```typescript
Effect.succeed(value)           // Wrap success value
Effect.fail(error)              // Create failed effect
Effect.tryPromise(fn)           // Wrap promise-returning function
Effect.try(fn)                  // Wrap synchronous throwing function
Effect.sync(fn)                 // Wrap synchronous non-throwing function
```

## Composing Effects

```typescript
Effect.flatMap(effect, fn)      // Chain effects
Effect.map(effect, fn)          // Transform success value
Effect.tap(effect, fn)          // Side effect without changing value
Effect.all([...effects])        // Run effects (concurrency configurable)
Effect.forEach(items, fn)       // Map over items with effects

// Collect ALL errors (not just first)
Effect.all([e1, e2, e3], { mode: "validate" })  // Returns all failures

// Partial success handling
Effect.partition([e1, e2, e3])  // Returns [failures, successes]
```

## Error Handling Combinators

For domain/API error modeling (`Schema.TaggedError` vs `Data.TaggedError`), see
[critical-rules.md](critical-rules.md#avoid-global-error-in-the-effect-error-channel) and
[schema-jsonschema.md](schema-jsonschema.md#schema-backed-domain-errors).

Direct yield of errors (no `Effect.fail` wrapper needed):

```typescript
Effect.gen(function* () {
  if (!user) {
    return yield* new UserNotFoundError({ userId })
  }
})
```

```typescript
Effect.catchTag(effect, tag, fn) // Handle specific error tag
Effect.catchAll(effect, fn)      // Handle all errors
Effect.result(effect)            // Convert to Exit value
Effect.orElse(effect, alt)       // Fallback effect
```

## Error Taxonomy

Categorize errors for appropriate handling:

| Category                | Examples                   | Handling                  |
| ----------------------- | -------------------------- | ------------------------- |
| **Expected Rejections** | User cancel, deny          | Graceful exit, no retry   |
| **Domain Errors**       | Validation, business rules | Show to user, don't retry |
| **Defects**             | Bugs, assertions           | Log + alert, investigate  |
| **Interruptions**       | Fiber cancel, timeout      | Cleanup, may retry        |
| **Unknown/Foreign**     | Thrown exceptions          | Normalize at boundary     |

```typescript
// Pattern: Normalize unknown errors at boundary
const safeBoundary = Effect.catchAllDefect(effect, (defect) =>
  Effect.fail(new UnknownError({ cause: defect }))
)

// Pattern: Catch user-initiated cancellations separately
Effect.catchTag(effect, "UserCancelledError", () => Effect.succeed(null))

// Pattern: Handle interruptions differently from failures
Effect.onInterrupt(effect, () => Effect.log("Operation cancelled"))
```
