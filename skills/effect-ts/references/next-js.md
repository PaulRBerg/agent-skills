# Effect 4 and Next.js

Use this reference for Next.js route handlers, server actions, and caching with Effect 4. `@prb/effect-next` 2.x
requires Effect 4 (1.x targets Effect 3). Its README has a "Migrating from 1.x" section. Inspect the installed package
README and declarations before relying on an API because the package is experimental.

## Choose the Boundary Helper

- Route handlers: build a named handler with `Next.make(tag, layer)` from `@prb/effect-next/handlers`, add middleware
  with `.middleware(MiddlewareTag)`, then expose methods with `Route.build(handler)`. Use
  `Next.makeWithRuntime(tag, runtime)` when a shared `ManagedRuntime` must own the services.
- Server actions: provide the application Layer at the action boundary, or pass `{ runtime }`, then use
  `runServerAction` or `runServerActionOrThrow` from `@prb/effect-next/action` according to the caller's error contract.
- Request data: call the Effects exported as `Headers()`, `Cookies()`, and `DraftMode()`. They are not service keys.
- Params: decode with `decodeParamsUnknown` and `decodeSearchParamsUnknown` from `@prb/effect-next/params`. For numeric
  params, use `Schema.FiniteFromString`. v4 `NumberFromString` accepts non-finite values.
- Navigation: yield the package navigation helpers (`Redirect`, `PermanentRedirect`, `NotFound`) so navigation remains
  in the Effect control flow.

## Serve an HttpApi from a Route Handler

Declare the API with `effect/http-api` and convert it to a Web handler with `effect/http`:

```ts
import { Layer } from "effect";
import { HttpRouter, HttpServer } from "effect/http";
import { HttpApiBuilder } from "effect/http-api";

// endpoints: HttpApiEndpoint.get("check", "/", { success: HealthSchema })
const ApiLive = HttpApiBuilder.layer(Api).pipe(Layer.provide(HealthLive), Layer.provide(HttpServer.layerServices));
const { handler } = HttpRouter.toWebHandler(ApiLive);
export const GET: (request: Request) => Promise<Response> = handler;
```

The router matches the full request path, so set `HttpApi.make(...).prefix("/api/...")` to the route's mount path.
`toWebHandler` builds the Layer as soon as it is called (module load) and returns `{ handler, dispose }`. If the build
fails, every request rejects with the build error.

## Pick the Cache by Lifetime

- `reactCache(effectFn)` from `@prb/effect-next/react-cache` deduplicates work within one React request and preserves
  the first caller's context and span. It rejects Effects requiring `Scope`. Wrap those Effects in `Effect.scoped` or
  move acquisition into a Layer. The root export's `reactCache(effect, runtime)` is a different Promise-returning
  helper.
- `cachedEffect` and `cachedEffectWithKey` from `@prb/effect-next/persistent-cache` implement cross-request cache-aside
  behavior with an explicit store, TTL, optional stale-while-revalidate window, Schema, and failure policy.
- Cache-control helpers build browser/CDN headers. Set visibility explicitly and keep browser, generic CDN, and Vercel
  CDN policies distinct.

Reading Headers or Cookies opts the route into dynamic rendering. Keep those reads out of layouts or components that
must remain static or CDN-cacheable.

## Middleware and Telemetry

Compose middleware through the route builder and package middleware tags/layers. Do not build a parallel handler
pipeline manually. Use the telemetry adapter Layer only when an application supplies the backend. OTLP comes from
`effect/observability`, with no separate OpenTelemetry package. Bound sampling and redact high-cardinality or sensitive
values on high-volume routes.

Use the package testing kit for its documented Exit and runtime helpers, but preserve the host project's Vitest and
`@effect/vitest` conventions.
