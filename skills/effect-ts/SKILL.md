---
disable-model-invocation: false
name: effect-ts
user-invocable: true
description: Use for Effect-TS code and patterns including services, layers, errors, Schema/JSONSchema, @effect/ai tools, code importing from `effect`, or @prb/effect-next.
---

# Effect-TS Expert

Expert guidance for functional programming with the Effect library, covering error handling, dependency injection,
composability, and testing patterns.

## Prerequisites Check

Before starting any Effect-related work, verify the Effect-TS source code exists at `~/.effect`.

**If missing, stop immediately and inform the user.** Clone it before proceeding:

```bash
git clone https://github.com/Effect-TS/effect.git ~/.effect
```

## Upstream Baseline

Last checked against `~/.effect` HEAD `05d72eab7` from 2026-06-05:

- `effect@3.21.3`
- `@effect/ai@0.36.0`
- `@effect/ai-openai@0.40.0`
- `@effect/platform@0.96.1`
- `@effect/rpc@0.75.1`
- `@effect/cluster@0.59.0`

Your local `~/.effect` checkout need not match these exact versions. Drift is expected and fine **as long as the major
versions match**. For `effect` that means the `3.x` line. For the `0.x` `@effect/*` packages, semver treats the leading
non-zero segment as the break boundary, so match the minor too (e.g. `@effect/ai@0.36.x`). Patch differences, and minor
differences on stable packages, won't invalidate this skill's guidance; only a break-boundary bump warrants caution.

If `git -C ~/.effect log -1 --oneline` is newer, inspect the touched package changelogs and commits before relying on
this skill. Capture public API or guidance changes in a reference file.

## Research Strategy

Effect-TS has many ways to accomplish the same task. Proactively research best practices using the Task tool to spawn
research agents when working with Effect patterns, especially for moderate to high complexity tasks.

### Research Sources (Priority Order)

1. **Codebase Patterns First** — Examine similar patterns in the current project before implementing. If Effect patterns
   exist in the codebase, follow them for consistency. If no patterns exist, skip this step.

2. **Effect Source Code** — For complex type errors, unclear behavior, or implementation details, examine the Effect
   source at `~/.effect/packages/effect/src/`. This contains the core Effect logic and modules.

3. **Package Changelogs** — When behavior changed recently, read the relevant changelog under `~/.effect/packages/*/`
   before inferring from old examples.

### When to Research

**HIGH Priority (Always Research):**

- Implementing Services, Layers, or complex dependency injection
- Error handling with multiple error types or complex error hierarchies
- Stream-based operations and reactive patterns
- Resource management with scoped effects and cleanup
- Concurrent/parallel operations and performance-critical code
- Testing patterns, especially unfamiliar test scenarios

**MEDIUM Priority (Research if Complex):**

- Refactoring imperative code (try-catch, promises) to Effect patterns
- Adding new service dependencies or restructuring service layers
- Custom error types or extending existing error hierarchies
- Integrations with external systems (databases, APIs, third-party services)

### Research Approach

- Spawn multiple concurrent Task agents when investigating multiple related patterns
- Focus on finding canonical, readable, and maintainable solutions rather than clever optimizations
- Verify suggested approaches against existing codebase patterns for consistency (if patterns exist)
- When multiple approaches are possible, research to find the most idiomatic Effect-TS solution

## Codebase Pattern Discovery

When working in a project that uses Effect, check for existing patterns before implementing new code:

1. **Search for Effect imports** — Look for files importing from `'effect'` to understand existing usage
2. **Identify service patterns** — Find how Services and Layers are structured in the project
3. **Note error handling conventions** — Check how errors are defined and propagated
4. **Examine test patterns** — Look at how Effect code is tested in the project

**If no Effect patterns exist in the codebase**, proceed using canonical patterns from the Effect source and examples.
Do not block on missing codebase patterns.

## Effect Principles

Apply these core principles when writing Effect code:

### Error Handling

- Use Effect's typed error system instead of throwing exceptions
- Define descriptive error types with proper error propagation
- Use `Effect.fail`, `Effect.catchTag`, `Effect.catchAll` for error control flow
- See `./references/critical-rules.md` for forbidden patterns

### Dependency Injection

- Implement dependency injection using Services and Layers
- Define services with `Context.Tag`
- Compose layers with `Layer.merge`, `Layer.provide`
- Use `Effect.provide` to inject dependencies

### Composability

- Leverage Effect's composability for complex operations
- Use appropriate constructors: `Effect.succeed`, `Effect.fail`, `Effect.tryPromise`, `Effect.try`
- Apply proper resource management with scoped effects
- Chain operations with `Effect.flatMap`, `Effect.map`, `Effect.tap`

### Code Quality

- Write type-safe code that leverages Effect's type system
- Use `Effect.gen` for readable sequential code
- Implement proper testing patterns using Effect's testing utilities
- Prefer `Effect.fn()` for automatic telemetry and better stack traces

## Critical Rules

Read and internalize `./references/critical-rules.md` before writing any Effect code. Key guidelines:

