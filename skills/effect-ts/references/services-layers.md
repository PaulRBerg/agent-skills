# Services and Layers

Use this reference when defining services, choosing Layer boundaries, or composing generator-based business logic.
Inspect neighboring services first. If the project's established service and layer style is type-safe, preserve it.

## Choose the Service Shape Deliberately

`Context.Service` replaces v3 `Context.Tag`, `Context.GenericTag`, `Effect.Tag`, and `Effect.Service`.

- Use `Context.Service<Self, Shape>()("id")` when the service interface and its implementations should remain separate.
- Use `Context.Service<Self>()("id", { make })` when a default constructor belongs with the declaration. `make` does not
  generate a layer and there is no `dependencies` option. Define
  `static readonly layer = Layer.effect(this, this.make)`. Wire dependencies with `Layer.provide`.
- Use `Context.Reference<T>("id", { defaultValue: () => value })` for a context value with a safe default, such as a
  feature flag or policy value. Built-in fiber-local settings (v3 `FiberRef`) are `References.*`.
- For request-local values such as actor, tenant, locale, or request identifier, use `Effect.provideService`. Do not
  build a Layer for data that changes per request.

```ts
class UserRepository extends Context.Service<
  UserRepository,
  { readonly findById: (id: string) => Effect.Effect<string> }
>()("app/UserRepository") {}
```

Keep stable service identifiers globally unique within the application or package. The string is the runtime identity.
Static accessor proxies are gone. In generators, prefer `yield* Service`. Reserve `Service.use`/`useSync` for
one-liners. Name the primary layer `layer` and variants descriptively (`layerTest`, `layerConfig`) unless the project
differs.

## Put Acquisition in the Layer

Choose the constructor by lifecycle:

- `Layer.succeed(Service, value)` for a ready, pure value.
- `Layer.effect(Service, effect)` for effectful construction, including scoped acquisition with finalizers
  (`Layer.scoped` was merged into it).
- `Layer.unwrap` when an Effect decides which Layer to build.

Do not hide effectful or resourceful construction inside `Layer.succeed`. Provide the completed application Layer at a
runtime boundary. Avoid scattering `Effect.provide` through domain methods unless the local architecture deliberately
encapsulates a private dependency.

Layers memoize by object identity, and v4 shares the memo map across `Effect.provide` calls on a fiber. Reuse one Layer
value to share an instance. Still compose layers before providing once. To build an isolated instance, use `Layer.fresh`
or `Effect.provide(layer, { local: true })`. A factory call already creates a distinct Layer.

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
