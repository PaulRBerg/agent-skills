# Schema and JSON Schema

> When to read: pull this in when decoding external data, generating JSON Schema, building AI tool parameter schemas,
> or debugging closed object shapes.

## Decode at the Boundary

Keep Schema at IO boundaries. Decode unknown input once, then pass typed domain values through the rest of the program.

```typescript
import { Effect, Schema } from "effect"

const User = Schema.Struct({
  id: Schema.Number,
  name: Schema.String
})

const decodeUser = Schema.decodeUnknown(User)

const program = (input: unknown) =>
  Effect.gen(function* () {
    const user = yield* decodeUser(input)
    return user.name
  })
```

## Optional, Nullable, and Exact Fields

- Use `Schema.optionalWith(schema, { as: "Option" })` for optional domain fields that should decode to `Option<A>`.
- Use `Schema.NullOr(schema)` when the serialized form explicitly uses `null`.
- Use exact optional fields when extra keys must be rejected in generated JSON Schema.

```typescript
const ApiUser = Schema.Struct({
  id: Schema.Number,
  nickname: Schema.NullOr(Schema.String),
  displayName: Schema.optionalWith(Schema.String, { exact: true })
})
```

## Closed Empty Records

As of `effect@3.21.3`, JSON Schema generation emits `additionalProperties: false` for string-keyed records whose values
are `Schema.Never`.

Use this shape when you need an object schema that accepts no dynamic properties, especially for no-parameter AI tools:

```typescript
import { Schema } from "effect"

const NoParams = Schema.Record({
  key: Schema.String,
  value: Schema.Never
})
```

That record represents an empty object parameter shape:

- `{}` is valid.
- `{ anything: null }` is invalid.

Prefer a named `NoParams` schema or `Tool.EmptyParams` over ad hoc `{}` types when a JSON Schema consumer needs the
closed shape.

## JSON Schema Generation

For direct JSON Schema generation, inspect `~/.effect/packages/effect/src/JSONSchema.ts` and use the public
`JSONSchema.fromAST` API with the schema AST.

```typescript
import { JSONSchema, Schema } from "effect"

const jsonSchema = JSONSchema.fromAST(NoParams.ast, {
  definitions: {},
  topLevelReferenceStrategy: "skip"
})
```

If generation fails, look for unsupported schema nodes or missing JSON Schema annotations before weakening the domain
schema.
