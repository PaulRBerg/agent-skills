---
argument-hint: "[task or use case]"
compatibility: Requires network access. The optional Gateway helper requires Bun or Node.js 24 or newer.
coordination: exempt
disable-model-invocation: true
name: llm-finder
skill-dependencies:
  - chromium-browser
description:
  Research and compare current language models for a task or use case. Covers hosted APIs, model gateways, open weights,
  and local inference, with source evidence, deployment fit, quality uncertainty, and equal-workload cost comparisons.
  Use only when explicitly invoked by the user or loaded by an authorized parent workflow.
---

# LLM Finder

Recommend models that fit the stated task, with explicit quality, deployment, and cost tradeoffs.

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

User instructions take precedence over these defaults. This skill reads sources and reports in chat. Save a report only
when the user requests a file. Repository writes beyond this declared work must enter the applicable coordination gate.
Do not implement a recommendation, install tools, or run paid inference by default. Explicit task or session
authorization governs those actions.

This skill runs only through explicit invocation or an authorized parent workflow. A parent workflow may load these
instructions and apply narrower requirements. Do not infer a product directory or require a repository.

## Workflow

### 1. Define the workload

Read the caller's requirements and relevant supplied context before researching candidates. State material assumptions.
Ask only when missing information changes scope, deployment safety, or the ranking. Continue independent research while
that information is pending.

Record the task, modalities, required languages, output contract, context size, latency, throughput, and expected
volume. Record the current model and exact version when available. Record budget, hardware, license, region, retention,
and no-training constraints. Treat deployment restrictions as caller requirements. Do not impose a particular gateway or
hosting platform.

### 2. Find current candidates

Read [references/sources.md](references/sources.md) before gathering evidence. Start with creator documentation and
model cards. Check provider availability, pricing, capabilities, and retirement notices. Use task-matched leaderboards
to test quality claims. Use Hugging Face metadata, cards, revisions, licenses, and hardware evidence for open-weights
candidates.

Cover hosted APIs, gateways, and local inference when they fit the workload. Narrow the search when the caller restricts
deployment. Batch independent source lookups. Give brief progress updates during sustained research.

Record exact model IDs, versions or revisions, serving providers, quantizations, and retrieval dates. Distinguish a
creator's model from a provider's deployment. Confirm that a benchmark result matches the proposed version and settings.

Use public APIs and plain page retrieval first. When a required page needs rendering, use the `chromium-browser` skill.
Do not install a browser, CLI, or package automatically.

### 3. Compare evidence and fit

Apply hard requirements before ranking quality or cost. Check input and output modalities, schema and tool support,
context limits, languages, licenses, lifecycle, region, and retention. For local inference, check memory, runtime
support, quantization, and expected speed on the available hardware.

Keep demonstrated quality separate from popularity and adoption. Downloads, likes, usage share, and creator recognition
can support adoption claims. They do not establish task accuracy. Do not reject a newcomer solely for missing popularity
or an arbitrary adoption threshold. Apply an explicit caller reputation gate when one exists.

Use task-relevant benchmark categories and evaluation methods. Record score dates, settings, uncertainty, and material
method differences. Missing evidence does not establish equal quality. Label unsupported quality claims and cost gaps.
Do not fabricate scores, prices, latency, or availability.

When a discrete decision precedes language generation, assess a decision model plus a language model. Recommend the pair
only when its modalities fit and its expected cost, latency, or control benefit justifies both stages.

### 4. Compare cost on one workload

Use the same workload and volume for every option. State input, output, reasoning, image, audio, caching, batch, and
region assumptions when applicable. Include both stages of a pair and any routing frequency. For local inference,
separate hardware and hosting expense from token charges. Do not treat local inference as free by assumption.

Show cost per request and at the expected volume. Compare absolute savings or premiums and ratios against the current
model. If its price is unavailable, name another baseline. Keep observed costs separate from estimates. Use scenarios
when usage or prices remain uncertain. Do not infer total cost from input price alone.

### 5. Deliver the recommendation

Lead with the preferred option and useful alternatives. Explain when each option is preferable. State whether the
recommendation changes the current model. Do not turn alternatives into a routing implementation requirement.

Include these fields in the report:

- Workload, constraints, baseline, and material assumptions.
- Exact candidate IDs and versions, deployment paths, retrieval dates, and fit decisions.
- Task-relevant quality evidence and separate adoption evidence, with source links.
- Prices, cost assumptions, estimated costs, and relative costs on the common workload.
- Excluded candidates and reasons, including unavailable or retired deployments.
- Evidence gaps and the smallest evaluation needed to resolve a material uncertainty.

## Completion Evidence

Complete the authorized research and deliver the report in chat. If a file was requested, also save it at the requested
path. Do not stop after a shortlist or a progress report. Stop for a missing requirement only when it prevents a useful
recommendation. State the exact missing input and complete independent research first.

## Gateway Helper

Resolve `scripts/` relative to this skill directory. The helper uses runtime builtins and needs no package installation.
It reads the public catalog without authentication:

```sh
bun run <skill-root>/scripts/gateway-models.ts --input image --tag structured-output --region eu --max-input 2
node <skill-root>/scripts/gateway-models.ts --type evaluation --input text
bun run <skill-root>/scripts/gateway-models.ts --endpoints <creator/model>
```

Default: non-deprecated language models, newest first. Options: `--type <t>`, `--match <a,b>`, `--creator <a,b>`,
`--input <a,b>`, `--tag <a,b>`, `--region <r>`, `--zdr`, `--max-input <USD per 1M>`, `--max-output <USD per 1M>`,
`--include-deprecated`, `--describe`, `--endpoints <id>`, and `--json`. `--zdr` accepts models whose catalog value is
`all` or `some`. Verify the chosen provider when a requirement needs universal zero data retention.

The helper preserves unknown model fields in JSON output. Missing prices display as `-` and fail an active price
ceiling. Requests time out after 20 seconds. Exit codes: 0 for success, 2 for invalid arguments, and 1 for network or
response-validation errors. Catalog values are evidence of the provider's claims, not an independent service guarantee.
