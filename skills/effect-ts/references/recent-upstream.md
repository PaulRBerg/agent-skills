# Recent Upstream Notes

> When to read: pull this in when checking whether local guidance is stale against `~/.effect` or when a behavior changed
> recently.

Checked against `~/.effect` HEAD `05d72eab7` from 2026-06-05.

| Package             | Version | Public guidance reflected here                                     |
| ------------------- | ------- | ------------------------------------------------------------------ |
| `effect`            | 3.21.3  | Generic `Data.taggedEnum` `$match`; closed `Schema.Never` records  |
| `@effect/ai`        | 0.36.0  | `Tool.EmptyParams` and omitted tool parameters                     |
| `@effect/ai-openai` | 0.40.0  | OpenAI `strict` handling, prompt cache enum, output dedup behavior |
| `@effect/platform`  | 0.96.1  | `HttpLayerRouter.addHttpApi` applies API-level middleware          |
| `@effect/sql`       | 0.51.1  | `SqlSchema` row decoding helpers and repository boundary guidance  |
| `@effect/rpc`       | 0.75.1  | `RpcSerialization.makeMsgPack(options)` and Cloudflare msgpack fix |
| `@effect/cluster`   | 0.59.0  | Shard group routing and SQL advisory lock numbering fixes          |
| `@effect/workflow`  | 0.18.2  | Child workflow parent pointer forwarded with `discard: true`       |
| `@effect/cli`       | latest  | Completion command hyphen replacement and dark-terminal help spans |

Before updating this file, prefer package changelogs under `~/.effect/packages/*/CHANGELOG.md` over inferring from
commit titles alone.
