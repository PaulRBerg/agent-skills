# Config and Secrets

Use Effect `Config` at application boundaries so missing or invalid configuration remains typed. Keep configuration
descriptions declarative and provide alternate `ConfigProvider`s at the runtime or test boundary.

```ts
import { Config } from "effect";

const AppConfig = Config.all({
  host: Config.string("HOST").pipe(Config.withDefault("localhost")),
  port: Config.number("PORT"),
  apiKey: Config.redacted("API_KEY"),
});
```

- Use `Config.redacted` for credentials and tokens. Call `Redacted.value` only at the narrow boundary that passes the
  secret to a client; never interpolate the value into logs or errors.
- Use `Config.nested` for stable prefixes instead of repeating environment-variable names.
- Validate constrained values in the Config description so startup fails before partially constructing the application.
- For tests, provide a map-backed or custom `ConfigProvider` through `Layer.setConfigProvider`; do not mutate process
  environment globally when a provider expresses the dependency.
- Do not turn constants or request data into Config merely because they are values. Config owns deployment-time input.
