# Services and Layers

Use this reference when defining services, choosing Layer boundaries, or composing generator-based business logic.
Inspect neighboring services first; preserve the project's established tag and layer style when it is type-safe.

## Choose the Service Shape Deliberately

- Use `Context.Tag` when the service interface and its implementations should remain separate.
- Use `Effect.Service` when a default implementation and dependency Layer belong with the service declaration.
- Use `Context.Reference` for a context value with a safe default, such as a feature flag or policy value.
- Use `Effect.provideService` for request-local values such as actor, tenant, locale, or request identifier; do not
  build a Layer for data that changes per request.

```ts
class UserRepository extends Context.Tag("app/UserRepository")<
  UserRepository,
  { readonly findById: (id: string) => Effect.Effect<string, never, never> }
>() {}
```

Keep stable service identifiers globally unique within the application or package.

## Put Acquisition in the Layer

Choose the constructor by lifecycle:

- `Layer.succeed` for a ready, pure value;
- `Layer.effect` for effectful construction without cleanup;
- `Layer.scoped` for acquisition that registers finalizers;
- `Layer.unwrapEffect` when an Effect decides which Layer to build.

Do not hide effectful or resourceful construction inside `Layer.succeed`. Provide the completed application Layer at a
runtime boundary; avoid scattering `Effect.provide` through domain methods unless the local architecture deliberately
encapsulates a private dependency.

Layers memoize by object identity within a composition. Reuse one Layer value to share an instance. A factory call
already creates a distinct Layer; use `Layer.fresh` only when deliberately escaping memoization of the same Layer
object.

## Use `Effect.fn` for Reusable Effectful Functions

Prefer `Effect.fn("qualifiedName")` for reusable generator functions that benefit from named traces and better stack
information. Keep a raw `Effect.gen` for one-off program composition.

```ts
const findUser = Effect.fn("UserRepository.findUser")(function* (id: string) {
  const repository = yield* UserRepository;
  return yield* repository.findById(id);
});
```

Keep service methods domain-oriented. Avoid exporting one accessor wrapper per method when callers can yield the service
directly.
