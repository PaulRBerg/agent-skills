# Agent Skills

Collection of self-contained agent skills for Claude Code, Codex, and compatible agents. Keep `README.md` minimal and human-facing; put maintainer and agent guidance here.

## Structure

- `skills/<name>/SKILL.md` is the skill entrypoint.
- `skills/<name>/references/` contains skill-local reference docs.
- `skills/<name>/scripts/` contains executable helpers.
- `skills/<name>/agents/openai.yaml` contains Codex-specific metadata.
- `skills/<name>/examples/` contains sample files.
- `skills/<name>/assets/` contains bundled media or other static assets.
- `shelved/<name>/` contains retired skills preserved from the old `shelved` branch for reference or restoration.
- `.agents/internal-skills/<name>.md` contains repo-private internal skills referenced with `@`.
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
- `just evm-atlas-check` - verify generated `evm-atlas` references match `@prb/crypto-registry` plus atlas overlays.
- `just evm-atlas-generate` - regenerate generated `evm-atlas` references from `@prb/crypto-registry` plus atlas overlays.
- `just skill-invocation-check` - verify `SKILL.md` invocation fields match `agents/openai.yaml`.
- `just skill-invocation-fix` - update `agents/openai.yaml` invocation policy from `SKILL.md`.
- `just pre-commit` - run staged-file checks through `nlx lint-staged`.
- `just hooks-install` - install Husky hooks for this checkout through `nlx husky`.
- `just shelve <skill>` - require a clean worktree, move `skills/<skill>` to `shelved/<skill>`, and commit the move.
- `just sync` - commit your staged changes via `ccc --staged` and push, install skills into `~/.agents`, sync `~/.claude`, then commit and push there.
- `just unshelve <skill>` - require a clean worktree, move `shelved/<skill>` to `skills/<skill>`, and commit the move.

`package.json` exists only for hook and lint-staged wiring; there is no build step. Treat Markdown formatting, invocation metadata checks, and skill-specific helper scripts as the verification surface unless a task introduces a narrower check.

## Rules

- When asked to create, edit, or remove an installable catalog skill while the current working directory is this repo, modify the skill under `skills/` here only, not the installed copy under `~/.agents`.
- Changes here are not live for the agents until installed into `~/.agents` and symlinked into `~/.claude`. After committing or pushing in this repo, recommend `just sync` (it does that propagation) and offer to run it on the user's behalf; do not run it unprompted.
- When an installable catalog skill is added or removed, update the skills table in `README.md`.
- Shelved skills under `shelved/` are not installable catalog skills. Do not list them in `README.md` or sync them to `~/.agents`; keep `agents/openai.yaml` present so restoring a skill is a pure move plus modernization.
- To restore a shelved skill, move it from `shelved/<name>/` to `skills/<name>/`, bring it up to current repo rules, and update `README.md`.
- Internal skills are special repo-private runbooks. Place them under `.agents/internal-skills/<name>.md`, not under `skills/`. Do not add them to `README.md`, do not create `agents/openai.yaml`, and do not treat them as installable catalog skills.
- After editing skills that must stay aligned, run the `sync-skills` internal skill to check coupled skills and helper data.
- Keep skills self-contained. Do not de-duplicate content across skills by extracting shared references or canonical files; users install skills individually.
- Resolve `references/`, `scripts/`, `examples/`, and `assets/` paths relative to the owning skill directory.
- Every `skills/cli-*` skill must maintain `references/version.txt` with exactly one normalized semver for the CLI version the docs were last refreshed against: no leading `v`, prose, comments, ranges, prerelease labels, or extra lines. The wakeup automation maps `skills/cli-<name>` to binary `<name>` and refreshes the skill when the installed binary is newer than this file.
- Bash scripts must be compatible with Bash v3.2 (`/bin/bash`), because Codex uses the built-in Bash by default.
- In `SKILL.md` frontmatter, sort fields alphabetically but always place `description` last.
- Keep generated docs terse, imperative, and expert-to-expert.
- Never leak personal crypto (EVM) addresses in any skill. Use well-known public addresses (e.g. Multicall3, token contracts) or the standard Etherscan doc example (`0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe`) for examples; never hardcode a real user, maintainer, or personal wallet address. The same applies to private keys, mnemonics, and API keys — reference env-var placeholders (`$ETH_PRIVATE_KEY`), never literal secrets.

## Codex Metadata

Reference: <https://developers.openai.com/codex/skills.md#optional-metadata>

Every skill must include `skills/<name>/agents/openai.yaml` with invocation policy derived from `SKILL.md`. `SKILL.md` is authoritative.

```yaml
policy:
  allow_implicit_invocation: true # inverse of SKILL.md disable-model-invocation
```

Why: Claude and Codex store related invocation policy in different places. Claude reads `disable-model-invocation` from `SKILL.md`; Codex reads `policy.allow_implicit_invocation` from `agents/openai.yaml`.

How: create an `agents/` directory next to `SKILL.md` and add `openai.yaml` with `policy.allow_implicit_invocation: true` when `disable-model-invocation` is absent or `false`; use `false` only when `disable-model-invocation: true`. If the file later needs UI metadata or tool dependencies, merge those fields into the same file; do not remove the policy.

Codex `allow_implicit_invocation` defaults to `true`; when set to `false`, Codex will not choose the skill from the prompt, but explicit `$skill` invocation still works. The Claude-equivalent implicit-invocation gate is the inverse of `disable-model-invocation`.

`SKILL.md` is authoritative for invocation policy. When changing `disable-model-invocation`, run `just skill-invocation-fix` to update `agents/openai.yaml` from the `SKILL.md` value.

## Skill Frontmatter

Full reference: <https://code.claude.com/docs/en/skills>

### Invocation Control

Reference: <https://code.claude.com/docs/en/skills#control-who-invokes-a-skill>

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

Do not treat `agents/openai.yaml` as authoritative and do not treat `user-invocable` as Codex's implicit-invocation equivalent. Claude documents `user-invocable` as slash-menu visibility, while `disable-model-invocation` controls whether Claude can load the skill automatically. Codex has no equivalent metadata bit for `user-invocable`.

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
