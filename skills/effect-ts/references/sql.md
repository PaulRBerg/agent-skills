# Effect SQL

> When to read: pull this in when using `@effect/sql`, `SqlSchema`, repository services, database transactions, or row
> decoding.

## Decode Rows with Schema

Prefer `SqlSchema` or explicit Schema decoding over TypeScript type parameters on raw SQL calls. A type parameter can
describe the row shape, but it does not validate database output.

```typescript
import { SqlClient, SqlSchema } from "@effect/sql"
import { Context, Effect, Option, Schema } from "effect"

const AccountRow = Schema.Struct({
  id: AccountId,
  name: Schema.String,
  accountType: AccountType
})

const findById = SqlSchema.findOne({
  Request: AccountId,
  Result: AccountRow,
  execute: (id) =>
    Effect.gen(function* () {
      const sql = yield* SqlClient.SqlClient
      return yield* sql`
        SELECT id, name, account_type AS "accountType"
        FROM accounts
        WHERE id = ${id}
      `
    })
})
```

Use precise domain schemas for rows. If the database value should be an `AccountId`, `CurrencyCode`, `BigDecimal`, or
domain literal union, decode it as that type instead of weakening it to `Schema.String` or `Schema.Number`.

## Pick the Right `SqlSchema` Helper

```typescript
SqlSchema.findOne({ Request, Result, execute }) // Option<A>; zero or one row
SqlSchema.findAll({ Request, Result, execute }) // Array<A>; zero or more rows
SqlSchema.single({ Request, Result, execute })  // A; exactly one row
SqlSchema.void({ Request, execute })            // void; writes with no returned row
```

Use `findOne` when absence is a normal case, then convert `Option.none` to a domain error at the service boundary if the
caller requires existence.

## Repository Boundaries

Keep SQL details in repository services. Domain services should depend on repository tags and speak in domain values,
not raw rows.

```typescript
class AccountRepository extends Context.Tag("AccountRepository")<
  AccountRepository,
  {
    readonly findById: (id: AccountId) => Effect.Effect<Option.Option<Account>, RepositoryError>
    readonly save: (account: Account) => Effect.Effect<void, RepositoryError>
  }
>() {}
```

Map driver and decode errors into repository errors with `Effect.mapError`; do not use `catchAllCause` unless you are at
a deliberate reporting boundary and want to include defects.

## Transactions

Use the SQL client's transaction API around all writes that must commit atomically. Keep audit, outbox, or ledger writes
in the same transaction when the product invariant requires them to succeed or fail together.

```typescript
const createAccount = (account: Account) =>
  Effect.gen(function* () {
    const sql = yield* SqlClient.SqlClient

    yield* sql.withTransaction(
      Effect.gen(function* () {
        yield* insertAccount(account)
        yield* insertAuditEntry(account)
      })
    )
  })
```
