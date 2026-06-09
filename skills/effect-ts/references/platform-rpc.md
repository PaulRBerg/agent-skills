# Platform and RPC

> When to read: pull this in when using `@effect/platform`, `HttpLayerRouter`, `@effect/rpc`, msgpack serialization, or
> deployment runtimes such as Cloudflare Workers.

## HttpLayerRouter Middleware

`@effect/platform@0.96.1` preserves fiber context in `HttpLayerRouter.addHttpApi`, so API-level middleware is applied
when an API is registered through the router.

If API-level middleware appears to be skipped, check the pinned `@effect/platform` version before adding route-local
workarounds.

## RPC MsgPack Serialization

`@effect/rpc@0.75.1` adds configurable msgpack serialization:

```typescript
import { RpcSerialization } from "@effect/rpc"

const serialization = RpcSerialization.makeMsgPack({
  useRecords: true
})
```

The default `RpcSerialization.msgPack` is equivalent to `makeMsgPack({ useRecords: true })`.

## Cloudflare Workers

The current RPC stack uses `msgpackr@1.11.10`, which falls back when dynamic code evaluation is blocked. In Cloudflare
Workers or similarly restricted runtimes, prefer upgrading `@effect/rpc` / `@effect/platform` over custom serialization
patches for silent msgpack decode failures.
