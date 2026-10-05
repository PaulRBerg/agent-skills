# Runtime, Resources, and Concurrency

## Resource Lifetimes

Acquire resources with `Effect.acquireRelease` or `Effect.acquireUseRelease` and run them in a Scope. Put long-lived
clients and background processes in `Layer.scoped`; keep the Layer's Scope owned by the application runtime.

Every forked fiber needs an owner and a completion policy: join it, interrupt it, or place it in a Scope that closes. Do
not create fire-and-forget fibers whose failures and finalizers become invisible.

## Time and Scheduling

Use Effect `Clock`, `Duration`, and `Schedule` instead of ambient time and ad hoc timer loops. Duration inputs accept
human-readable strings; preserve the project's established representation rather than normalizing for style alone.

Choose retry schedules from failure semantics: retry only transient failures, bound attempts or elapsed time, and keep
non-retryable domain failures outside the retry predicate.

## Coordination Primitives

- `Ref` owns mutable state accessed by Effects.
- `Deferred` is a one-shot synchronization or result handoff.
- `SubscriptionRef` owns state plus a stream of changes; construct it with the safe `make` API.

Do not reach for unsafe constructors merely to avoid yielding an Effect. For concurrency-sensitive behavior, test the
coordination point explicitly rather than assuming a forked fiber has already run.
