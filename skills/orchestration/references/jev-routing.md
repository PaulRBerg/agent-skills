# Jev Configuration Routing

Before new research or implementation briefs launch, use this workflow to select their worker configurations. The parent
retains the worker family selected in `SKILL.md`. Jev rates how demanding each brief is on a fixed five-level rubric.
The helper maps that rating to the allowed model and effort pairs of the selected route. User instructions take
precedence over this workflow.

## Allowed Candidates and Local Fallback

1. Read the selected adapter. Classify each brief with its existing work tiers and retain that configuration as the
   local fallback.
2. Select the route key that matches the adapter: `native-claude`, `native-codex`, `codex-cli`, or `claude-cli`. The
   helper owns each route's candidate ladder. Do not write candidate lists or descriptions for a preset route.
3. Apply explicit user model and effort constraints with the `models` and `efforts` filters. A filter lists the exact
   model or effort strings that stay allowed. Never offer Jev an override of a fixed dimension.
4. Preserve research-only restrictions, model ceilings, host permissions, and verified model and effort support. For
   Claude, drop each pair that the selected tool or CLI and model do not support. Never infer pair support from the flag
   alone.

| Route                          | Research candidates                                        | Implementation candidates                                                |
| ------------------------------ | ---------------------------------------------------------- | ------------------------------------------------------------------------ |
| `native-codex`                 | `luna_high`, `sol_medium`, `sol_high`                      | `luna_high`, `sol_medium`, `sol_high`, `sol_xhigh`, `astra_xhigh`        |
| `codex-cli`                    | `luna_high`, `sol_medium`                                  | `luna_high`, `sol_medium`, `sol_high`, `sol_xhigh`, `astra_xhigh`        |
| `native-claude` / `claude-cli` | `sonnet_medium`, `sonnet_high`, `opus_medium`, `opus_high` | `sonnet_medium`, `sonnet_high`, `opus_medium`, `opus_high`, `opus_xhigh` |

The helper returns exact model strings: `gpt-6-luna`, `gpt-6.1-sol`, and `gpt-6-astra` for Codex, and the aliases
`sonnet` and `opus` for Claude. Each candidate serves one demand level. Levels run from 0 (routine) to 4 (hardest), in
the table order. When filters narrow a ladder, each demand level goes to the nearest remaining candidate. A tie goes to
the stronger candidate.

