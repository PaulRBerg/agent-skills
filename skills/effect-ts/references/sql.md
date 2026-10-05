# Effect SQL

Core SQL modules ship inside `effect` under `effect/sql` (`SqlClient`, `SqlSchema`, `SqlResolver`, `SqlError`,
`Statement`, `Migrator`); they are `@stability unstable`. Drivers live in `@effect/sql-*` packages (`@effect/sql-pg`,
`@effect/sql-sqlite-node`, ...) at the same version as `effect`. Variant models moved to `effect/schema/Model`. Use the
installed declarations for exact driver and helper signatures. Keep SQL at repository boundaries and return domain
values rather than unchecked row shapes.

```ts
import { SqlClient, SqlSchema } from "effect/sql";
import { PgClient } from "@effect/sql-pg";
```

## Decode Rows

Build queries with `SqlSchema.*({ Request, Result, execute })` and pick the constructor whose cardinality matches:

- `findAll` returns an array; `findNonEmpty` fails with `Cause.NoSuchElementError` on zero rows;
- `findOne` returns the first row or fails with `Cause.NoSuchElementError` (v3 `single`);
- `findOneOption` returns `Option<A>` (v3 `findOne`);
- `SqlSchema.void` encodes the request and discards the result.

A raw SQL type parameter describes a row but does not validate database output. Use precise schemas for identifiers,
literals, decimals, and encoded values. Use `findOneOption` when no row is normal; translate absence to a tagged domain
error when the service contract requires existence.

## Preserve Repository and Transaction Boundaries

`SqlError` carries a tagged `reason` (`UniqueViolation`, `ConstraintError`, `DeadlockError`, `SerializationError`,
`ConnectionError`, ...) and an `isRetryable` getter. Branch on the reason with `Effect.catchReason("SqlError", ...)`
instead of parsing driver messages. Repository services may expose domain errors while retaining driver and decode
causes for diagnostics. Map expected SQL or decode failures with `Effect.mapError`; do not map defects through
`Effect.catchCause` in ordinary repository code.

Use `sql.withTransaction` for writes that must commit atomically. Include audit, outbox, or ledger writes in the same
transaction only when the product invariant requires one commit boundary.
