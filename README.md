# Agent Skills

PRB's collection of AI agent skills. Designed to work across agents, but primarily built for [Claude Code](https://claude.com/product/claude-code) and [Codex](https://github.com/openai/codex).

> [!WARNING]
> These skills are optimized for my personal setup and workflow. If you install them, do your own due diligence and customize them to fit your stack and agents. No warranties, guarantees, or support are provided — use at your own risk.

## Installation

```sh
bunx skills add PaulRBerg/agent-skills
```

## Skills

| Skill                | Description                                                |
| -------------------- | ---------------------------------------------------------- |
| agents-context       | Create or polish README.md, AGENTS.md, and existing skills |
| agents-introspection | Retrospect on Codex/Claude Code transcript history         |
| autoresearch         | Autonomous experiment loop                                 |
| bump-deps            | Node.js dependency updates via taze                        |
| bump-release         | Release workflow with changelog, tagging                   |
| cli-cast             | Foundry cast CLI guidance                                  |
| cli-gh               | GitHub CLI operations                                      |
| cli-just             | Just command runner guidance                               |
| code-polish          | Combined simplification and review                         |
| code-review          | Expert code review                                         |
| code-simplify        | Code simplification and refactoring                        |
| coingecko-cli        | CoinGecko CLI for prices and market data                   |
| coingecko-historical | Open CoinGecko historical data page in browser             |
| commit               | Git commit with conventional commits                       |
| create-skill         | Bootstrap a new agent skill                                |
| debrief              | Interactive HTML or Markdown task debrief                  |
| effect-ts            | Effect-TS patterns and guidance                            |
| evm-chains           | EVM chain resolution + Etherscan/Blockscout query routing  |
| find-tool            | Find and compare current developer tools                   |
| git-squash           | Squash PR branch with semantic commit message              |
| grill-me             | Relentlessly stress-test plans and designs                 |
| playground           | Interactive single-file HTML playgrounds                   |
| skill-map            | Find skill dependencies and references across machine      |
| spreadsheets         | Opinionated CSV/TSV/XLSX wrangling on macOS                |
| tailwind-css         | Tailwind CSS v4 styling guidance                           |
| todo-archive         | Archive checked TODO.md tasks into .ai/todos               |
| vitest               | Vitest test writing and debugging                          |
| work                 | End-to-end task implementation                             |
| yeet                 | GitHub contribution workflows                              |

Some skills pin `model: opus` in their frontmatter: their workflows are mechanical enough that Opus-level intelligence is sufficient, so there is no need to spend a more capable default model on them.

## References

- [Skills Issues](https://github.com/vercel-labs/skills/issues)
- [Introducing Skills](https://vercel.com/changelog/introducing-skills-the-open-agent-skills-ecosystem)
- [dot-claude](https://github.com/PaulRBerg/dot-claude)
- [dot-agents](https://github.com/PaulRBerg/dot-agents)

## License

MIT
