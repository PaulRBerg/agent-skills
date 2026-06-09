# Effect AI

> When to read: pull this in when defining `@effect/ai` tools, toolkits, provider-defined tools, or OpenAI language
> model configuration.

## Tool Parameters

`@effect/ai@0.36.0` supports no-parameter tools directly. Omit `parameters` for the default empty parameter schema, or
use `Tool.EmptyParams` when the emptiness needs to be explicit.

```typescript
import { Tool } from "@effect/ai"
import { Schema } from "effect"

const GetCurrentTime = Tool.make("GetCurrentTime", {
  description: "Returns the current timestamp",
  success: Schema.Number
})

const Ping = Tool.make("Ping", {
  parameters: Tool.EmptyParams,
  success: Schema.String
})

const ReadFile = Tool.make("ReadFile").setParameters({
  filePath: Schema.String
})

const NoArgsAgain = ReadFile.setParameters(Tool.EmptyParams)
```

`Tool.EmptyParams` is `Schema.Record({ key: Schema.String, value: Schema.Never })`, so generated JSON Schema should be a
closed empty object shape. Do not replace it with a loose `Record<string, unknown>`.

## Tool Type Extraction

Use the built-in utility types when handlers need the decoded or encoded parameter shape:

```typescript
type Params = Tool.Parameters<typeof ReadFile>
type EncodedParams = Tool.ParametersEncoded<typeof ReadFile>
type ParamsSchema = Tool.ParametersSchema<typeof ReadFile>
```

## OpenAI Strict Mode

`@effect/ai-openai` exposes `strict?: boolean` on `OpenAiLanguageModel` config.

- Default is strict structured-output behavior for generated tool schemas and JSON Schema response formats.
- Set `strict: false` only when a model or schema construct cannot satisfy OpenAI strict schema requirements.
- Recent versions consume `strict` during schema preparation; it should not be sent as a top-level Responses API request
  parameter.

## OpenAI Prompt Cache Enums

Use `"in_memory"` for prompt cache retention enum values. Older examples that use `"in-memory"` are stale.

```typescript
const promptCacheRetention = "in_memory"
```

## Response Output Handling

Recent OpenAI provider versions deduplicate `response.output` items before JSON concatenation. If you see duplicated
structured output in a pinned project, check `@effect/ai-openai` before working around it at the application layer.
