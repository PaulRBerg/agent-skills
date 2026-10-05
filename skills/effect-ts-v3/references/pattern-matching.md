# Pattern Matching

Use `Match` when tagged-union branching should be exhaustive or when a multi-case error handler would otherwise become a
chain of nested conditionals.

```ts
const renderError = Match.type<AppError>().pipe(
  Match.tag("ValidationError", (error) => error.message),
  Match.tag("NetworkError", () => "Connection failed"),
  Match.exhaustive,
);
```

Use `Match.value` for one local value and `Match.type` when defining a reusable matcher. Prefer `Match.exhaustive` when
every variant must be handled; use `Match.orElse` only when the fallback is a real domain case.

For a `Data.taggedEnum`, prefer its `$match` helper when generic variant payloads or recursive unions would otherwise
require assertions. Verify constructor and matcher signatures against the installed `Data` source before changing a
generic union.
