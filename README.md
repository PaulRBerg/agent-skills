# Agent Skills

PRB's collection of AI agent skills. Designed to work across agents, but primarily built for
[Claude Code](https://claude.com/product/claude-code) and [Codex](https://github.com/openai/codex).

> [!WARNING] This catalog intentionally reflects Paul's preferred tools, defaults, safety boundaries, and writing voice;
> it is not a neutral template. If you install it, review every workflow and customize it for your stack and agents. No
> warranties, guarantees, or support are provided — use at your own risk.

## Installation

```sh
bunx skills add PaulRBerg/agent-skills
```

## Skills

| Skill                     | Description                                                |
| ------------------------- | ---------------------------------------------------------- |
| agents-context-management | Create or polish README.md, AGENTS.md, and existing skills |
| agents-introspection      | Retrospect on Codex/Claude Code transcript history         |
| audit-stale-comments      | Verify stale comments in JavaScript, TypeScript, and Go    |
| autoresearch              | Autonomous experiment loop                                 |
| bump-deps                 | Node.js dependency updates via taze                        |
| bump-release              | Release workflow with changelog, tagging                   |
| cli-cast                  | Foundry cast CLI guidance                                  |
| cli-gh                    | GitHub CLI operations                                      |
| cli-just                  | Just command runner guidance                               |
| codex-handoff             | Delegate approved Claude plans to Codex implementation     |
| code-polish               | Simplify and/or risk-profiled review with autofix          |
| coingecko-cli             | CoinGecko CLI for prices and market data                   |
| coingecko-open-page       | Open CoinGecko historical data page in Chromium            |
| commit                    | Git commit with conventional commits                       |
| create-skill              | Bootstrap a new agent skill                                |
| debrief                   | Interactive HTML or Markdown task debrief                  |
| effect-ts                 | Effect-TS patterns and guidance                            |
| evm-atlas                 | EVM chain/account lookup + cross-chain data routing        |
| find-tool                 | Find and compare current developer tools                   |
| git-squash                | Squash PR branch with semantic commit message              |
| grill-me                  | Relentlessly stress-test plans and designs                 |
| large-file-refactor       | Large source-file report and Serena split plans            |
| night-shift               | Autonomous overnight codebase improvement                  |
| playground                | Interactive single-file HTML playgrounds                   |
| repo-rename               | Rename GitHub repo, folder, and agent thread references    |
| skill-doctor              | Audit Agent Skills catalogs and installed skill roots      |
| skill-map                 | Find skill dependencies and references across machine      |
| spreadsheets              | Opinionated CSV/TSV/XLSX wrangling on macOS                |
| tailwind-css              | Tailwind CSS v4 styling guidance                           |
| todo-archive              | Archive checked TODO.md tasks into .ai/todos               |
| vitest                    | Vitest test writing and debugging                          |
| yeet                      | GitHub contribution workflows                              |

## Forked Skills

| Skill      | Source                                                                                                                                                 |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| playground | [anthropics/claude-plugins-official: playground](https://github.com/anthropics/claude-plugins-official/tree/main/plugins/playground/skills/playground) |

Some mechanical skills pin `model: sonnet` in their frontmatter so they do not spend a more capable inherited model
where deterministic tooling already carries the workflow.

## References

- [Skills Issues](https://github.com/vercel-labs/skills/issues)
- [Introducing Skills](https://vercel.com/changelog/introducing-skills-the-open-agent-skills-ecosystem)
- [dot-claude](https://github.com/PaulRBerg/dot-claude)
- [dot-agents](https://github.com/PaulRBerg/dot-agents)

## License

MIT
