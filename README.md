# Agent Skills

PRB's collection of AI agent skills. Designed to work across agents, but primarily built for [Claude Code](https://claude.com/product/claude-code) and [Codex](https://github.com/openai/codex).

## Installation

```sh
npx skills add PaulRBerg/agent-skills
```

## Skills

| Skill           | Description                              |
| --------------- | ---------------------------------------- |
| biome-js        | BiomeJS linting/formatting guidance      |
| bump-release    | Release workflow with changelog, tagging |
| code-polish     | Combined simplification and review       |
| code-review     | Expert code review                       |
| cli-gh          | GitHub CLI operations                    |
| cli-just        | Just command runner guidance             |
| cli-sentry      | Sentry CLI issue management              |
| code-simplify   | Code simplification and refactoring      |
| commit          | Git commit with conventional commits     |
| delayed-command | Wait and execute bash command            |
| ls-lint         | Directory/filename linting               |
| md-docs         | Markdown documentation management        |
| bump-deps       | Node.js dependency updates               |
| openclaw        | OpenClaw CLI guidance                    |
| oracle-codex    | Codex oracle for planning                |
| oss             | OSS contribution workflows               |
| refine-prompt   | LLM prompt optimization                  |
| work            | End-to-end task implementation           |

## SKILL.md Frontmatter Guide

> [!NOTE]
> Full reference: [Claude Code Skills docs](https://code.claude.com/docs/en/skills)

### Invocation Control

These fields control who can invoke a skill — the user, Claude, or both:

| Field                      | Type      | Default | Effect                                                                            | Use when…                                                                        |
| -------------------------- | --------- | ------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `user-invocable`           | `boolean` | `true`  | Controls visibility in the `/` slash-command menu                                 | Set to `false` for background-knowledge skills Claude should auto-load silently  |
| `disable-model-invocation` | `boolean` | `false` | Prevents Claude from auto-loading the skill; removes its description from context | Set to `true` for side-effect workflows you trigger manually (deploy, commit, …) |

Combined behavior:

| Frontmatter                      | `/` menu | Claude auto-invokes | Description in context |
| -------------------------------- | -------- | ------------------- | ---------------------- |
| _(defaults)_                     | Yes      | Yes                 | Yes                    |
| `disable-model-invocation: true` | Yes      | No                  | No                     |
| `user-invocable: false`          | No       | Yes                 | Yes                    |
| Both `true` / `false`            | No       | No                  | No (skill is dead)     |

### Execution Context

The `context` field controls where a skill runs:

| Value       | Behavior                                                                                                          |
| ----------- | ----------------------------------------------------------------------------------------------------------------- |
| _(default)_ | Runs **inline** in the current conversation — the skill content is injected into the active context               |
| `fork`      | Runs in an **isolated subagent** — no access to conversation history; skill content becomes the subagent's prompt |

When `context: fork` is set, the optional `agent` field selects the subagent type:

| `agent` value | Description                                        |
| ------------- | -------------------------------------------------- |
| _(default)_   | `general-purpose` — full read/write tools          |
| `Explore`     | Read-only tools optimized for codebase exploration |
| `Plan`        | Read-only tools for designing implementation plans |
| Custom agent  | Any subagent defined in `.claude/agents/`          |

## References

- [Skills Issues](https://github.com/vercel-labs/skills/issues)
- [Introducing Skills](https://vercel.com/changelog/introducing-skills-the-open-agent-skills-ecosystem)
- [dot-claude](https://github.com/PaulRBerg/dot-claude)
- [dot-agents](https://github.com/PaulRBerg/dot-agents)

## License

MIT
