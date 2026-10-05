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

Install the CLIs the skills rely on:

```sh
cargo install --locked --git https://github.com/PaulRBerg/agent-skills ai-commit ai-coord ai-handoff ai-notify ai-skillet
```

From a clone, `just toolkit::install-cli` installs them into `~/.local/bin`.

## Skills

| Skill                  | Description                                                                |
| ---------------------- | -------------------------------------------------------------------------- |
| agents-brain           | Maintain repo context and repo-local skills                                |
| agents-docs            | Fetch current official Codex and Claude Code docs, including hooks/trust   |
| agents-introspection   | Assess recurrence risk from Codex/Claude Code transcript history           |
| ai-prune               | Archive stale .ai files and trash stale .cache entries on macOS            |
| autoresearch           | Autonomous experiment loop                                                 |
| brainstorm-with-me     | Co-create novel ideas and converge on a testable concept                   |
| chromium-browser       | Shared Chromium browsing, DevTools automation, and Wayback research        |
| claude-handoff         | Research or implement approved plans with Claude subagents                 |
| cli-cast               | Foundry cast CLI guidance                                                  |
| cli-coingecko          | CoinGecko CLI for prices and market data                                   |
| cli-just               | Just command runner guidance                                               |
| code-polish            | Simplify and/or risk-profiled review with autofix                          |
| codebase-design        | Shared vocabulary and principles for designing deep modules                |
| codex-handoff          | Research or implement approved plans with Codex agents                     |
| commit                 | Semantic commit messages with deterministic ai-commit mechanics            |
| copy-transcript-path   | Copy the active Claude Code or Codex CLI transcript path                   |
| effect-ts              | Effect 4 patterns and source-aligned guidance                              |
| evm-atlas              | EVM lookup + DEX transaction/order interpretation                          |
| fresh-eyes-sweep       | Whole-repository audit of code, tests, and comments with verified fixes    |
| frontend-design        | Distinctive, subject-specific frontend design                              |
| git-squash             | Squash PR branch with semantic commit message                              |
| grill-me               | Relentlessly stress-test plans and designs                                 |
| html-playground        | Interactive single-file HTML playgrounds and task debriefs                 |
| interview-me           | Clarify plans and ideas through a focused, lightweight interview           |
| large-file-refactor    | Large source-file report and Serena split plans                            |
| naming-refactor        | Exhaustive behavior-preserving repository naming refactor                  |
| node-deps-bumper       | Node.js dependency updates via taze                                        |
| pdf                    | Exact local PDF reading and manipulation on macOS                          |
| release-bumper         | Release workflow with changelog, tagging                                   |
| repo-cross-pollination | Transfer useful guidance, workflows, and dependencies between repositories |
| repo-harmonization     | Audit and align multiple interdependent repositories                       |
| repo-rename            | Rename GitHub repo, folder, and agent thread references                    |
| retro                  | Session retrospectives with transcript-backed environment fixes            |
| skill-harmonization    | Harmonize repository and user-installed skill portfolios                   |
| skill-writing          | Bootstrap a project-local agent skill                                      |
| spreadsheets           | Opinionated CSV/TSV/XLSX wrangling on macOS                                |
| tailwind-css           | Tailwind CSS v4 styling guidance                                           |
| task-handoff           | Create one single- or cross-repository Codex task handoff plan             |
| todo-archive           | Archive checked TODO.md tasks into .ai/todos                               |
| tool-finder            | Find and compare current developer tools                                   |
| vitest                 | Vitest test writing and debugging                                          |
| wrap-up                | Wind down a session fast: collect subagent wrap-ups and hand off the rest  |
| yeet                   | GitHub contribution workflows                                              |

## Forked Skills

| Skill           | Source                                                                                                                                                 |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| html-playground | [anthropics/claude-plugins-official: playground](https://github.com/anthropics/claude-plugins-official/tree/main/plugins/playground/skills/playground) |

Some mechanical skills pin `model: sonnet` in their frontmatter so they do not spend a more capable inherited model
where deterministic tooling already carries the workflow.

## CLIs

| Tool                                                    | Description                                                               |
| ------------------------------------------------------- | ------------------------------------------------------------------------- |
| [ai-commit](toolkit/commit/)                            | Prepare and commit immutable Git snapshots safely in shared working trees |
| [ai-coord](toolkit/coord/)                              | Coordinate parallel Codex and Claude Code agents                          |
| [ai-handoff](toolkit/handoff/)                          | Create and archive agent task handoffs                                    |
| [ai-notify](toolkit/notify/)                            | Deliver desktop notifications for Claude Code and Codex CLI               |
| [ai-skillet](toolkit/skillet/)                          | Inspect and maintain agent-skill catalogs                                 |
| [Coordination dashboard](toolkit/apps/coord-dashboard/) | Local live view of ai-coord state                                         |
| [AI Handoffs](toolkit/apps/handoffs/)                   | Local, read-only task-handoff viewer                                      |

## References

- [Skills Issues](https://github.com/vercel-labs/skills/issues)
- [Introducing Skills](https://vercel.com/changelog/introducing-skills-the-open-agent-skills-ecosystem)
- [dot-claude](https://github.com/PaulRBerg/dot-claude)
- [dot-agents](https://github.com/PaulRBerg/dot-agents)

## License

MIT
