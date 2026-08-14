# Testing Effect 3 with Vitest

Use `@effect/vitest` for Effect programs. Keep assertions inside the returned Effect and run only the tests covering the
changed behavior.

## Choose the Test Runtime

- `it.effect` provides Effect's test environment, including `TestClock`.
- `it.live` uses live runtime services.
- `it.scoped` combines the test environment with a Scope.
- `it.scopedLive` combines live services with a Scope.
- `layer(...)` shares one Layer across a test block; use nested `it.layer(...)` when a subgroup needs another Layer.

Use regular `it` for pure synchronous tests. Do not call `Effect.runPromise`, `runSync`, or another runtime launcher
inside an Effect test; that escapes the test runtime and can silently replace test services.

## Advance Virtual Time Deliberately

Under `it.effect`, time does not advance until the test calls `TestClock.adjust` or `TestClock.setTime`. Fork the effect
that sleeps, retries, polls, or repeats, then advance enough time for the whole schedule and join the fiber. Use
`it.live` only when wall-clock behavior is genuinely under test.

Production Effect code should read time through `Clock` or `DateTime`, not `Date.now`, so tests can control it.

## Prove Fiber Startup Before Opening Gates

Forking schedules a fiber; it does not prove that the fiber reached the intended coordination point. For overlap,
deduplication, or sharing tests, have the worker complete a `started` Deferred immediately before awaiting a separate
gate. Await `started` before opening the gate.

## Bound and Release

Bound infinite streams and polling loops. Join or interrupt every fiber. Use scoped tests when code allocates scoped
resources, and let Layer/test scopes own finalizers instead of launching detached runtimes.
