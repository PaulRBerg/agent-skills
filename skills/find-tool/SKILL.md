---
disable-model-invocation: false
name: find-tool
user-invocable: true
description: Use to find, compare, and recommend current tools, packages, libraries, CLIs, VSCode extensions, agent skills, databases, or infrastructure options for a development task.
---

# Find Tool

Find and evaluate current tools, packages, and libraries across development ecosystems using fresh research.

## Workflow

1. Identify the target ecosystem from the user's request or repository context.
2. Ask only when the task or ecosystem is ambiguous enough to change the recommendation.
3. Search current sources; never rely only on training data because package and tool ecosystems change quickly.
4. Compare the top 3-5 viable options using ecosystem-appropriate metrics.
5. Recommend one option, explain the tradeoffs, and call out red flags.

Use [references/find-tool.md](references/find-tool.md) for ecosystem-specific search queries, source priorities, evaluation criteria, output tables, install commands, red flags, and examples.

## Ecosystem Defaults

- Default to JavaScript/TypeScript npm packages and Node.js tooling when no ecosystem is specified and repo context is unavailable.
- Route Python requests to PyPI, uv, pip, or Poetry.
- Route Rust requests to crates.io, lib.rs, cargo, and relevant curated lists.
- Route Go requests to pkg.go.dev, Go modules, and the standard library first.
- Route CLI/system tool requests to GitHub, Homebrew, package managers, and maintained command-line tool lists.
- Route VSCode extension requests to the VS Code Marketplace, Open VSX, GitHub, and extension changelogs.
- Route agent skill requests to skills.sh, agentskills.io, GitHub, and broad web search; never rely on a single registry.
- Route database and infrastructure requests to product docs, production-use comparisons, managed service docs, and operations references.

## Research Requirements

- Use web search or other current source lookups for every recommendation.
- Prefer primary or authoritative sources for facts: official registries, package pages, docs, GitHub repositories, changelogs, security advisories, and marketplace listings.
- Capture concrete evidence: GitHub URL, stars or adoption signal, release/update recency, install/download metric when available, license or security concerns when relevant, and ecosystem-specific fit.
- Check whether the standard library or an already-available platform primitive solves the task before adding a dependency.
- When researching agent skills, combine generic web search, skills.sh, agentskills.io, and GitHub searches for `SKILL.md`; registries miss skills published only in repositories or posts.

## Recommendation Rules

- Rank by adoption, maintenance, security/quality, documentation/DX, and performance, then apply ecosystem-specific tie-breakers from the reference.
- For JavaScript/TypeScript, prefer native TypeScript, ESM, tree-shaking, small bundles, Bun compatibility, and minimal dependencies.
- For Python, prefer typed, maintained packages with current Python support and low native-extension burden unless performance requires it.
- For Rust, weigh unsafe usage, compile time, binary size, and `no_std` support when relevant.
- For Go, prefer the standard library when suitable, then packages with minimal dependencies and idiomatic `context.Context` support.
- For CLIs, weigh install method, startup time, output ergonomics, platform support, and plugin ecosystem.
- For VSCode extensions, weigh installs, rating, activation cost, extension size, compatibility with Cursor, and permission surface.
- For agent skills, weigh `SKILL.md` quality, cross-agent portability, bundled script risk, token footprint, license, and freshness.

## Output

Present a concise recommendation:

1. `### Recommended: <tool-name>`
2. Installation or adoption command when appropriate.
3. Why this option wins, with concrete evidence.
4. Key stats including GitHub URL and stars whenever available.
5. Two or three alternatives with their differentiators.
6. A comparison table using the ecosystem-specific columns in the reference.
7. Red flags, caveats, or "no good package exists" if the research points that way.

For agent skills, provide source/adoption guidance instead of assuming the user's host-agent install command.

## Final Check

- Confirm current research was used.
- Confirm the ecosystem and task fit.
- Confirm multiple sources were checked.
- Confirm GitHub URL and stars were included where available.
- Confirm red flags and security/maintenance concerns were considered.
- Confirm alternatives were compared, not merely listed.
