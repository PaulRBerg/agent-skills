# Effect AI

Use installed `@effect/ai` and provider package source for exact model, request, and response configuration. Keep tool
contracts Schema-driven so parameters and structured output are validated at runtime.

## Tool Parameters

Omit `parameters` for a no-argument tool or use `Tool.EmptyParams` when the closed empty-object contract must be
explicit. `Tool.EmptyParams` is a record whose values are `Schema.Never`; do not replace it with a loose record.

Use `Tool.Parameters<T>`, `Tool.ParametersEncoded<T>`, and `Tool.ParametersSchema<T>` rather than reconstructing a
tool's types manually. Use `setParameters` when deriving a tool with another parameter schema.

## OpenAI Structured Output

The OpenAI language-model configuration supports `strict?: boolean` and enables strict schema handling by default. Set
`strict: false` only when the selected model or a required schema construct cannot satisfy strict structured-output
requirements. The provider consumes this option while preparing tools; do not forward it as an unrelated top-level
request field.

Use `"in_memory"` for prompt-cache retention. Before adding a provider workaround for request or response behavior,
inspect the installed provider source and changelog so application code does not duplicate a fixed package concern.
