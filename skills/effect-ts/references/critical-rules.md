# Critical Rules for Effect-TS

These rules address common mistakes when working with Effect. Understanding why they matter helps write idiomatic Effect
code.

## INEFFECTIVE: try-catch in Effect.gen

**Avoid `try-catch` blocks inside `Effect.gen` generators for handling Effect failures.**

Effect failures are returned as exits, not thrown as JavaScript exceptions. Using try-catch will not catch Effect
failures—it only catches synchronous throws from non-Effect code.

**Problematic:**

```typescript
Effect.gen(function* () {
  try {
    const result = yield* someEffect;
  } catch (error) {
    // This catches synchronous throws only, NOT Effect failures
    // Effect failures bypass this entirely
  }
});
```

**Correct:**

```typescript
Effect.gen(function* () {
  const result = yield* Effect.result(someEffect);
  if (result._tag === "Failure") {
    // Handle error case
  }
});
```

Alternative patterns:

- `Effect.catchAll` / `Effect.catchTag` for error recovery
- `Effect.result` to inspect success/failure
- `Effect.tryPromise` / `Effect.try` for wrapping external code

## AVOID: Type Assertions

**Avoid `as never`, `as any`, or `as unknown` type assertions.**

These break TypeScript's type safety and hide real type errors. Always fix the underlying type issues instead.

**Patterns to avoid:**

```typescript
const value = something as any;
const value = something as never;
const value = something as unknown;
```

**Correct approach:**

- Use proper generic type parameters
- Import correct types from Effect
- Use proper Effect constructors and combinators
- Adjust function signatures to match usage

Note: This is general TypeScript guidance. Occasional assertions may be justified when interfacing with poorly-typed
external libraries, but document the reason.

## AVOID: Global Error in the Effect Error Channel

Do not model expected failures as `Error` in `Effect.Effect<A, Error, R>`. It erases domain information and weakens
`catchTag`, `Match`, API error mapping, and serialization.

```typescript
// Avoid
Effect.fail(new Error("User not found"))

// Prefer for domain/API errors
class UserNotFound extends Schema.TaggedError<UserNotFound>()(
  "UserNotFound",
  { userId: UserId }
) {}

Effect.fail(new UserNotFound({ userId }))
```

Use `Data.TaggedError` for internal errors that do not need Schema decoding, encoding, annotations, or HTTP/OpenAPI
integration.

## AVOID: `catchAllCause` for Error Mapping

`Cause` includes both expected failures and defects. Mapping it into a normal error hides bugs that should stay defects.

```typescript
// Avoid: catches defects too
effect.pipe(
  Effect.catchAllCause((cause) =>
    Effect.fail(new RepositoryError({ cause }))
  )
)

// Prefer: transform only expected errors
effect.pipe(
  Effect.mapError((error) =>
    new RepositoryError({ cause: error })
  )
)
```

Reach for `catchAllCause` only when you intentionally need full cause inspection at a runtime/reporting boundary.

## AVOID: Silent Error Swallowing

If a side effect matters, let its failure remain visible in the error channel. Audit logging, billing, persistence,
security checks, and notification guarantees should not quietly become `Effect.void`.

```typescript
// Avoid for important side effects
yield* audit.log(entry).pipe(
  Effect.catchTag("AuditLogError", () => Effect.void)
)

// Prefer: propagate or map the error
yield* audit.log(entry).pipe(
  Effect.mapError((error) => new CreateUserError({ cause: error }))
)
```

Fallback values are fine for optional queries; swallowing side-effect failures is not.

## AVOID: Effect Wrappers Around Safe Pure Code

`Effect.try` and `Effect.tryPromise` are boundary constructors. Do not wrap ordinary pure transformations just to make
them "Effect-shaped".

```typescript
// Avoid
const names = Effect.try(() => users.map((user) => user.name))

// Prefer
const names = users.map((user) => user.name)
```

Use `Effect.sync` for synchronous effects with observable side effects, and `Effect.try` only for code that can throw.

## RECOMMENDED: return `yield*` for Errors

**Use `return yield*` when yielding errors or interrupts in Effect.gen for clarity.**

The runtime halts on failed yields regardless of `return`, but the explicit `return` makes termination obvious and
prevents unreachable-code warnings.

**Recommended:**

```typescript
Effect.gen(function* () {
  if (someCondition) {
    return yield* Effect.fail("error message");
  }

  if (shouldInterrupt) {
    return yield* Effect.interrupt;
  }

  const result = yield* someOtherEffect;
  return result;
});
```

**Acceptable but less clear:**

```typescript
Effect.gen(function* () {
  if (someCondition) {
    yield* Effect.fail("error message");
    // Runtime halts here, but looks like code might continue
  }
});
```

The `return` keyword makes termination explicit and improves code readability.

## Null vs Option<T> Rule

**Use `Option<T>` internally, `T | null` at boundaries.**

- Internal Effect computations → `Option<T>`
- React state/props → `T | null`
- JSON serialization → `T | null` or `T | undefined`
- External API responses → normalize to `Option<T>` at boundary

See `option-null.md` for comprehensive patterns.
