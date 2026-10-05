# Option and Nullable Boundaries

Use `Option<A>` for meaningful absence inside Effect domain logic. Use `A | null` or `A | undefined` only when the
external contract requires it, such as JSON, React state, browser storage, or a third-party API.

Normalize once:

- incoming nullable value: `Option.fromNullishOr` (v3 `fromNullable`), or `Option.fromNullOr`/`Option.fromUndefinedOr`
  when only one empty value is legal.
- outgoing JSON or React value: `Option.getOrNull` or `Option.getOrUndefined` at the boundary.
- optional Schema domain field: `Schema.OptionFromOptionalKey(schema)`.
- explicitly nullable encoded field: `Schema.NullOr(schema)`, or `Schema.OptionFromNullOr(schema)` to decode to
  `Option`.

`Option` is not an Effect in v4: lift it with `Effect.fromOption(option)` (fails with `Cause.NoSuchElementError`) or
pass an `onNone` callback for a domain error. Do not repeatedly wrap an `Option` with `Option.fromNullishOr`. When
separate operations each introduce meaningful absence, flatten nested options. Database repositories may return
`Option<A>` when no row is normal, then translate `Option.none` to a tagged domain error at the service boundary when
the caller requires existence.