For Native Claude, determine per-call effort support as [the native adapter](native-claude.md#configuration) describes.
If the tool lacks effort control, set `inherit_effort: true`. The helper then offers one candidate per model with effort
`inherited`. Forked subagents inherit the parent's model and effort, so do not route them through Jev. For a custom
Claude agent, preserve its configured model and effort unless the user explicitly overrides them. If host-controlled
settings leave no choice, skip Jev.

An explicit model can expose supported efforts beyond the presets. An example is a user-named model that no preset
contains. Verify those pairs. Then replace the preset with a `candidates` list of `{id, model, effort, level}` objects.
Give each candidate the demand level from 0 to 4 that it serves best. To add Opus/xhigh to a research ladder, verify its
support and need, then pass that ladder as `candidates`. An explicit effort can narrow supported models. If the route
cannot honor a requested value, report the adapter incompatibility. Do not use Jev to replace an incompatible user
request.

If only one configuration remains, the helper returns `status: fixed` without a network request. You may also skip the
helper in that case. Use the configuration directly and record `selection: fixed` in the brief. For either Claude
route's fallback, preserve configured effort unless the user specified one. Keep CLI timeouts under the adapter's local
rules.

## Write the Brief

Jev rates only the text that it receives. Describe the work, not the routing:

- `task`: the outcome and the work in one to three sentences.
- `context`: the facts that make the work easy or hard. Give the number of files or sources, and say whether an existing
  pattern applies. Name the unfamiliar or ambiguous parts. Name what a mistake costs, such as concurrency, security,
  money, data loss, or an action that cannot be undone.

The helper adds `Research.` or `Implementation.` to the context. Do not repeat the kind. Omit the worker family,
adapter, tool names, commit and coordination rules, and authority boilerplate. These facts are the same for every brief
and do not help Jev rate the work. Never name an expected tier, a model, or an instruction to Jev.

In many abstentions, a brief combined a small scope with a high cost of error. State both facts explicitly so that Jev
can weigh them.

Before sending data to Vercel, review the task and context for external disclosure. Remove secrets, unrelated personal
or customer data, unsuitable private paths or repository names, and unrelated transcript material. Never send repository
contents or the full parent conversation by default.

## Request

Route every brief of a wave in one call. Resolve `../scripts/select-model.py` relative to this reference. Run it with
Python 3 through the host's available shell. Use the target project's Python command convention when present. Supply one
JSON object on stdin through a single heredoc. Do not write wrapper scripts, loops, or candidate files.

```sh
python3 <orchestration-skill-dir>/scripts/select-model.py <<'JSON'
{
  "route": "native-codex",
  "kind": "implementation",
  "briefs": [
    {
      "id": "A1",
      "task": "Correct a bounded parser bug and add a focused regression test.",
      "context": "Two files. The existing tests show the expected pattern."
    },
    {
      "id": "R1",
      "kind": "research",
      "task": "Survey how five packages configure their linters and report the differences with file evidence.",
      "context": "About fifteen config files. No unfamiliar tooling."
    }
  ]
}
JSON
```

Top-level `kind` is the default for briefs without their own `kind`. Optional top-level fields are `models`, `efforts`,
`inherit_effort`, and `candidates`. A call accepts at most twelve briefs with unique IDs. Use the manifest IDs.

Read `AI_GATEWAY_API_KEY` from the environment. Do not read credential files, print the key, or write it into artifacts.
The helper makes one POST per brief to `https://ai-gateway.vercel.sh/v1/evaluate` with model `typesafe-ai/jev`. The
requests run concurrently. Each request asks one [Score question](https://docs.typesafe.ai/api) named `demand`, whose
ordered levels are the rubric levels. It does not use a chat endpoint. Use the
[Vercel Decision HTTP API](https://vercel.com/docs/ai-gateway/modalities/decision#http-api) request format. The helper
disables redirects and environment-configured proxies. It bounds each response to 64 KiB. All requests share one
eight-second deadline. A brief that has no answer at the deadline returns `timeout`.

## Read the Result

The helper prints one JSON line per brief, in input order. Each line has the brief `id`, a `status`, and a `record`.

- `selected` or `fixed`: launch with the returned `model` and `effort`. Copy `record` into the manifest brief.
- `fallback`: use the local fallback configuration. Copy `record` into the manifest brief.

A malformed request returns one line without an `id`, with `reason: invalid_input` and a `detail` code that names the
faulty part: `payload`, `briefs`, `brief`, `route`, `kind`, `candidates`, `filters`, or `no_candidates`. Correct that
part and call the helper one more time. If the second call also fails, use the local fallback.

## Accept or Fall Back

The helper accepts only a complete, finite probability distribution over the five demand levels that approximately sums
to one. It recomputes [Jev's Score confidence formula](https://docs.typesafe.ai/confidence#score) and checks the
returned confidence against it. This formula discounts probability on distant levels more than probability on
neighbouring levels.

Observed live responses and [Jev's published examples](https://docs.typesafe.ai/api) are consistent with independent
rounding to hundredths. The local tolerance permits this rounding without assuming guaranteed wire precision. Under this
rounding, each probability can differ by `0.005` from its unrounded value. The helper permits a distribution sum error
of `5 * 0.005`. For the confidence check, it scales the probability rounding by the total level distance over the
uniform spread and adds `0.005` for confidence rounding. The helper does not normalize probabilities or raise
confidence.

Next, the helper folds the distribution onto the allowed ladder and recomputes the Score confidence on that ladder. The
reported `confidence` is this folded value. A tie for the most probable candidate returns `ambiguous_choice`. The helper
selects the most probable candidate under either rule:

- `rule: confidence`. The folded confidence reaches `0.6`.
- `rule: adjacent_tiers`. The runner-up candidate is a neighbour of the most probable one, no distant candidate ties
  with the runner-up, and the two candidates hold at least `0.8` of the probability together. A split between
  neighbouring tiers means either tier would do, so the helper takes the more probable one. With two candidates, every
  split is adjacent.

An `adjacent_tiers` result also reports `top_two_mass`. On 207 recorded briefs, the earlier ladder of model-named
candidates accepted 84 percent, with a median Jev confidence of 0.63. The demand rubric accepted 92 percent, with a
median of 0.70. It moved about a quarter of the briefs one level up, and half of those moves left the cheapest pair. On
20 graded briefs, the rubric placed every brief within one level of its intended level. These rates are a local
measurement, not a calibrated accuracy guarantee. A `low_confidence` result is an expected abstention, not an API
malfunction. The parent must still reject a selection that conflicts with the task, constraints, or verified runtime
support.

On any failure, uncertainty, dubious selection, unavailable Python or shell, missing key, malformed output, or nonzero
helper exit, use the local fallback. Except for the single corrected call after `invalid_input`, do not retry. Do not
ask for approval, stop the handoff, or substitute a worker family because Jev is unavailable.

The helper returns safe reason codes only. An `invalid_response` result can include a fixed validation `detail` code. A
`low_confidence` result includes the folded `confidence`. Never expose raw API errors, response bodies, or credentials.

For a parent rejection, record `selection: fallback; reason: parent_rejected`. Keep each record concise. Do not explain
an abstention in user-facing prose. The local fallback is the normal path, not an incident.

Continue through the selected adapter's normal launch mechanics. Reuse the selected configuration for same-agent
continuation. Route each genuinely new brief again, together with the other new briefs of its wave.
