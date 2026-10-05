# Effect SQL

Use the installed `@effect/sql` package source for exact driver and helper signatures. Keep SQL at repository boundaries
and return domain values rather than unchecked row shapes.

## Decode Rows

Prefer `SqlSchema.findOne`, `SqlSchema.findAll`, or `SqlSchema.single` when their cardinality matches the query. A raw
SQL type parameter describes a row but does not validate database output.

Use precise schemas for identifiers, literals, decimals, and encoded values. Keep absence as `Option<A>` when no row is
normal; translate it to a tagged domain error when the service contract requires existence.

## Preserve Repository and Transaction Boundaries

Repository services may expose domain errors while retaining driver and decode causes for diagnostics. Map expected SQL
or decode failures with `Effect.mapError`; do not map defects through `catchAllCause` in ordinary repository code.

Use the client's transaction API for writes that must commit atomically. Include audit, outbox, or ledger writes in the
same transaction only when the product invariant requires one commit boundary.
