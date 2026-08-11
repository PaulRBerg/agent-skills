---
compatibility:
  Requires curl and a writable user cache directory; network populates or refreshes fixed artifacts and fetches uncached
  topic pages.
coordination: exempt
name: agents-docs
description: >-
  Use for current official documentation about Codex, Codex CLI, or Claude Code behavior, configuration, prompting,
  skills, permissions, tools, surfaces, capabilities, troubleshooting, hooks, app-server, or hook trust; consult the
  relevant official source before answering.
---

# Agents Docs

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

## Supported Chat Hosts

Before doing any work, identify the current chat host. If it is not Claude Code or Codex CLI, stop with this error:
`This skill only works in Claude Code or Codex CLI.`

Answer Codex and Claude Code product questions from the narrowest relevant official documentation.

## Classify the request

Classify the request as Codex, Claude Code, or a comparison. Activate only for questions about those products and their
documented behavior. Do not capture unrelated repository mentions, generic agent development, provider APIs, SDKs,
pricing, authentication, model selection, migration, or managed-agent workflows.

## Route official sources

- For Codex, start only at `https://developers.openai.com`. Resolve `scripts/fetch-doc.sh` relative to this skill
  directory, run `fetch-doc.sh codex-manual`, search the returned path narrowly with `rg`, and read only the matching
  heading range into context. The fixed manual and schema endpoints may redirect to their exact
  `https://learn.chatgpt.com/docs/` counterparts.
- For Claude Code, use only `https://code.claude.com`. Start broad product research at
  `https://code.claude.com/docs/en/overview.md`.
- For Codex, accept a `cached` manual for ordinary product questions. Use `--refresh` for release/change questions,
  materially disputed freshness, schema/config discrepancies, conflicts with observed local behavior, and other
  explicitly unstable cases.
- When the Codex manual contains sufficient supporting text and identifies the final topic-specific official URL, answer
  from that evidence and cite the topic URL. When it identifies the topic page but lacks enough detail, fetch that exact
  Markdown page directly. Run one compact domain-restricted search only when the exact official page remains unknown.
  Never search merely to rediscover a URL already identified by the manual.
- For a specific Claude Code topic, run a compact domain-restricted web search for the exact official page, then fetch
  its Markdown representation when available. Retrieve only the page or section needed for the answer; never fetch a
  complete documentation inventory as a discovery shortcut.
- Prefer the current client's URL-capable fetch for narrow topic pages. Use the helper only for its two fixed Codex
  artifacts.
- For comparisons, complete both provider routes independently. Never infer one product's behavior from the other's
  documentation.

Interpret helper diagnostics precisely:

- `cached` means an integrity-valid artifact was validated no more than 24 hours ago and reused without network access;
  the validation timestamp did not advance.
- `revalidated` means an older artifact received a successful conditional `304` response and had its validation metadata
  refreshed.
- `fetched` means a successful `200` response supplied integrity-valid content that replaced the prior artifact
  atomically.
- `stale` means ordinary revalidation failed and the helper fell back to an integrity-valid artifact no more than seven
  days old. Disclose its validation timestamp and the retrieval failure. Do not describe `cached` or `stale` evidence as
  fetched live.

Follow redirects only within the matching official domain, except for the helper's exact Codex artifact redirects to
`https://learn.chatgpt.com/docs/`. Cite the final page URL reported by the helper or live fetch.

## Investigate under-documented Codex configuration

When a Codex feature or `config.toml` key is missing from the prose documentation, use this bounded procedure:

1. Record the installed client and effective feature state with `codex --version` and `codex features list`.
2. Resolve the bundled helper relative to this skill directory and run `fetch-doc.sh --refresh codex-config-schema`,
   then search only the relevant definition or path in the returned file.
3. Compare the schema with the current prose config reference. Treat schema-only fields as under-documented, not as
   behavior guaranteed by every client.
4. Validate candidate syntax against the installed client with `codex --strict-config --help`; use one-off `-c`
   overrides when needed without persisting changes. Parsing acceptance is not runtime proof.
5. Report evidence separately as official docs/schema, local observations, or inference. If sources disagree, prefer the
   installed client's observed behavior for the user's environment and disclose the discrepancy.

Do not turn this into a full feature inventory: investigate only the key or feature relevant to the request.

## Answer from evidence

- Treat official documentation returned under the bounded cache policy or retrieved live as authoritative for published
  product claims. Do not answer current or unstable product facts from memory when the relevant page can be refreshed.
- Cite the exact official page supporting each material claim, not a broad entry point when a topic-specific page was
  used. Prefer short paraphrases; quote only when exact wording matters.
- If verified local commands, versions, configuration, or callable capabilities conflict with the latest docs, report
  the discrepancy and prefer the observed behavior for that installed environment.
- If an exact term is absent, search obvious adjacent official concepts and state that the term itself is not
  documented.

## Route Codex hook, app-server, and trust questions

For Codex hooks, app-server, managed hooks, or hook-trust operations, first fetch the relevant official pages:

- `https://developers.openai.com/codex/hooks`
- `https://developers.openai.com/codex/app-server`

Then read [the version-aware hooks reference](references/codex-hooks.md). Use its local protocol and config details only
when the installed client is explicitly verified as the version named there. Keep official behavior and local
observations distinct in the answer; do not turn a local implementation detail into a product guarantee.

## Bound failures

If topic-page retrieval fails, try the direct URL and one domain-restricted search, then stop expanding sources. The
helper may return a cache entry validated within the previous seven days after an ordinary retrieval failure; when it
reports `stale`, disclose its validation timestamp and do not describe it as live. Forced refreshes, expired entries,
and integrity failures fail closed. State the bounded uncertainty. Use local `--help`, `--version`, configuration, or
observed behavior only when relevant and label it as local evidence. Never silently substitute third-party sources,
another provider's docs, or bundled knowledge.

For comparisons, answer a supported side even if the other provider is unavailable; mark the unsupported side unknown
instead of manufacturing symmetry. In network-disabled sessions, use only a helper cache entry that satisfies the
bounded fresh or stale policy; otherwise fail transparently. Never create caches in a repository or skill installation.

## Completion

Finish when the answer uses the narrowest relevant official source, includes precise citations, and exposes any
documentation/local-version discrepancy or retrieval limitation.
