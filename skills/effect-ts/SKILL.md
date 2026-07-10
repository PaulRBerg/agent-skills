---
disable-model-invocation: false
name: effect-ts
user-invocable: true
description:
  Use for nontrivial Effect-TS work including services/layers, typed errors, Schema/JSONSchema, Config,
  runtime/concurrency, @effect/vitest, @effect/ai, @effect/sql, or @prb/effect-next.
---

# Effect-TS

Apply Effect semantics using local project patterns first, then only the task-specific reference and upstream source
needed to resolve uncertainty.

## Fast Path

Do not activate this workflow merely because a file imports `effect`. For nontrivial Effect changes:

1. Inspect neighboring services, layers, errors, schemas, runtime boundaries, and tests.
2. Read `references/critical-rules.md` before editing. In particular, Effect failures are not caught by ordinary
   `try/catch` inside `Effect.gen`; avoid assertion-driven type escapes; use explicit terminating yields for error
   branches.
3. Read only the matching reference rows below.
4. Implement the least surprising pattern consistent with local code and verify the changed Effect behavior.

## Reference Router

| Task                                       | Reference                             |
| ------------------------------------------ | ------------------------------------- |
| Services, Layers, tags, `Effect.fn`        | `references/services-layers.md`       |
| Config and secrets                         | `references/config.md`                |
| Schema, JSON Schema, encoded errors/models | `references/schema-jsonschema.md`     |
| `@effect/vitest`, clocks, fibers, retries  | `references/testing.md`               |
| resources, scheduling, refs, concurrency   | `references/runtime.md`               |
| streams and backpressure                   | `references/streams.md`               |
| pattern matching and tagged unions         | `references/pattern-matching.md`      |
| `@effect/ai`                               | `references/ai.md`                    |
| `@effect/sql`                              | `references/sql.md`                   |
| platform and RPC                           | `references/platform-rpc.md`          |
| Next.js / `@prb/effect-next`               | `references/next-js.md`               |
| Effect Atom                                | `references/effect-atom.md`           |
| collection operations                      | `references/collection-operations.md` |
| small utilities and deprecations           | `references/quick-utils.md`           |
| `Option` at nullable boundaries            | `references/option-null.md`           |
| recent upstream drift                      | `references/recent-upstream.md`       |
| constructor/combinator lookup              | `references/quick-reference.md`       |

## Upstream Evidence

Inspect `~/.effect` only when local patterns and the selected reference do not resolve an API, type/runtime, or
recent-behavior question. If the task requires it and the checkout is missing, stop rather than guessing. Compare the
project's installed package version with the relevant package source/changelog; do not assume this skill's last-known
version is authoritative.

## Boundaries

- Keep pure helpers, constants, and path manipulation pure unless an Effect boundary provides a concrete dependency,
  testability, resource-safety, or error-model benefit.
- Preserve existing domain facades and service/runtime boundaries unless the user requested redesign.
- Prefer typed failures and scoped resources at IO boundaries; choose Schema-backed errors/models only when encoding or
  boundary validation is needed.
- Do not broaden environment requirements merely to replace a small platform call.

Completion requires code consistent with local Effect architecture, selected references/upstream evidence where needed,
and the narrowest test/typecheck that exercises the changed semantics.
