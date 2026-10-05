# Critical Effect 4 Rules

Read this before changing nontrivial Effect code. These rules protect semantics that ordinary TypeScript intuition and
v3 habits often get wrong. Use the installed package source for exact combinator signatures.

## Effect Failures Are Not Thrown Exceptions

The Effect error channel represents an Effect failure yielded inside `Effect.gen`. An ordinary `try/catch` around
`yield*` does not recover it.

```ts
// Wrong: the catch block does not handle an Effect failure.
Effect.gen(function* () {
  try {
    return yield* program;
  } catch {
    return fallback;
  }
});
```

Use `Effect.catchTag`, `Effect.catchTags`, `Effect.catch`, `Effect.result`, or `Effect.exit` according to whether the
caller should recover, map, or inspect the failure. v3 names such as `catchAll`, `catchAllCause`, `catchSome`, and
`Effect.either` no longer exist. Wrap foreign throwing code with `Effect.try` or `Effect.tryPromise` at the boundary
where it enters Effect.

## Preserve Typed Failures

Model expected failures with tagged domain types rather than the global `Error` class. Use `Schema.TaggedError` when the
failure crosses an encoding, persistence, API, or documentation boundary. For internal-only failures, use
`Data.TaggedError`.

Do not use `as any`, `as never`, double assertions, or widened `Error` channels to make an Effect typecheck. Fix the
service, error, or environment type that produced the mismatch. A narrow assertion at a poorly typed external boundary
needs a documented reason.

## Keep Defects Out of Expected Error Mapping

`Cause` is a flat `reasons` array of `Fail`, `Die`, and `Interrupt`. There is no `Sequential`/`Parallel` tree to
traverse. Use `Effect.mapError` or tagged recovery for expected failures. Use `Effect.catchCause` only at a deliberate
runtime, reporting, or supervision boundary where handling the whole cause is the requirement.

Do not silently convert a required audit, billing, persistence, authorization, or notification effect to `Effect.void`.
Propagate or translate its expected failure. Fallback values are appropriate only when the product semantics make the
operation optional.

## Yield Only Effects

`Option`, `Result` (v3 `Either`), `Ref`, `Deferred`, and `Fiber` are plain values, not Effects. Convert explicitly:
`Effect.fromOption`, `Effect.fromResult`, `Ref.get`, `Deferred.await`, `Fiber.join`. Services and `Config` remain
yieldable.

## Keep Pure Work Pure

Do not wrap safe array transformations, constants, path manipulation, or other deterministic pure work in `Effect.try`.
Use `Effect.sync` for synchronous observable effects and `Effect.try` only for code that can throw. `Equal.equals` is
structural by default for plain objects, arrays, `Map`, `Set`, and `Date`. Do not add `Data` wrappers only for equality.

## Make Generator Termination Explicit

Use `return yield*` for failures and interruption inside conditional generator branches. The runtime stops on the failed
yield either way, but the explicit return preserves control-flow clarity and avoids misleading unreachable code.

```ts
Effect.gen(function* () {
  if (!isAuthorized) {
    return yield* Effect.fail("Unauthorized");
  }
  return yield* performAction;
});
```

For absence modeling, follow [option-null.md](option-null.md) and normalize once at the system boundary.
