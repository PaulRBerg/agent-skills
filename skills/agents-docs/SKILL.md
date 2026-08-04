---
compatibility: Requires network access and a URL-capable web fetch or curl.
coordination: exempt
disable-model-invocation: false
name: agents-docs
user-invocable: true
description: >-
  Use for current official documentation about Codex, Codex CLI, or Claude Code behavior, configuration, prompting,
  skills, permissions, tools, surfaces, capabilities, or troubleshooting; fetch the relevant official URL before
  answering.
---

# Agents Docs

This skill is coordination-exempt: skip the ai-coord gate (`git status` / `ai-coord status` / `ai-coord start`) for this
skill's own work.

Answer Codex and Claude Code product questions from the narrowest relevant live official documentation.

## Classify the request

Classify the request as Codex, Claude Code, or a comparison. Activate only for questions about those products and their
documented behavior. Do not capture unrelated repository mentions, generic agent development, provider APIs, SDKs,
pricing, authentication, model selection, migration, or managed-agent workflows.

## Route official sources

- For Codex, use only `https://developers.openai.com`. Start broad product research at
  `https://developers.openai.com/codex/codex-manual.md`.
- For Claude Code, use only `https://code.claude.com`. Start broad product research at
  `https://code.claude.com/docs/en/overview.md`.
- For a specific topic, run a compact domain-restricted web search for the exact official page, then fetch its Markdown
  representation when available. Retrieve only the page or section needed for the answer; never fetch a complete
  documentation inventory as a discovery shortcut.
- For broad Codex synthesis, fetch the consolidated manual to an operating-system temporary location, search it with
  `rg`, and read only the matching heading range into context. Do not write it or a cache into the repository or skill
  installation.
- Prefer the current client's URL-capable fetch. If it cannot fetch a known URL, use `curl -fsSL`.
- For comparisons, complete both provider routes independently. Never infer one product's behavior from the other's
  documentation.

Follow redirects only within the matching official domain and cite the final page URL.

## Answer from evidence

- Treat live official documentation as authoritative for published product claims. Do not answer current or unstable
  product facts from memory when the relevant page can be fetched.
- Cite the exact official page supporting each material claim, not a broad entry point when a topic-specific page was
  used. Prefer short paraphrases; quote only when exact wording matters.
- If verified local commands, versions, configuration, or callable capabilities conflict with the latest docs, report
  the discrepancy and prefer the observed behavior for that installed environment.
- If an exact term is absent, search obvious adjacent official concepts and state that the term itself is not
  documented.

## Bound failures

If retrieval fails, try the direct URL and one domain-restricted search, then stop expanding sources. State the bounded
uncertainty. Use local `--help`, `--version`, configuration, or observed behavior only when relevant and label it as
local evidence. Never silently substitute third-party sources, another provider's docs, or bundled knowledge.

For comparisons, answer a supported side even if the other provider is unavailable; mark the unsupported side unknown
instead of manufacturing symmetry. In network-disabled sessions, fail transparently without creating repository files or
persistent caches.

## Completion

Finish when the answer uses the narrowest relevant official source, includes precise citations, and exposes any
documentation/local-version discrepancy or retrieval limitation.
