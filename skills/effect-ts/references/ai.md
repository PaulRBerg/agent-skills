# Effect AI

Core AI modules ship inside `effect` under `effect/ai` (`LanguageModel`, `Tool`, `Toolkit`, `Prompt`, `Chat`,
`Response`, `AiError`, `EmbeddingModel`, `Model`); every module is `@stability unstable`. Providers live in
`@effect/ai-*` packages (`@effect/ai-openai`, `@effect/ai-anthropic`) at the same version as `effect`. Their `Generated`
API schemas are also unstable: depend on the provider's `*LanguageModel` and `*Client` modules, not generated wire
types. Check the installed declarations for exact request and response options.

```ts
import { LanguageModel, Tool, Toolkit } from "effect/ai";
import { OpenAiClient, OpenAiLanguageModel } from "@effect/ai-openai";
```

## Tools

Omit `parameters` for a no-argument tool; it defaults to `Tool.EmptyParams`. Pass `Tool.EmptyParams` only when the
closed empty-object contract must be explicit. It is a record whose values are `Schema.Never`; do not replace it with a
loose record.

Use `Tool.Parameters<T>`, `Tool.ParametersEncoded<T>`, `Tool.ParametersSchema<T>`, `Tool.Success<T>`, and
`Tool.Failure<T>` rather than reconstructing a tool's types. Use `setParameters` to derive a tool with another parameter
schema. `failureMode: "error"` (default) fails the calling Effect with the handler error; `"return"` captures failures
in the tool call result.

Group tools with `Toolkit.make(...)`, provide handlers with `toolkit.toLayer({ ... })`, and pass the toolkit to
`LanguageModel.generateText`, `generateObject`, or `streamText`. AI failures are one `AiError` whose `reason` is tagged
(`RateLimitError`, `InvalidOutputError`, `ToolParameterValidationError`, ...); handle one reason with
`Effect.catchReason("AiError", "<ReasonTag>", ...)`.

## OpenAI Structured Output

`OpenAiLanguageModel` config `strictJsonSchema` defaults to `true` for tools and response formats. Set
`strictJsonSchema: false` only when the selected model or a required schema construct cannot satisfy strict
structured-output requirements; disable one tool instead with `.annotate(Tool.Strict, false)`. The provider strips this
option before sending the request; do not forward it as a top-level request field.

Supply request defaults through `OpenAiLanguageModel.layer({ model, config })` and scope overrides with
`OpenAiLanguageModel.withConfigOverride`. Before adding a provider workaround, inspect the installed provider source and
changelog so application code does not duplicate a fixed package concern.
