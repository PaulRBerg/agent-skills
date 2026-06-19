---
disable-model-invocation: false
name: effect-ts
user-invocable: true
description: Use for nontrivial Effect-TS work including services/layers, typed errors, Schema/JSONSchema, Config, runtime/concurrency, @effect/vitest, @effect/ai, @effect/sql, or @prb/effect-next.
---

# Effect-TS Expert

Expert guidance for functional programming with the Effect library, covering error handling, dependency injection,
composability, testing, and runtime-boundary patterns.

## Fast Path

Use this skill for nontrivial Effect work. Do not route through this skill just because a file imports from `effect`; use
it when the change depends on Effect semantics such as services, layers, typed errors, Schema, Config, runtime/concurrency,
streams, or Effect-aware tests.

For small code edits:

1. Inspect local project patterns first.
2. Read `./references/critical-rules.md` before writing or changing Effect code.
3. Open only the reference files that match the task.
4. Run the narrowest project check that proves the changed Effect behavior.

## Upstream Source Check

Check the Effect source at `~/.effect` only when the task needs upstream API details, changelog verification, or a complex
type/runtime question that local project patterns do not answer.

If `~/.effect` is required but missing, stop and inform the user. Clone it before proceeding:

```bash
git clone https://github.com/Effect-TS/effect.git ~/.effect
```

## Upstream Baseline

Last checked against `~/.effect` HEAD `05d72eab7` from 2026-06-05:

- `effect@3.21.3`
- `@effect/ai@0.36.0`
- `@effect/ai-openai@0.40.0`
- `@effect/platform@0.96.1`
- `@effect/sql@0.51.1`
- `@effect/rpc@0.75.1`
- `@effect/cluster@0.59.0`

Your local `~/.effect` checkout need not match these exact versions. Drift is expected and fine **as long as the major
versions match**. For `effect` that means the `3.x` line. For the `0.x` `@effect/*` packages, semver treats the leading
non-zero segment as the break boundary, so match the minor too (e.g. `@effect/ai@0.36.x`). Patch differences, and minor
differences on stable packages, won't invalidate this skill's guidance; only a break-boundary bump warrants caution.

Local `~/.effect` drift is usually fine for routine project work. If `git -C ~/.effect log -1 --oneline` is newer and the
task depends on upstream behavior, inspect the touched package changelogs and commits before relying on this skill.
Capture public API or guidance changes in a reference file.

## Research Strategy

Effect-TS has many ways to accomplish the same task. For moderate to high complexity tasks, research enough to choose the
least surprising pattern that fits the current codebase. Prefer parallel local reads/searches. Use subagents only when the
environment explicitly supports them and the task has separable research tracks.

### Research Sources (Priority Order)

1. **Codebase Patterns First** — Examine similar patterns in the current project before implementing. If Effect patterns
   exist in the codebase, follow them for consistency. If no patterns exist, skip this step.

2. **Effect Source Code** — For complex type errors, unclear behavior, or implementation details, examine the relevant
   package source under `~/.effect/packages/<package>/src/`. For core `Effect`, use `~/.effect/packages/effect/src/`.

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

- Focus on canonical, readable, and maintainable solutions rather than clever optimizations
- Verify suggested approaches against existing codebase patterns for consistency (if patterns exist)
- When multiple approaches are possible, prefer the one already used locally unless it is clearly flawed

## Reference Routing

Open references selectively:

| Task shape                                                     | Read                                |
| -------------------------------------------------------------- | ----------------------------------- |
| Writing/changing Effect code                                   | `./references/critical-rules.md`    |
| Services, Layers, `Effect.Service`, `Context.Tag`, `Effect.fn` | `./references/services-layers.md`   |
| Config, env vars, secrets, custom providers                    | `./references/config.md`            |
| Schema decoding, JSON Schema, AI parameter shapes              | `./references/schema-jsonschema.md` |
| `@effect/vitest`, `TestClock`, sleeps/retries, fibers in tests | `./references/testing.md`           |
| Resources, scheduling, refs, concurrency, `SubscriptionRef`    | `./references/runtime.md`           |
| Streams, backpressure, bounded consumption                     | `./references/streams.md`           |
| Pattern matching, tagged unions, `Data.taggedEnum`             | `./references/pattern-matching.md`  |
| `@effect/ai` tools/providers/OpenAI integration                | `./references/ai.md`                |
| `@effect/sql`, `SqlSchema`, repository row decoding            | `./references/sql.md`               |
| `@effect/platform`, `@effect/rpc`, deployment runtimes         | `./references/platform-rpc.md`      |
| `@prb/effect-next` / Next.js App Router                        | `./references/next-js.md`           |
| `@effect-atom/*` React state                                   | `./references/effect-atom.md`       |
| Array helpers, `Order`, tiny utility functions, deprecations   | `./references/quick-utils.md`       |
| Upstream drift or recent package behavior                      | `./references/recent-upstream.md`   |

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
- Prefer `Schema.TaggedError` for domain/API errors that cross serialization or HTTP boundaries
- Use `Data.TaggedError` for internal, non-encoded errors when Schema integration is unnecessary
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
- Prefer `Schema.Class` for domain and API models that need construction, validation, encoding, or equality
- Use `Effect.gen` for readable sequential code
- Implement proper testing patterns using Effect's testing utilities
- Prefer `Effect.fn()` for automatic telemetry and better stack traces

### Boundary Refactors

- Use Effect services at IO/runtime boundaries where dependency injection, testability, or resource safety improves the
  design.
- Do not make pure helpers, module constants, path strings, or tiny build-time utilities effectful just to replace Node or
  platform APIs.
- `@effect/platform` services such as `FileSystem` and `Path` are environment requirements. Keep them inside existing
  service/runtime boundaries unless widening a function's environment is a deliberate design improvement.
- Preserve local domain facades when they already centralize Effect services, for example filesystem, reporter, logger, or
  config services.

## Critical Rules

Read `./references/critical-rules.md` before writing or changing nontrivial Effect code. Key guidelines:

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
- **Layer reuse surprises** — Layers memoize by object identity; use `Layer.fresh` only when needed →
  [services-layers.md](./references/services-layers.md)

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
// Domain/API errors that cross boundaries: prefer Schema.TaggedError
class UserNotFoundError extends Schema.TaggedError<UserNotFoundError>()(
  "UserNotFoundError",
  {
    userId: Schema.String
  }
) {
  get message() {
    return `User not found: ${this.userId}`
  }
}

// Internal-only errors may use Data.TaggedError
class CacheMissError extends Data.TaggedError("CacheMissError")<{
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
- **`./references/sql.md`** — `@effect/sql`, `SqlSchema`, row decoding, repository patterns
- **`./references/streams.md`** — Stream patterns and backpressure gotchas
- **`./references/testing.md`** — Vitest deterministic testing patterns
