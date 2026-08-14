---
compatibility:
  Requires a project using current stable Effect 3 packages; verify exact APIs against the target's installed package
  source.
name: effect-ts
description:
  Use for nontrivial Effect 3 work including services/layers, typed errors, Schema/JSONSchema, Config,
  runtime/concurrency, @effect/vitest, @effect/ai, @effect/sql, Effect Atom, or @prb/effect-next.
---

# Effect 3

Apply Effect 3 semantics from project-local architecture, the narrowest relevant reference, and source matching the
target's installed packages.

## Workflow

Do not activate this workflow merely because a file imports `effect`. For nontrivial Effect work:

1. Resolve the target package or workspace and its exact installed `effect` and relevant `@effect/*` versions. If
   `effect` is not 3.x, stop because this skill does not apply.
2. Inspect neighboring services, layers, errors, schemas, runtime boundaries, and tests. Local conventions decide
   organization; installed package evidence decides API facts.
3. Read `references/critical-rules.md`, then only the task-specific references below.
4. Verify every uncertain import, signature, or behavior against the package installation visible to the target
   workspace before editing.
5. Implement the smallest pattern consistent with the project and run the narrowest test or typecheck covering the
   changed semantics.

## Evidence Order

Use the target workspace's manifest and lockfile to identify versions. Prefer, in order:

1. the installed package's `src/`, README, tests, and changelog;
2. its emitted declarations when source is not shipped;
3. the matching official package artifact or source tag.

Do not install or update dependencies solely to obtain documentation. Do not trust an unrelated checkout or a source
branch that does not match the target's installed version. If exact behavior cannot be verified, stop rather than
guessing.

## Reference Router

| Task                                       | Reference                         |
| ------------------------------------------ | --------------------------------- |
| Services, Layers, tags, `Effect.fn`        | `references/services-layers.md`   |
| Config providers and secrets               | `references/config.md`            |
| Schema, JSON Schema, encoded errors/models | `references/schema-jsonschema.md` |
| `@effect/vitest`, clocks, fibers, retries  | `references/testing.md`           |
| resources, scheduling, refs, concurrency   | `references/runtime.md`           |
| streams and backpressure                   | `references/streams.md`           |
| pattern matching and tagged unions         | `references/pattern-matching.md`  |
| `@effect/ai`                               | `references/ai.md`                |
| `@effect/sql`                              | `references/sql.md`               |
| Next.js / `@prb/effect-next`               | `references/next-js.md`           |
| Effect Atom                                | `references/effect-atom.md`       |
| `Option` at nullable boundaries            | `references/option-null.md`       |

For platform/RPC APIs, collection utilities, deprecations, or constructor lookup, inspect the installed package source
directly instead of loading a local API inventory.

## Boundaries

- Keep pure helpers, constants, and path manipulation pure unless an Effect boundary provides a concrete dependency,
  testability, resource-safety, or error-model benefit.
- Preserve existing domain facades and service/runtime boundaries unless the user requested redesign.
- Prefer typed failures and scoped resources at IO boundaries; choose Schema-backed errors/models only when encoding or
  boundary validation is needed.
- Do not broaden environment requirements merely to replace a small platform call.

For changes, completion requires code consistent with local Effect architecture, selected references and installed
source where needed, and the narrowest test/typecheck that exercises the changed semantics. Read-only work requires
evidence for the reported conclusion. Finish with `### ⚡ Effect — ✅ change complete` after verified edits or
`### ⚡ Effect — 🔎 reviewed, no files written` for read-only work, one sentence naming the boundary or pattern used,
and `### 🧪 Verification` with exact scoped commands/results. If required validation is incomplete, use
`### ⚡ Effect — ⛔ blocked` instead. Add `### ⚠️ Limitation` only for non-blocking caveats. Never decorate typed
errors, Schema messages, logs, tests, generated JSON/API responses, or command output.
