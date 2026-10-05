# Critical Effect 3 Rules

Read this before changing nontrivial Effect code. These rules protect semantics that ordinary TypeScript intuition often
gets wrong; use the installed package source for exact combinator signatures.

## Effect Failures Are Not Thrown Exceptions

An Effect failure yielded inside `Effect.gen` is represented in the Effect error channel. An ordinary `try/catch` around
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

Use `Effect.catchTag`, `Effect.catchTags`, `Effect.catchAll`, or `Effect.exit` according to whether the caller should
recover, map, or inspect the failure. Wrap foreign throwing code with `Effect.try` or `Effect.tryPromise` at the
boundary where it enters Effect.

## Preserve Typed Failures

Model expected failures with tagged domain types rather than the global `Error` class. Use `Schema.TaggedError` when the
failure crosses an encoding, persistence, API, or documentation boundary; use `Data.TaggedError` for internal-only
failures.

Do not use `as any`, `as never`, double assertions, or widened `Error` channels to make an Effect typecheck. Fix the
service, error, or environment type that produced the mismatch. A narrow assertion at a poorly typed external boundary
needs a documented reason.

## Keep Defects Out of Expected Error Mapping

`Cause` contains expected failures, defects, and interruption. Use `Effect.mapError` or tagged recovery for expected
failures. Use `catchAllCause` only at a deliberate runtime, reporting, or supervision boundary where handling the whole
cause is the requirement.

Do not silently convert a required audit, billing, persistence, authorization, or notification effect to `Effect.void`.
Propagate or translate its expected failure. Fallback values are appropriate only when the product semantics make the
operation optional.

## Keep Pure Work Pure

Do not wrap safe array transformations, constants, path manipulation, or other deterministic pure work in `Effect.try`.
Use `Effect.sync` for synchronous observable effects and `Effect.try` only for code that can throw.

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
