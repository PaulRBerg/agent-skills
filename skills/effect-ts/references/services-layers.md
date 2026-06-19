# Services and Layers

> When to read: pull this in when defining or composing Effect services, choosing between `Context.Tag` / `Effect.Service` / `Context.Reference` / `Context.ReadonlyTag`, or writing generator-based effects with `Effect.gen` and `Effect.fn`.

## Services and Layers

```typescript
// Pattern 1: Context.Tag (implementation provided separately via Layer)
class MyService extends Context.Tag("MyService")<MyService, { ... }>() {}
const MyServiceLive = Layer.succeed(MyService, { ... })
Effect.provide(effect, MyServiceLive)

// Pattern 2: Effect.Service (default implementation bundled)
class UserRepo extends Effect.Service<UserRepo>()("UserRepo", {
  effect: Effect.gen(function* () {
    const db = yield* Database
    return { findAll: db.query("SELECT * FROM users") }
  }),
  dependencies: [Database.Default],  // Optional service dependencies
  accessors: true                     // Auto-generate method accessors
}) {}
Effect.provide(effect, UserRepo.Default)  // .Default layer auto-generated
// Use UserRepo.DefaultWithoutDependencies when deps provided separately

// Effect.Service with parameters (3.16.0+)
class ConfiguredApi extends Effect.Service<ConfiguredApi>()("ConfiguredApi", {
  effect: (config: { baseUrl: string }) =>
    Effect.succeed({ fetch: (path: string) => `${config.baseUrl}/${path}` })
}) {}

// Pattern 3: Context.Reference (defaultable tags - 3.11.0+)
class SpecialNumber extends Context.Reference<SpecialNumber>()(
  "SpecialNumber",
  { defaultValue: () => 2048 }
) {}
// No Layer required if default value suffices

// Pattern 4: Context.ReadonlyTag (covariant - 3.18.0+)
// Use for functions that consume services without modifying the type
function effectHandler<I, A, E, R>(service: Context.ReadonlyTag<I, Effect.Effect<A, E, R>>) {
  // Handler can use service in a covariant position
}
```

## Global Context vs Per-Request Context

Use Layers for long-lived dependencies wired at startup: config, clients, repositories, and services. Use
`Effect.provideService` for per-request values such as authenticated user, tenant, organization, locale, request id, or
authorization context.

```typescript
const handleRequest = (request: Request) =>
  program.pipe(
    Effect.provideService(CurrentUserId, extractUserId(request)),
    Effect.provideService(RequestId, extractRequestId(request))
  )
```

Avoid constructing a Layer for one request's data. Per-request values are not application services, and wrapping them in
`Layer.succeed` makes the runtime boundary harder to see.

## Layer Construction

Choose the layer constructor by lifecycle:

```typescript
Layer.succeed(Tag, value)     // Static pure value, common for tests and simple constants
Layer.effect(Tag, make)       // Effectful construction without cleanup
Layer.scoped(Tag, acquire)    // Resourceful construction with cleanup
Layer.unwrapEffect(makeLayer) // Effectfully builds a Layer
```

For live services that read dependencies, config, or allocate resources, prefer `Layer.effect` or `Layer.scoped` over
prebuilding a value and hiding acquisition in `Layer.succeed`.

## Layer Memoization

Layers are memoized by object identity. Reusing the same layer object in one composition shares one instance; creating a
new layer object creates a distinct instance.

```typescript
const Shared = Layer.effect(Client, makeClient)

const oneClient = Layer.mergeAll(Shared, Shared)

const twoClients = Layer.mergeAll(
  Layer.effect(Client, makeClient),
  Layer.effect(Client, makeClient)
)
```

Use `Layer.fresh(layer)` only when you need to escape memoization for the same layer reference, such as a module-level
constant live layer reused with different test configuration. Do not wrap factory-created test layers in `Layer.fresh`;
each factory call already returns a new layer object.

## Generator Pattern

```typescript
Effect.gen(function* () {
  const a = yield* effectA;
  const b = yield* effectB;
  if (error) {
    return yield* Effect.fail(new MyError());
  }
  return result;
});

// Effect.fn - automatic tracing and telemetry (preferred for named functions)
const fetchUser = Effect.fn("fetchUser")(function* (id: string) {
  const db = yield* Database
  return yield* db.query(id)
})
// Creates spans, captures call sites, provides better stack traces
```
