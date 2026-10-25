# Agent Skills

PRB's collection of AI agent skills and the CLIs they rely on. Designed to work across agents, but primarily built for
[Claude Code](https://claude.com/product/claude-code) and [Codex](https://github.com/openai/codex).

> [!WARNING] This catalog intentionally reflects Paul's preferred tools, defaults, safety boundaries, and writing voice;
> it is not a neutral template. If you install it, review every workflow and customize it for your stack and agents. No
> warranties, guarantees, or support are provided — use at your own risk.

## Installation

```sh
bunx skills add PaulRBerg/agent-skills
```

Install the CLIs on macOS or Linux with Rust nightly:

```sh
cargo +nightly install --locked --git https://github.com/PaulRBerg/agent-skills ai-commit ai-coord ai-handoff ai-skillet
```

On macOS, also install desktop notifications:

```sh
cargo +nightly install --locked --git https://github.com/PaulRBerg/agent-skills ai-notify
```

From a clone, `just toolkit::install-cli` installs the supported CLIs into `~/.local/bin`. See
[toolkit installation](toolkit/README.md#installation) for prerequisites and Linux clipboard setup.

## Skills

| Skill                | Category      | Description                                                               |
| -------------------- | ------------- | ------------------------------------------------------------------------- |
| agents-brain         | Agent tooling | Maintain repo context and create AGENTS.md files or repo-local skills     |
| agents-docs          | Agent tooling | Fetch current official Codex and Claude Code docs, including hooks/trust  |
| agents-introspection | Agent tooling | Assess recurrence risk from Codex/Claude Code transcript history          |
| agents-system        | Agent tooling | Redesign repo systems for agent understanding, control, and learning      |
| copy-transcript-path | Agent tooling | Copy the active Claude Code or Codex CLI transcript path                  |
| orchestration        | Agent tooling | Research, plan, and implement work with Claude or Codex agents            |
| repo-fanout          | Agent tooling | Run one task or skill across many repositories with per-repo agents       |
| retro                | Agent tooling | Session retrospectives with transcript-backed environment fixes           |
| skill-harmonization  | Agent tooling | Harmonize repository and user-installed skill portfolios                  |
| task-handoff         | Agent tooling | Create one single- or cross-repository Codex task handoff plan            |
| wrap-up              | Agent tooling | Wind down a session fast: collect subagent wrap-ups and hand off the rest |
| cli-cast             | Crypto        | Foundry cast CLI guidance                                                 |
| cli-coingecko        | Crypto        | CoinGecko CLI for prices and market data                                  |
| evm-atlas            | Crypto        | EVM chain, account, bridge, and DEX evidence lookups                      |
| pdf                  | Documents     | Exact local PDF reading and manipulation on macOS                         |
| spreadsheets         | Documents     | Opinionated CSV/TSV/XLSX wrangling on macOS                               |
| writing-editor       | Documents     | Surgical grammar and phrasing edits that keep writing natural and casual  |
| agent-qa             | Engineering   | agent-qa regression tests with memidex memory and self-improvement        |
| cli-just             | Engineering   | Just command runner guidance                                              |
| code-polish          | Engineering   | Simplify and/or risk-profiled review with autofix                         |
| codebase-design      | Engineering   | Deep-module vocabulary and agent-legible codebase design                  |
| effect-ts            | Engineering   | Effect 4 patterns and source-aligned guidance                             |
| fresh-eyes-sweep     | Engineering   | Whole-repository audit of code, tests, and comments with verified fixes   |
| large-file-refactor  | Engineering   | Large source-file report and semantic split plans                         |
| naming-refactor      | Engineering   | Exhaustive behavior-preserving repository naming refactor                 |
| node-deps-bumper     | Engineering   | Node.js dependency updates via taze                                       |
| repo-harmonization   | Engineering   | Align interdependent repositories and transfer practices between them     |
| vitest               | Engineering   | Vitest test writing and debugging                                         |
| frontend-design      | Frontend      | Distinctive, subject-specific frontend design                             |
| html-playground      | Frontend      | Interactive single-file HTML playgrounds and task debriefs                |
| tailwind-css         | Frontend      | Tailwind CSS v4 styling guidance                                          |
| cli-gh               | Git & GitHub  | GitHub CLI operations                                                     |
| commit               | Git & GitHub  | Semantic commit messages and branch squashes via ai-commit                |
| release-bumper       | Git & GitHub  | Release workflow with changelog, tagging                                  |
| repo-rename          | Git & GitHub  | Rename or relocate repos and preserve agent thread references             |
| yeet                 | Git & GitHub  | GitHub contribution workflows                                             |
| ai-prune             | Productivity  | Archive stale .ai files and trash stale .cache entries on macOS           |
| brainstorm-with-me   | Productivity  | Co-create novel ideas and converge on a testable concept                  |
| grill-me             | Productivity  | Relentlessly stress-test plans and designs                                |
| interview-me         | Productivity  | Clarify plans and ideas through a focused, lightweight interview          |
| social-media         | Productivity  | Prepare social images, cross-post, refresh previews, and find subreddits  |
| todo-archive         | Productivity  | Archive checked TODO.md tasks into .ai/todos                              |
| chromium-browser     | Research      | Shared Chromium browsing, DevTools automation, and Wayback research       |
| tool-finder          | Research      | Find and compare current developer tools                                  |

Some mechanical skills pin `model: sonnet` in their frontmatter so they do not spend a more capable inherited model
where deterministic tooling already carries the workflow.

## Forked Skills

This table lists confirmed whole-skill sources. See the [provenance index](.agents/skill-upstreams/README.md) for
component adaptations, inferred origins, and independent rewrites. Circadian's weekly job discovers both this table and
active per-skill provenance records from fresh `origin/main`. Each record's update boundary controls its review or
merge.

| Skill           | Source                                                                                                                                                 |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| codebase-design | [mattpocock/skills: codebase-design](https://github.com/mattpocock/skills/tree/main/skills/engineering/codebase-design)                                |
| grill-me        | [mattpocock/skills: grilling](https://github.com/mattpocock/skills/tree/main/skills/productivity/grilling)                                             |
| html-playground | [anthropics/claude-plugins-official: playground](https://github.com/anthropics/claude-plugins-official/tree/main/plugins/playground/skills/playground) |
| retro           | [mattpocock/skills: retro](https://github.com/mattpocock/skills/tree/main/skills/engineering/retro)                                                    |

## CLIs

| Tool                                                    | Description                                                               |
| ------------------------------------------------------- | ------------------------------------------------------------------------- |
| [ai-commit](toolkit/commit/)                            | Prepare and commit immutable Git snapshots safely in shared working trees |
| [ai-coord](toolkit/coord/)                              | Coordinate parallel Codex and Claude Code agents                          |
| [ai-handoff](toolkit/handoff/)                          | Create and archive agent task handoffs                                    |
| [ai-notify](toolkit/notify/)                            | Deliver desktop notifications for Claude Code and Codex CLI               |
| [ai-skillet](toolkit/skillet/)                          | Inspect and maintain agent-skill catalogs                                 |
| [Coordination dashboard](toolkit/apps/coord-dashboard/) | Local live view of ai-coord state and read-only task handoffs             |

## References

- [Skills Issues](https://github.com/vercel-labs/skills/issues)
- [Introducing Skills](https://vercel.com/changelog/introducing-skills-the-open-agent-skills-ecosystem)
- [dot-claude](https://github.com/PaulRBerg/dot-claude)
- [dot-agents](https://github.com/PaulRBerg/dot-agents)

## License

MIT