- **INEFFECTIVE:** try-catch in Effect.gen (Effect failures aren't thrown)
- **AVOID:** Type assertions (as never/any/unknown)
- **RECOMMENDED:** `return yield*` pattern for errors (makes termination explicit)

## Common Failure Modes

Quick links to patterns that frequently cause issues:

- **SubscriptionRef version mismatch** — `unsafeMake is not a function` → [runtime.md](./references/runtime.md)
- **Cancellation vs Failure** — Interrupts aren't errors → [Error Taxonomy](#error-taxonomy)
- **Option vs null** — Use Option internally, null at boundaries → [option-null.md](./references/option-null.md)
- **Stream backpressure** — Infinite streams hang → [streams.md](./references/streams.md)
- **JSON Schema closed records** — `Schema.Record(String, Never)` emits no extra properties →
  [schema-jsonschema.md](./references/schema-jsonschema.md)
- **No-parameter AI tools** — Use `Tool.EmptyParams` or omit `parameters` → [ai.md](./references/ai.md)

## Explaining Solutions

When providing solutions, explain the Effect-TS concepts being used and why they're appropriate for the specific use
case. If encountering patterns not covered in the documentation, suggest improvements while maintaining consistency with
existing codebase patterns (when they exist).

## Quick Reference

### Creating Effects

```typescript
Effect.succeed(value)           // Wrap success value
Effect.fail(error)              // Create failed effect
Effect.tryPromise(fn)           // Wrap promise-returning function
Effect.try(fn)                  // Wrap synchronous throwing function
Effect.sync(fn)                 // Wrap synchronous non-throwing function
```

### Composing Effects

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

### Error Handling

```typescript
// Define typed errors with Data.TaggedError (preferred)
class UserNotFoundError extends Data.TaggedError("UserNotFoundError")<{
  userId: string
}> {}

// Direct yield of errors (no Effect.fail wrapper needed)
Effect.gen(function* () {
  if (!user) {
    return yield* new UserNotFoundError({ userId })
  }
})

Effect.catchTag(effect, tag, fn) // Handle specific error tag
Effect.catchAll(effect, fn)      // Handle all errors
Effect.result(effect)            // Convert to Exit value
Effect.orElse(effect, alt)       // Fallback effect
```

### Error Taxonomy

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

### Pattern Matching (Match Module)

When you need to use Effect's Match module for pattern matching, see [references/pattern-matching.md](references/pattern-matching.md).

### Schema and JSON Schema

For schema decoding, JSON Schema generation, closed object shapes, and `Schema.Record({ key: Schema.String, value: Schema.Never })`, see [references/schema-jsonschema.md](references/schema-jsonschema.md).

### AI Tooling

For `@effect/ai` tool definitions, empty tool parameters, OpenAI strict schema behavior, and prompt cache enum gotchas,
see [references/ai.md](references/ai.md).

### Services and Layers / Generator Pattern

For service definition patterns (`Context.Tag`, `Effect.Service`, `Context.Reference`, `Context.ReadonlyTag`) and the generator pattern (`Effect.gen`, `Effect.fn`), see [references/services-layers.md](references/services-layers.md).

### Runtime Patterns (Resource Management, Duration, Scheduling, State, SubscriptionRef, Concurrency)

For resource lifecycles, durations, scheduling, state management, reactive refs, and concurrency primitives, see [references/runtime.md](references/runtime.md).

### Configuration & Environment Variables

When you need to read configuration with `Config`, handle secrets via `Redacted`, or wire custom config providers, see [references/config.md](references/config.md).

### Quick Utilities (Array Operations, Utility Functions, Deprecations)

For Effect's `Array`/`Order` sorting helpers, small utility functions like `constVoid`, and the running list of deprecations, see [references/quick-utils.md](references/quick-utils.md).

### Platform and RPC

For `HttpLayerRouter`, `RpcSerialization.makeMsgPack`, and deployment gotchas such as Cloudflare Workers msgpack support,
see [references/platform-rpc.md](references/platform-rpc.md).

## Additional Resources

### Local Effect Resources

- **`~/.effect/packages/effect/src/`** — Core Effect modules and implementation

### External Resources

- **Effect-Atom** — https://github.com/tim-smart/effect-atom (open in browser for reactive state management patterns)

### Reference Files

- **`./references/ai.md`** — `@effect/ai` tools, `Tool.EmptyParams`, OpenAI provider notes
- **`./references/config.md`** — `Config`, `Redacted`, and custom config providers
- **`./references/critical-rules.md`** — Forbidden patterns and mandatory conventions
- **`./references/effect-atom.md`** — Effect-Atom reactive state management for React
- **`./references/next-js.md`** — Effect + Next.js 15+ App Router integration patterns
- **`./references/option-null.md`** — Option vs null boundary patterns
- **`./references/pattern-matching.md`** — `Match` module for tagged unions and conditionals
- **`./references/platform-rpc.md`** — `@effect/platform` and `@effect/rpc` integration notes
- **`./references/quick-utils.md`** — `Array`/`Order`, utility helpers, deprecations
- **`./references/recent-upstream.md`** — Recent upstream public changes reflected by this skill
- **`./references/runtime.md`** — Resource management, Duration, Scheduling, State, SubscriptionRef, Concurrency
- **`./references/schema-jsonschema.md`** — Schema decoding and JSON Schema generation patterns
- **`./references/services-layers.md`** — Services, Layers, generator (`Effect.gen` / `Effect.fn`)
- **`./references/streams.md`** — Stream patterns and backpressure gotchas
- **`./references/testing.md`** — Vitest deterministic testing patterns
