# Agent Skills

Collection of self-contained agent skills for Claude Code, Codex, and compatible agents. Keep `README.md` minimal and human-facing; put maintainer and agent guidance here.

## Structure

- `skills/<name>/SKILL.md` is the skill entrypoint.
- `skills/<name>/references/` contains skill-local reference docs.
- `skills/<name>/scripts/` contains executable helpers.
- `skills/<name>/examples/` contains sample files.
- `skills/<name>/assets/` contains bundled media or other static assets.
- `README.md` lists every skill and stays minimal.
- `CLAUDE.md` is a symlink to `AGENTS.md`; do not edit it separately.

## Lint Rules

After editing Markdown, run these commands **in order**.

1. **`just mdformat-write`** — format Markdown in place.
2. **`just mdformat-check`** — verify formatting passes.

If `mdformat-check` fails, analyze the errors and fix only files you changed.

## Commands

- `just` - list recipes.
- `just mdformat-check` - check Markdown formatting with `mdformat-gfm` and `mdformat-frontmatter`.
- `just mdformat-write` - format Markdown in place.
- `just sync` - commit this repo, install skills into `~/.agents`, then commit installed changes there.

There is no package manifest or build step. Treat Markdown formatting and skill-specific helper scripts as the verification surface unless a task introduces a narrower check.

## Rules

- When asked to create, edit, or remove a skill while the current working directory is this repo, modify the skill under `skills/` here only, not the installed copy under `~/.agents`.
- When a skill is added or removed, update the skills table in `README.md`.
- Keep skills self-contained. Do not de-duplicate content across skills by extracting shared references or canonical files; users install skills individually.
- Resolve `references/`, `scripts/`, `examples/`, and `assets/` paths relative to the owning skill directory.
- Bash scripts must be compatible with Bash v3.2 (`/bin/bash`), because Codex uses the built-in Bash by default.
- In `SKILL.md` frontmatter, sort fields alphabetically but always place `description` last.
- Keep generated docs terse, imperative, and expert-to-expert.

## Skill Frontmatter

Full reference: <https://code.claude.com/docs/en/skills>

### Invocation Control

Use these fields to control who can invoke a skill: the user, Claude, or both.

| Field                      | Type      | Default | Effect                                                                            | Use when                                                                       |
| -------------------------- | --------- | ------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `user-invocable`           | `boolean` | `true`  | Controls visibility in the `/` slash-command menu                                 | Claude should auto-load background knowledge without exposing a slash command  |
| `disable-model-invocation` | `boolean` | `false` | Prevents Claude from auto-loading the skill; removes its description from context | The skill is a side-effect workflow that should run only when invoked manually |

Combined behavior:

| Frontmatter                      | `/` menu | Claude auto-invokes | Description in context |
| -------------------------------- | -------- | ------------------- | ---------------------- |
| Defaults                         | Yes      | Yes                 | Yes                    |
| `disable-model-invocation: true` | Yes      | No                  | No                     |
| `user-invocable: false`          | No       | Yes                 | Yes                    |
| Both disabled                    | No       | No                  | No                     |

### Execution Context

Use `context` to control where a skill runs.

| Value   | Behavior                                                                  |
| ------- | ------------------------------------------------------------------------- |
| Default | Runs inline in the current conversation                                   |
| `fork`  | Runs in an isolated subagent without access to prior conversation history |

When `context: fork` is set, `agent` selects the subagent type.

| `agent` value | Description                                        |
| ------------- | -------------------------------------------------- |
| Default       | `general-purpose`, with full read/write tools      |
| `Explore`     | Read-only tools optimized for codebase exploration |
| `Plan`        | Read-only tools for implementation plans           |
| Custom agent  | Any subagent defined in `.claude/agents/`          |
