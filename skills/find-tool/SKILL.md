---
disable-model-invocation: false
name: find-tool
user-invocable: true
description: Use to find, compare, and recommend current tools, packages, libraries, CLIs, VSCode extensions, agent skills, databases, or infrastructure options for a development task.
---

# Find Tool

Find and evaluate current tools, packages, and libraries across development ecosystems using fresh research.

## Workflow

1. Identify the target ecosystem from the user's request or repository context. Default to JavaScript/TypeScript npm packages and Node.js tooling when no ecosystem is specified and repo context is unavailable.
2. Ask only when the task or ecosystem is ambiguous enough to change the recommendation.
3. Search current sources; never rely only on training data because package and tool ecosystems change quickly.
4. Compare the top 3-5 viable options using ecosystem-appropriate metrics.
5. Recommend one option, explain the tradeoffs, and call out red flags.

**Read [references/find-tool.md](references/find-tool.md) before comparing or recommending anything** — it is the single source of truth for ecosystem-specific search queries, source priorities, evaluation criteria and weights, tie-breakers, output format, comparison tables, install commands, red flags, and examples.

## Research Requirements

- Use web search or other current source lookups for every recommendation.
- Prefer primary or authoritative sources for facts: official registries, package pages, docs, GitHub repositories, changelogs, security advisories, and marketplace listings.
- Capture concrete evidence: GitHub URL, stars or adoption signal, release/update recency, install/download metric when available, license or security concerns when relevant, and ecosystem-specific fit.
- Check whether the standard library or an already-available platform primitive solves the task before adding a dependency.
- When researching agent skills, combine generic web search, skills.sh, agentskills.io, and GitHub searches for `SKILL.md`; registries miss skills published only in repositories or posts. Provide source/adoption guidance instead of assuming the user's host-agent install command.
