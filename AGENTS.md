# Agent Skills

Collection of self-contained agent skills for Claude Code, Codex, and compatible agents. Keep `README.md` minimal and
human-facing; put maintainer and agent guidance here.

## Model Optimization

Optimize every skill and other agent-facing content for GPT-5.6 and Claude Fable 5. The summaries below are reminders,
not substitutes for the live guides. Read both guides before complex, long-running, multi-tool, or orchestration-heavy
work because their recommendations may evolve.

- [GPT-5.6 prompting guidance](https://developers.openai.com/api/docs/guides/prompt-guidance-gpt-5p6): Prefer lean,
  outcome-first prompts that specify the goal, success and stopping criteria, constraints, evidence, permission
  boundaries, tool routing, output shape, and validation. Remove redundant scaffolding and evaluate changes on
  representative tasks.
- [Claude Fable 5 prompting guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5):
  Use concise instructions that explain intent and boundaries; avoid over-prescription and scope creep; tune effort
  deliberately; ground progress claims in tool evidence; and make long-run verification and scaffolding explicit when
  needed.

## User-Facing Communication

Treat visual structure as information architecture, not decoration.

- Use a small semantic status vocabulary consistently: `🔎` preview/read-only, `⏳` running, `✅` verified success, `⚠️`
  caveat/approval/risk, `⛔` blocked/not written, `❓` unknown, and `↩` reverted/rolled back. When a status icon is
  used, pair it with a status word; never make color or emoji the only carrier of meaning.
- Lead reports with one outcome line. Use at most one non-status domain icon per heading for identity, then add compact
  headings for changed artifacts, verification, and risks only when those sections are useful.
- Use tables for repeated fields, trees for real hierarchy or file structure, and progress bars only when a measured
  numerator and denominator exist. Never imply progress from elapsed time, activity, or guesswork.
- Keep JSON, JSONL, DOT, CSV/TSV, exact commands, confirmation tokens, identifiers, addresses, hashes, signatures,
  diagnostics, logs, copied prompts, and other machine-consumed or verbatim content undecorated. Keep safety and
  rollback wording direct.
- Decoration belongs to the agent's status wrapper. Do not add emoji or ASCII ornament to source code, generated
  artifacts, product UI copy, GitHub contributions, spreadsheet data, or user-authored content unless the task itself
  calls for it.

## Structure

- `skills/<name>/SKILL.md` is the skill entrypoint.
- `skills/<name>/references/` contains skill-local reference docs.
- `skills/<name>/scripts/` contains executable helpers.
- `skills/<name>/agents/openai.yaml` contains Codex-specific metadata, including for skills not installed into Codex.
- `skills/<name>/examples/` contains sample files.
- `skills/<name>/assets/` contains bundled media or other static assets.
- `.agents/internal-skills/<name>.md` contains repo-private internal skills referenced with `@`.
- `README.md` lists every skill and stays minimal.
- `CLAUDE.md` is a symlink to `AGENTS.md`; do not edit it separately.

## Lint Rules

After editing Markdown, run these commands **in order**.

1. **`just prettier-write`** — format Markdown in place.
2. **`just prettier-check`** — verify formatting passes.

If `prettier-check` fails, analyze the errors and fix only files you changed.

## Commands

- `just` - list recipes.
- `just prettier-check` - check Markdown formatting with Prettier.
- `just prettier-write` - format Markdown in place.
- `just evm-atlas-check` - verify generated `evm-atlas` references match `@prb/crypto-registry`'s canonical chain JSON
  plus atlas overlays.
- `just evm-atlas-generate` - regenerate generated `evm-atlas` references from `@prb/crypto-registry`'s canonical chain
  JSON plus atlas overlays.
- `just evm-atlas-discover-routemesh` - refresh `atlas-overlays.json` `routeMesh` flags against RouteMesh's live
  `https://api.routeme.sh/chains` list (network call; run `just evm-atlas-generate` afterward to propagate).
- `just skill-invocation-check` - verify `SKILL.md` invocation fields match `agents/openai.yaml`.
- `just skill-invocation-fix` - update `agents/openai.yaml` invocation policy from `SKILL.md`.
- `just pre-commit` - run staged-file checks through `nlx lint-staged`.
- `just hooks-install` - install Husky hooks for this checkout through `nlx husky`.
- `just sync` - rough fallback that commits and pushes staged changes, performs a catalog-wide reinstall into
  `~/.agents` and `~/.claude`, then commits and pushes `~/.agents`.

`package.json` exists only for local formatting and hook wiring; there is no build step. Treat Markdown formatting,
invocation metadata checks, and skill-specific helper scripts as the verification surface unless a task introduces a
narrower check.

## Rules

- When asked to create, edit, or remove an installable catalog skill while the current working directory is this repo,
  modify the skill under `skills/` here only, not the installed copy under `~/.agents`.
- Changes here are not live until installed into every target declared by the skill. After editing installable catalog
  skills, recommend the `publish-skills` internal skill and offer to run `@publish-skills`; do not run it unprompted
  because it commits, pushes, and changes global installations. Use `just sync` only as a rough fallback when surgical,
  transcript-scoped propagation is unavailable.
- When an installable catalog skill is added or removed, update the skills table in `README.md`.
- Internal skills are special repo-private runbooks. Place them under `.agents/internal-skills/<name>.md`, not under
  `skills/`. Do not add them to `README.md`, do not create `agents/openai.yaml`, and do not treat them as installable
  catalog skills.
- After editing skills that must stay aligned, run the `sync-skills` internal skill to check coupled skills and helper
  data.
- Keep skills self-contained. Do not de-duplicate content across skills by extracting shared references or canonical
  files; users install skills individually.
- Write skill content for end users and other repos, not for this repo. Skills must not assume this repo's own tooling
  (e.g. `just prettier-write`, `just skill-invocation-check`) is present elsewhere; have skills detect and use whatever
  the target repo provides instead of naming this repo's recipes.
- Resolve `references/`, `scripts/`, `examples/`, and `assets/` paths relative to the owning skill directory.
- Every `skills/cli-*` skill must maintain `references/version.txt` with exactly one normalized semver for the CLI
  version the docs were last refreshed against: no leading `v`, prose, comments, ranges, prerelease labels, or extra
  lines. The wakeup automation maps `skills/cli-<name>` to binary `<name>` and refreshes the skill when the installed
  binary is newer than this file.
- Bash scripts must be compatible with Bash v3.2 (`/bin/bash`), because Codex uses the built-in Bash by default.
- In `SKILL.md` frontmatter, sort fields alphabetically but always place `description` last.
- Keep generated docs terse, imperative, and expert-to-expert.
- Never leak personal crypto (EVM) addresses in any skill. Use well-known public addresses (e.g. Multicall3, token
  contracts) or the standard Etherscan doc example (`0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe`) for examples; never
  hardcode a real user, maintainer, or personal wallet address. The same applies to private keys, mnemonics, and API
  keys — reference env-var placeholders (`$ETH_PRIVATE_KEY`), never literal secrets.

## Platform Targets

Skills may target Claude Code, Codex, or both. Declare exceptions in `SKILL.md` frontmatter with the repository-specific
string metadata field `metadata.install-targets`:

| Value               | Installation target   |
| ------------------- | --------------------- |
| Omitted             | Claude Code and Codex |
| `claude-code`       | Claude Code only      |
| `codex`             | Codex only            |
| `claude-code codex` | Claude Code and Codex |

Use only these exact values and canonical order. `~/.agents/justfile` is the policy consumer; agent clients and the
generic `skills` CLI do not interpret this field themselves.

- Add `compatibility` for the human-facing product and environment requirement.
- Write the skill only for its supported clients; do not add fallback branches for unsupported agents.
- Install the skill only into its declared targets. A target change must remove stale installations from targets that
  are no longer allowed.
- Keep `agents/openai.yaml` for every catalog skill. Its presence and invocation policy do not imply Codex
  compatibility.

## Codex Metadata

Reference: <https://developers.openai.com/codex/skills.md#optional-metadata>

Every skill must include `skills/<name>/agents/openai.yaml` with invocation policy derived from `SKILL.md`. `SKILL.md`
is authoritative.

```yaml
policy:
  allow_implicit_invocation: true # inverse of SKILL.md disable-model-invocation
```

Why: Claude and Codex store related invocation policy in different places. Claude reads `disable-model-invocation` from
`SKILL.md`; Codex reads `policy.allow_implicit_invocation` from `agents/openai.yaml`.

How: create an `agents/` directory next to `SKILL.md` and add `openai.yaml` with
`policy.allow_implicit_invocation: true` when `disable-model-invocation` is absent or `false`; use `false` only when
`disable-model-invocation: true`. If the file later needs UI metadata or tool dependencies, merge those fields into the
same file; do not remove the policy.

Codex `allow_implicit_invocation` defaults to `true`; when set to `false`, Codex will not choose the skill from the
prompt, but explicit `$skill` invocation still works. The Claude-equivalent implicit-invocation gate is the inverse of
`disable-model-invocation`.

`SKILL.md` is authoritative for invocation policy. When changing `disable-model-invocation`, run
`just skill-invocation-fix` to update `agents/openai.yaml` from the `SKILL.md` value.

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

Do not treat `agents/openai.yaml` as authoritative and do not treat `user-invocable` as Codex's implicit-invocation
equivalent. Claude documents `user-invocable` as slash-menu visibility, while `disable-model-invocation` controls
whether Claude can load the skill automatically. Codex has no equivalent metadata bit for `user-invocable`.

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
