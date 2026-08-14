# Effect 3 and Next.js

Use this reference only for projects using `@prb/effect-next`. Inspect the installed package README and declarations
before relying on an API because the package is experimental.

## Choose the Boundary Helper

- Route handlers: build a named handler with `Next.make(name, layer)` from `@prb/effect-next/handlers`, then expose
  methods with `Route.build`.
- Server actions: provide the application Layer at the action boundary, then use `runServerAction` or
  `runServerActionOrThrow` from `@prb/effect-next/action` according to the caller's error contract.
- Request data: call the Effects exported as `Headers()`, `Cookies()`, and `DraftMode()`; they are not service tags.
- Navigation: yield the package navigation helpers so redirects, rewrites, and not-found behavior remain in the Effect
  control flow.

## Pick the Cache by Lifetime

- `reactCache(effectFn)` deduplicates work within one React request. It rejects Effects requiring `Scope`; move resource
  acquisition into a Layer.
- `cachedEffect` and `cachedEffectWithKey` implement cross-request cache-aside behavior with an explicit store, TTL,
  optional stale-while-revalidate window, Schema, and failure policy.
- Cache-control helpers build browser/CDN headers. Set visibility explicitly and keep browser, generic CDN, and Vercel
  CDN policies distinct.

Reading Headers or Cookies opts the route into dynamic rendering. Keep those reads out of layouts or components that
must remain static or CDN-cacheable.

## Middleware and Telemetry

Compose middleware through the route builder and package middleware tags/layers; do not hand-roll a parallel handler
pipeline. Use the telemetry adapter Layer only when an application supplies the backend. Bound sampling and redact
high-cardinality or sensitive values on high-volume routes.

Use the package testing kit for its documented Exit and runtime helpers, but preserve the host project's Vitest and
`@effect/vitest` conventions.
