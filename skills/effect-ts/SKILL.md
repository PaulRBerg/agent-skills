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

Do not route through this skill just because a file imports from `effect`; use it only when the change depends on Effect
semantics, not just Effect's presence.

For small code edits:

1. Inspect local project patterns first.
2. Read `./references/critical-rules.md` before writing or changing Effect code — this is the one mandatory reference. Key
   guidelines:
   - **INEFFECTIVE:** try-catch in Effect.gen (Effect failures aren't thrown)
   - **AVOID:** Type assertions (as never/any/unknown)
   - **RECOMMENDED:** `return yield*` pattern for errors (makes termination explicit)
3. Open only the reference files that match the task — see [Reference Routing](#reference-routing).
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

This is the single index for every reference file — renaming or adding a reference only requires editing this table. Open
references selectively, only the row(s) that match the task:

| Task shape                                                                                                              | Read                                    |
| ----------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| Writing or changing any nontrivial Effect code (mandatory, read first)                                                  | `./references/critical-rules.md`        |
| Services, Layers, `Effect.Service`, `Context.Tag`, `Effect.fn` (layers memoize by identity — `Layer.fresh` when needed) | `./references/services-layers.md`       |
| Config, env vars, secrets, custom providers                                                                             | `./references/config.md`                |
| Schema decoding, JSON Schema, AI parameter shapes (`Schema.Record(String, Never)` for closed records)                   | `./references/schema-jsonschema.md`     |
| `@effect/vitest`, `TestClock`, sleeps/retries, fibers in tests                                                          | `./references/testing.md`               |
| Resources, scheduling, refs, concurrency, `SubscriptionRef` (version mismatch shows as `unsafeMake is not a function`)  | `./references/runtime.md`               |
| Streams, backpressure, bounded consumption (infinite streams hang without it)                                           | `./references/streams.md`               |
| Pattern matching, tagged unions, `Data.taggedEnum`                                                                      | `./references/pattern-matching.md`      |
| `@effect/ai` tools/providers/OpenAI integration (no-parameter tools: use `Tool.EmptyParams`)                            | `./references/ai.md`                    |
| `@effect/sql`, `SqlSchema`, repository row decoding                                                                     | `./references/sql.md`                   |
| `@effect/platform`, `@effect/rpc`, deployment runtimes                                                                  | `./references/platform-rpc.md`          |
| `@prb/effect-next` / Next.js App Router                                                                                 | `./references/next-js.md`               |
| `@effect-atom/*` React state                                                                                            | `./references/effect-atom.md`           |
| Array/Record reducers, filters, predicates, sorting                                                                     | `./references/collection-operations.md` |
| Tiny utility functions, deprecations                                                                                    | `./references/quick-utils.md`           |
| Option vs `null`/`undefined` at boundaries                                                                              | `./references/option-null.md`           |
| Upstream drift or recent package behavior                                                                               | `./references/recent-upstream.md`       |
| Quick lookup: basic constructors, combinators, error taxonomy (cancellation/interrupts aren't the same as failures)     | `./references/quick-reference.md`       |

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

### Dependency Injection

- Define services with `Context.Tag`
- Compose layers with `Layer.merge`, `Layer.provide`
- Use `Effect.provide` to inject dependencies

### Composability

- Use appropriate constructors: `Effect.succeed`, `Effect.fail`, `Effect.tryPromise`, `Effect.try`
- Apply proper resource management with scoped effects
- Chain operations with `Effect.flatMap`, `Effect.map`, `Effect.tap`

### Code Quality

- Prefer `Schema.Class` for domain and API models that need construction, validation, encoding, or equality
- Use `Effect.gen` for readable sequential code
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

## Explaining Solutions

When providing solutions, explain the Effect-TS concepts being used and why they're appropriate for the specific use
case. If encountering patterns not covered in the documentation, suggest improvements while maintaining consistency with
existing codebase patterns (when they exist).

## Additional Resources

### Local Effect Resources

- **`~/.effect/packages/effect/src/`** — Core Effect modules and implementation

### External Resources

- **Effect-Atom** — https://github.com/tim-smart/effect-atom (open in browser for reactive state management patterns)
