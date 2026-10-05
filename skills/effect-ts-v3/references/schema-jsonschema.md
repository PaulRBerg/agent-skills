# Schema and JSON Schema

Use Schema to decode untrusted input once at an IO boundary, then pass validated domain values internally. Prefer the
Effect-returning decoder inside Effect code so parse failures stay typed.

```ts
import { Schema } from "effect";

const UserId = Schema.NonEmptyTrimmedString.pipe(Schema.brand("UserId"));

class User extends Schema.Class<User>("app/User")({
  id: UserId,
  email: Schema.NonEmptyTrimmedString,
}) {}

const decodeUser = Schema.decodeUnknown(User);
```

## Model the Domain Precisely

- Prefer `Schema.Class` for named entities and API models that need construction, encoding, annotations, or structural
  equality.
- Brand identifiers and constrained primitives instead of weakening them to `Schema.String` or `Schema.Number`.
- Use `Schema.TaggedError` for errors that cross encoded boundaries; use `Data.TaggedError` for internal-only errors.
- Reuse decoders and encoders at module scope rather than rebuilding them for each request.

## Encode Absence Intentionally

- `Schema.optionalWith(schema, { as: "Option" })` is appropriate when absence belongs to the decoded domain model.
- `Schema.NullOr(schema)` is appropriate when the encoded contract uses `null`.
- Exact optional fields reject unexpected keys when the boundary requires a closed shape.
- Avoid `*FromSelf` schemas for JSON contracts unless the input is intentionally already decoded.

See [option-null.md](option-null.md) for the project boundary rule.

## JSON Schema Consumers

For a closed no-parameter object, use `Schema.Record({ key: Schema.String, value: Schema.Never })` or a library's named
equivalent such as `Tool.EmptyParams`. Do not replace it with a loose record.

When generating JSON Schema directly, verify `JSONSchema.fromAST` options against the installed source. If generation
fails, inspect unsupported AST nodes and missing annotations before weakening the domain schema.
