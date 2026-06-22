# Quick Utilities (Functions, Deprecations)

> When to read: pull this in when using small utility helpers like `constVoid`, or checking the running list of Effect
> deprecations.

## Utility Functions

```typescript
import { constVoid as noop } from "effect/Function"

// constVoid returns undefined, useful as a no-operation callback
noop()  // undefined

// Common use cases:
Effect.tap(effect, noop)              // Ignore value, just run effect
Promise.catch(noop)                   // Swallow errors
eventEmitter.on("event", noop)        // Register empty handler
```

## Deprecations

- **`BigDecimal.fromNumber`** — Use `BigDecimal.unsafeFromNumber` instead (3.11.0+)
- **`Schema.annotations()`** — Now removes previously set identifier annotations; identifiers are tied to the schema's
  `ast` reference only (3.17.10)
