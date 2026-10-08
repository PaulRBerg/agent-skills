# Jev Configuration Routing

Before each new research or implementation brief launches, use this workflow to select its worker configuration. The
parent retains the worker family selected in `SKILL.md`. Jev chooses one allowed model and effort pair. User
instructions take precedence over this workflow.

## Allowed Candidates and Local Fallback

1. Read the selected adapter. Classify the brief with its existing work tiers and retain that configuration as the local
   fallback.
2. Build candidates from the adapter's supported configurations. Apply explicit user model, custom-agent, and effort
   constraints first.
3. Preserve research-only restrictions, model ceilings, host permissions, and verified model and effort support.
4. For a fixed dimension, give every candidate the same value. Never offer Jev an override of that dimension.

Use these default candidates unless explicit constraints narrow them:

| Adapter       | Research pairs                                     | Implementation pairs                                           |
| ------------- | -------------------------------------------------- | -------------------------------------------------------------- |
| Native Codex  | Luna/high, Sol/medium, Sol/high                    | Luna/high, Sol/medium, Sol/high, Sol/xhigh, Astra/xhigh        |
| Codex CLI     | Luna/high, Sol/medium                              | Luna/high, Sol/medium, Sol/high, Sol/xhigh, Astra/xhigh        |
| Native Claude | Sonnet/inherited, Opus/inherited                   | Sonnet/inherited, Opus/inherited                               |
| Claude CLI    | Sonnet/medium, Sonnet/high, Opus/medium, Opus/high | Sonnet/medium, Sonnet/high, Opus/medium, Opus/high, Opus/xhigh |

Expand Codex names to `gpt-6-luna`, `gpt-6.1-sol`, and `gpt-6-astra`. Use the Claude aliases `sonnet` and `opus`. For
Claude CLI, include only pairs verified as supported by the installed CLI and selected model. Add Opus/xhigh to research
only when support and the investigation justify it. Never infer pair support from the flag alone.

Native Claude has no Agent-call effort control. Use `inherited` as the candidate effort and never send it as a tool
argument. For a custom Claude agent, preserve its configured model and effort unless the user explicitly overrides them.
Do not create agent configuration files to expose effort. If host-controlled settings leave no choice, skip Jev.

An explicit model can expose supported efforts beyond the default table. Verify those pairs before offering them. An
explicit effort can narrow supported models. If the route cannot honor a requested value, report the adapter
incompatibility. Do not use Jev to replace an incompatible user request.

If only one configuration remains, skip the network request. Use it directly and record `selection: fixed` in the brief.
For Claude CLI fallback, preserve configured effort unless the user specified one. Keep CLI timeouts under the adapter's
local rules.

## Request

Before sending data to Vercel, review the task summary and context for external disclosure. Remove secrets, unrelated
personal or customer data, unsuitable private paths or repository names, and unrelated transcript material. Send only
the brief's outcome, complexity, authority boundary, verification needs, and relevant configuration constraints. Never
send repository contents or the full parent conversation by default.

Resolve `../scripts/select-model.py` relative to this reference. Run it with Python 3 through the host's available
shell. Use the target project's Python command convention when present. Supply a JSON object on stdin:

```json
{
  "task": "Correct a bounded parser bug and add a focused regression test.",
  "context": "Implementation. Native Codex. Two files. No external writes.",
  "candidates": [
    { "id": "luna_high", "model": "gpt-6-luna", "effort": "high", "description": "Bounded routine work." },
    {
      "id": "sol_medium",
      "model": "gpt-6.1-sol",
      "effort": "medium",
      "description": "Involved work across unfamiliar code."
    }
  ]
}
```

Read `AI_GATEWAY_API_KEY` from the environment. Do not read credential files, print the key, or write it into artifacts.
The helper makes one POST to `https://ai-gateway.vercel.sh/v1/evaluate` with model `typesafe-ai/jev`. It uses a single
choice question for the complete configuration. It does not use a chat endpoint. Use the
[Vercel Decision HTTP API](https://vercel.com/docs/ai-gateway/modalities/decision#http-api) request format. Its
[native decision response](https://vercel.com/docs/ai-gateway/models-and-providers/decision-fallbacks#use-the-http-decision-api)
includes provider confidence. It disables redirects and environment-configured proxies. It bounds the response to 64 KiB
and the complete request to eight seconds. If the host cannot enforce that deadline, the helper returns fallback without
sending the request.

## Accept or Fall Back

Accept only `status: selected` with an exact allowed candidate ID and its unchanged model and effort. The helper
requires a complete finite probability distribution, approximately summing to one, and a unique maximum matching that
ID. It also requires confidence of at least `0.6`. This initial local policy is not a calibrated accuracy guarantee. It
checks [Jev's choice confidence formula](https://docs.typesafe.ai/confidence#choice),
`(n * max(probabilities) - 1) / (n - 1)`, against the returned distribution. The parent must still reject a selection
that conflicts with the task, constraints, or verified runtime support.

On any failure, uncertainty, dubious selection, unavailable Python or shell, missing key, malformed output, or nonzero
helper exit, use the local fallback. Do not retry, ask for approval, stop the handoff, or substitute a worker family
because Jev is unavailable. The helper returns only a safe reason code on failure. Never expose raw API errors, response
bodies, or credentials.

Record `selection: jev; confidence: <value>` or `selection: fallback; reason: <safe code>` in the manifest brief. For a
parent rejection, use `parent_rejected`. Keep this record concise. Continue through the selected adapter's normal launch
mechanics. Reuse the selected configuration for same-agent continuation. Route each genuinely new brief again.
