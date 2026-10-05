# Schema and JSON Schema

Use Schema to decode untrusted input once at an IO boundary, then pass validated domain values internally. Prefer the
Effect-returning decoder inside Effect code so decode failures stay typed as `Schema.SchemaError` (v3 `ParseError`).

```ts
import { Schema } from "effect";

const UserId = Schema.Trimmed.check(Schema.isNonEmpty()).pipe(Schema.brand("UserId"));

class User extends Schema.Class<User>("app/User")({
  id: UserId,
  email: Schema.String.check(Schema.isPattern(/^[^@\s]+@[^@\s]+$/, { message: "Invalid email" })),
}) {}

const decodeUser = Schema.decodeUnknownEffect(User);
```

## Use v4 Names

- Decoders: `decodeUnknownEffect`, `decodeUnknownSync`, `decodeUnknownExit`, `decodeUnknownResult`,
  `decodeUnknownOption`, and `decodeUnknownPromise` (v3 `decodeUnknown`/`decodeUnknownEither`). Encoders mirror them.
- Filters are checks: `schema.check(Schema.isPattern(re, { message }))`, `Schema.isMinLength`, `Schema.isInt`,
  `Schema.makeFilter(predicate)`; refinements use `Schema.refine`. Messages are plain strings.
- Variadic constructors take arrays: `Schema.Literals(["a", "b"])`, `Schema.Union([A, B])`, `Schema.Tuple([A, B])`.
  `Schema.Literal` takes exactly one value. `Schema.Record(key, value)` takes positional arguments.
- `*FromSelf` suffixes were dropped (`Schema.Option`, `Schema.Date`). `Schema.Date` now expects a `Date`; use
  `Schema.DateFromString` for ISO strings.
- Struct edits use `mapFields` with `Struct.pick`/`Struct.omit`/`Struct.assign`; `Schema.NonEmptyTrimmedString` is
  `Schema.Trimmed.check(Schema.isNonEmpty())`.

## Model the Domain Precisely

- Prefer `Schema.Class` for named entities and API models that need validated construction, encoding, annotations, or
  class methods.
- Brand identifiers and constrained primitives instead of weakening them to `Schema.String` or `Schema.Number`.
- Use `Schema.TaggedError<Self>()("Tag", fields)` for errors that cross encoded boundaries; use `Data.TaggedError` for
  internal-only errors.
- Reuse decoders and encoders at module scope rather than rebuilding them for each request.

## Encode Absence Intentionally

- `Schema.OptionFromOptionalKey(schema)` is appropriate when absence belongs to the decoded domain model.
- `Schema.NullOr(schema)` is appropriate when the encoded contract uses `null`; `Schema.OptionFromNullOr(schema)`
  decodes it to `Option`.
- `Schema.optionalKey` is an exact optional key; `Schema.optional` also accepts `undefined`.

See [option-null.md](option-null.md) for the project boundary rule.

## JSON Schema Consumers

For a closed no-parameter object, use `Schema.Record(Schema.String, Schema.Never)` or a library's named equivalent such
as `Tool.EmptyParams` from `effect/ai`. Do not replace it with a loose record.

Generate JSON Schema with `Schema.toJsonSchemaDocument(schema, options)` (draft 2020-12); the `JsonSchema` module (v3
`JSONSchema`) converts dialects. Generation defaults to `onExcessProperty: "ignore"`, which leaves objects open; pass
`"error"` for closed objects and use the same decoder option. If generation fails, inspect unsupported AST nodes and
missing annotations before weakening the domain schema.
