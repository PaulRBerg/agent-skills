# Testing Effect 4 with Vitest

For Effect programs, use `@effect/vitest` 4.x (same version as `effect`, requires Vitest 5). Keep assertions inside the
returned Effect and run only the tests covering the changed behavior.

## Choose the Test Runtime

- `it.effect` provides a `Scope` and the test environment (`TestClock`, `TestConsole`).
- `it.live` provides a `Scope` and live runtime services.
- v3 `it.scoped` and `it.scopedLive` are gone. Use `it.effect` and `it.live`.
- `layer(...)` shares one Layer across a test block. When a subgroup needs another Layer, use nested `it.layer(...)`.
  Layer blocks include the test environment unless `excludeTestServices: true`.

Use regular `it` for pure synchronous tests. Do not call `Effect.runPromise`, `runSync`, or another runtime launcher
inside an Effect test. That escapes the test runtime and can silently replace test services.

`effect/FastCheck` was removed. Use `it.prop` or `it.effect.prop` with Schemas or `effect/Arbitrary` values
(`Arbitrary.schema(S)`). Import `fast-check` directly only when a test needs its own API.

## Destructure Tuple Cases

`it.effect.each(cases)` passes each case as one argument, followed by the test context. For tuple rows, use a callback
such as `([label, value]) => ...`. Regular Vitest `it.each` instead spreads tuple members into separate arguments.
`@effect/vitest` 4.0 delegates to Vitest `it.for`. If versions differ, verify the installed adapter.

## Advance Virtual Time Deliberately

Import `TestClock` from `effect/testing`. Under `it.effect`, time does not advance until the test calls
`TestClock.adjust` or `TestClock.setTime`. Fork the effect that sleeps, retries, polls, or repeats with
`Effect.forkChild`, then advance enough time for the whole schedule and join the fiber. Use `it.live` or
`TestClock.withLive` only when wall-clock behavior is genuinely under test.

Production Effect code should read time through `Clock` or `DateTime`, not `Date.now`, so tests can control it.

## Prove Fiber Startup Before Opening Gates

Forking schedules a fiber. It does not prove that the fiber reached the intended coordination point. For overlap,
deduplication, or sharing tests, have the worker complete a `started` Deferred immediately before awaiting a separate
gate. Await `started` before opening the gate.

## Bound and Release

Bound infinite streams and polling loops. Join or interrupt every fiber. Effect tests are already scoped, so let the
test or Layer scope own finalizers instead of launching detached runtimes.

In `@effect/vitest/utils`, `assertSuccess`/`assertFailure` now assert `Result` values. Use `assertExitSuccess` and
`assertExitFailure` for `Exit`.
