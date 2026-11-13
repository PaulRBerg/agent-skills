# Research Sources

Use sources that answer the workload's questions. Record the retrieval date and the exact model version for each claim.
A source can measure adoption without measuring quality.

## Creator and Provider Sources

Start with creator documentation, model cards, release notes, and license terms. Check provider documentation for the
exact deployment ID, availability, supported parameters, context limits, regions, retention, and retirement dates. Use
current pricing pages or public catalogs. Note whether a price covers reasoning, cached input, batch requests, or media.

For local inference, read runtime support and hardware requirements for the exact weights and quantization. Record the
weight revision and serving runtime. Separate creator claims from independently measured results.

## Quality and Adoption

| Source                                                                   | Evidence                                                        | Limits                                                                                  |
| ------------------------------------------------------------------------ | --------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| [Artificial Analysis](https://artificialanalysis.ai/leaderboards/models) | Benchmark categories, methodology, speed, and price comparisons | Match the model version and effort settings. Overall indices can hide task differences. |
| [Arena](https://arena.ai/leaderboard)                                    | Human preference by modality and category                       | Preference does not establish structured-output correctness or domain accuracy.         |
| [LiveBench](https://livebench.ai/)                                       | Periodically refreshed task categories                          | Check the benchmark date, model version, and task match.                                |
| [Scale](https://labs.scale.com/leaderboard)                              | Domain evaluations                                              | Read the methodology and evaluated version.                                             |
| [OpenRouter rankings](https://openrouter.ai/rankings)                    | Usage across routed applications                                | Usage measures adoption, not task quality.                                              |
| [Hugging Face Spaces](https://huggingface.co/spaces?search=leaderboard)  | Community task-specific leaderboards                            | Check maintenance, datasets, methodology, and model revisions before trusting a Space.  |

Use public APIs and plain page retrieval first. When a page needs rendering, load `chromium-browser`. Extract the needed
values instead of taking large page snapshots. Do not assume that a retrieval method or page layout remains stable.

## Hugging Face

Use [model search](https://huggingface.co/models) to discover open weights. Sort by trending or downloads only for
discovery. Use the [Hub API documentation](https://huggingface.co/docs/hub/api) for current query fields and pagination.
Useful public requests include:

```text
GET https://huggingface.co/api/models?search=<query>&limit=20
GET https://huggingface.co/api/models/<org>/<model>
GET https://huggingface.co/api/models/<org>/<model>/revision/<revision>
GET https://huggingface.co/<org>/<model>/resolve/<revision>/README.md
```

The metadata response can include `downloads`, `likes`, `sha`, `pipeline_tag`, `tags`, `cardData`, and file siblings.
Read the model card for supported languages, modalities, evaluation details, base-model lineage, and restrictions. Read
the actual license when its terms affect use. A license tag alone does not resolve every restriction.

Record an immutable revision when reproducibility matters. Treat community quantizations as separate artifacts. Check
their source weights, quantization method, runtime compatibility, memory use, and quality evidence. Do not transfer a
full-precision score to a quantized artifact without a caveat.

[Download statistics](https://huggingface.co/docs/hub/models-download-stats) count qualifying file requests. They do not
count unique users or prove production adoption. Likes and trending ranks also do not establish quality. Use
task-matched Spaces when available. The
[original Open LLM Leaderboard retired](https://huggingface.co/spaces/open-llm-leaderboard/open_llm_leaderboard/discussions/1135).
Verify that a replacement remains maintained.

## Conditional Gateway Research

Use this section only when Vercel AI Gateway fits the caller's deployment requirements.

| Source                                                                        | Evidence                                                                   | Retrieval                                                                       |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| [Public catalog](https://ai-gateway.vercel.sh/v1/models)                      | IDs, type, price, context, modalities, tags, regions, and retention claims | `scripts/gateway-models.ts` relative to this skill root.                        |
| `https://ai-gateway.vercel.sh/v1/models/<creator>/<model>/endpoints`          | Per-provider price, uptime, latency, and throughput                        | Run the helper with `--endpoints <id>`. Missing or null metrics remain unknown. |
| [Gateway model browser](https://vercel.com/ai-gateway/models)                 | Catalog details and deployment links                                       | Read pages when the catalog needs clarification.                                |
| [Gateway leaderboard docs](https://vercel.com/docs/ai-gateway/leaderboards)   | Request, token, or spend share by model or creator                         | Use the current documented interface or an already-installed Vercel CLI.        |
| [Decision model docs](https://vercel.com/docs/ai-gateway/modalities/decision) | Decision capabilities and current request contracts                        | Read before assessing a decision-plus-language architecture.                    |

If the Vercel CLI is already available, inspect its help before using leaderboard commands. Common forms are
`vercel ai-gateway leaderboard models --modality text --format csv` and
`vercel ai-gateway leaderboard labs --format csv`. Do not install the CLI for this workflow. Use documented public pages
when the CLI is unavailable. Filter CSV rows by metric and date before comparing shares. `Other` aggregates the long
tail.

Catalog `zdr` and `no_training` values can be `all`, `some`, or `none`. A missing region does not establish regional
routing. A `-batch` deployment may have a different latency contract. Check provider details before applying a hard
retention or region gate.

Decision models use catalog type `evaluation`. They answer typed decisions rather than generating free-form text. Check
their supported modalities, context, pricing, and current API contract in the linked docs. Include fallbacks and both
stages in cost estimates. Do not assume compatibility with ordinary chat endpoints.
