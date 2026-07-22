---
disable-model-invocation: false
name: find-tool
user-invocable: true
description:
  Use to find, compare, and recommend current tools, packages, libraries, CLIs, VSCode extensions, agent skills,
  databases, or infrastructure options for a development task.
---

# Find Tool

Recommend the current option that best fits the user's actual constraints, backed by fresh primary evidence.

## Workflow

1. Infer the ecosystem, existing stack, must-haves, switching cost, and decision criteria from the request and
   repository. Ask only when an unknown would materially change the recommendation.
2. Check whether the standard library, platform, or an already-installed tool is sufficient before adding a dependency.
3. Search current authoritative sources. Read [references/find-tool.md](references/find-tool.md) only for the relevant
   ecosystem's source routing, fallback criteria, install conventions, and red flags.
4. Compare the viable options against the user's criteria. Use adoption, maintenance, security, documentation,
   performance, operational cost, and ecosystem fit only where they affect this decision; do not force fixed weights or
   a fixed number of candidates.
5. Recommend one option when the evidence supports it. State the decisive tradeoff, material red flags, and the closest
   alternative. Say when the evidence is too close or no external tool is justified.

## Defaults

- With no repository evidence, default to JavaScript/TypeScript and Node.js tooling.
- Prefer standard-library or platform primitives when adequate.
- For JavaScript installs, prefer the repository's package manager and otherwise `ni`; for macOS CLIs, prefer an
  official or well-maintained Homebrew formula.
- Prefer registries, official docs, repositories, changelogs, and security advisories over secondary comparisons. For
  agent skills, search registries and GitHub because no single index is complete.

## Completion

The answer is complete when the recommendation is traceable to current sources, the deciding criteria are explicit,
installation guidance matches the user's environment, and uncertainties or disqualifying risks are visible. Use a table
only when repeated fields make the comparison easier to scan.

Lead with `### 🏆 Pick: <tool>` and the decisive reason. Follow with `### 📦 Install` and the exact command, a compact
criteria/evidence table only when fields repeat, `### 🥈 Closest alternative` only for a credible material runner-up,
and `### ⚠️ Caveats` only when material. Use symbols in comparison cells only with text; never manufacture scores. Keep
URL targets exact and use descriptive Markdown links for human output. Keep install commands, versions, security
advisories, and installer-risk wording exact and undecorated.
