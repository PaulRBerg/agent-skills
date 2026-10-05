# Runtime, Resources, and Concurrency

## Resource Lifetimes

Acquire resources with `Effect.acquireRelease` or `Effect.acquireUseRelease` and run them in a Scope. Put long-lived
clients and background processes in `Layer.effect`, which owns and excludes the layer Scope; keep that Scope owned by
the application runtime. `Scope.provide` replaces v3 `Scope.extend`.

Every forked fiber needs an owner and a completion policy: join it, interrupt it, or place it in a Scope that closes.
Use `Effect.forkChild` (v3 `fork`), `Effect.forkScoped`, `Effect.forkIn`, or, only for deliberately detached work,
`Effect.forkDetach` (v3 `forkDaemon`). Observe results with `Fiber.join` or `Fiber.await`; fibers are not yieldable. Do
not create fire-and-forget fibers whose failures and finalizers become invisible.

## Runtime Boundaries

`Runtime<R>` is gone. Capture services with `Effect.context<R>()` and run with `Effect.runForkWith(context)` or
`Effect.runPromiseWith(context)`; use `ManagedRuntime.make(layer)` for a long-lived boundary that owns a Layer. The core
runtime keeps the process alive while fibers are suspended, but process entrypoints should still use the platform
`runMain` for signal handling, exit codes, and error reporting.

Fiber-local settings are `Context.Reference` values, not `FiberRef`s. Read them with `yield* References.X` and scope
changes with `Effect.provideService(effect, References.MinimumLogLevel, "Warn")` or
`Layer.succeed(References.MinimumLogLevel, "Info")`.

## Time and Scheduling

Use Effect `Clock`, `Duration`, and `Schedule` instead of ambient time and ad hoc timer loops. `Duration.Input` (v3
`DurationInput`) accepts human-readable strings such as `"5 seconds"`; preserve the project's established representation
rather than normalizing for style alone.

Choose retry schedules from failure semantics: retry only transient failures, bound attempts or elapsed time, and keep
non-retryable domain failures outside the retry predicate.

## Coordination Primitives

- `Ref` owns mutable state accessed by Effects; read it with `Ref.get`.
- `Deferred` is a one-shot synchronization or result handoff; wait with `Deferred.await`.
- `SubscriptionRef` owns state plus a stream of changes; construct it with the safe `make` API.

Do not reach for `*Unsafe` constructors (v3 `unsafe*`, e.g. `Ref.makeUnsafe`, `DateTime.nowUnsafe`) merely to avoid
yielding an Effect. For concurrency-sensitive behavior, test the coordination point explicitly rather than assuming a
forked fiber has already run.
