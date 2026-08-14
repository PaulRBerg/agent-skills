# Option and Nullable Boundaries

Use `Option<A>` for meaningful absence inside Effect domain logic. Use `A | null` or `A | undefined` only when the
external contract requires it, such as JSON, React state, browser storage, or a third-party API.

Normalize once:

- incoming nullable value: `Option.fromNullable` at the boundary;
- outgoing JSON or React value: `Option.getOrNull` or `Option.getOrUndefined` at the boundary;
- optional Schema domain field: `Schema.optionalWith(schema, { as: "Option" })`;
- explicitly nullable encoded field: `Schema.NullOr(schema)`.

Do not repeatedly wrap an `Option` with `Option.fromNullable`; flatten nested options when separate operations each
introduce meaningful absence. Database repositories may return `Option<A>` when no row is normal, then translate
`Option.none` to a tagged domain error at the service boundary when the caller requires existence.
