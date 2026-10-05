# Config and Secrets

Use Effect `Config` at application boundaries so missing or invalid configuration remains typed. Keep configuration
descriptions declarative and provide alternate `ConfigProvider`s at the runtime or test boundary. `Config` values are
yieldable in `Effect.gen`.

```ts
import { Config } from "effect";

const AppConfig = Config.all({
  host: Config.String("HOST").pipe(Config.withDefault("localhost")),
  port: Config.Port("PORT"),
  apiKey: Config.Redacted("API_KEY"),
});
```

- Constructors are PascalCase in v4 (`Config.String`, `Config.Number`, `Config.Int`, `Config.Port`, `Config.Redacted`);
  v3 `Config.string`/`Config.number`/`Config.redacted` no longer exist.
- Use `Config.Redacted` for credentials and tokens. Call `Redacted.value` only at the narrow boundary that passes the
  secret to a client; never interpolate the value into logs or errors.
- Use `Config.nested` for stable prefixes instead of repeating names; with `ConfigProvider.fromEnv` the prefix joins
  with `_`.
- Validate constrained or structured values with `Config.schema(schema, path)` so startup fails before partially
  constructing the application. `Config.withDefault` applies only when the value is absent, not when it is invalid;
  default individual leaves, because a default on a `Config.all` group replaces the whole group when any child is
  absent.
- For tests, provide `ConfigProvider.layer(ConfigProvider.fromUnknown({ ... }))` (v3 `Layer.setConfigProvider`); do not
  mutate process environment globally when a provider expresses the dependency.
- Do not turn constants or request data into Config merely because they are values. Config owns deployment-time input.
