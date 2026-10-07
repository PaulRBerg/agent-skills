---
name: skill-authoring
description:
  Authoritative reference for catalog-skill metadata — SKILL.md frontmatter fields, invocation control, execution
  context, coordination exemption, install targets, Codex metadata, and skill dependencies.
---

# Skill Authoring

Use this authoritative reference for `SKILL.md` frontmatter and the metadata files that mirror it. Consult it before
creating or editing frontmatter, `agents/openai.yaml`, `metadata.install-targets`, or `skill-dependencies`. Do not guess
field semantics.

## STE Authoring and Review

Before writing catalog-skill prose, read
[the STE authoring profile](../../skills/skill-writing/references/asd-ste100.md) completely. Apply it to descriptions,
body prose, references, examples, and agent-facing metadata descriptions. Complete its meaning and style review before
finishing. Preserve obligation strength, technical precision, and protected literal content.

This requirement also applies when the repository catalog guard replaces the project-local `skill-writing` workflow. The
profile does not establish official ASD dictionary compliance.

## Portable Specification Cache

Before relying on portable Agent Skills rules, resolve the repository root. Then run the catalog's cache helper:

```bash
repo_root=$(git rev-parse --show-toplevel)
spec_file=$("$repo_root/skills/skill-writing/scripts/fetch-agentskills-spec.sh")
```

Read the file at `$spec_file` completely. Use `--refresh` for explicitly latest or change-sensitive work, disputed
guidance, or a conflict with validator behavior. Read a `stale` result before using it. For a `stale` result, disclose
its validation timestamp and retrieval failure. If the helper cannot return a valid file, stop before writing.

Never fetch the specification directly or create its cache in this repository.

## Skill Frontmatter

Full reference: <https://code.claude.com/docs/en/skills>

### Supported Dialect and Validation

`ai-skillet doctor --root '<skill-directory>'` is the canonical deterministic local validator. It accepts one extended
top-level field union:

- Portable Agent Skills fields from the cached specification: `name`, `description`, `license`, `compatibility`,
  `metadata`, and `allowed-tools`.
- [Claude Code](https://code.claude.com/docs/en/skills#frontmatter-reference) fields: `when_to_use`, `argument-hint`,
  `arguments`, `disable-model-invocation`, `user-invocable`, `disallowed-tools`, `model`, `effort`, `context`, `agent`,
  `background`, `hooks`, `paths`, and `shell`.
- Repository fields: `coordination` and `skill-dependencies`.

Unknown top-level fields are errors. `metadata` must be a string-to-string mapping. Tool, argument, and path fields
accept a string or a list of strings. `hooks` must be a mapping. Claude Boolean fields accept `true`/`false`,
`yes`/`no`, `on`/`off`, or `1`/`0`.

`context` accepts only `fork`. `effort` accepts `low`, `medium`, `high`, `xhigh`, or `max`. `shell` accepts `bash` or
`powershell`. `agent` and `background` require `context: fork`.

Use portable-only validators such as `skills-ref` or `agentskills` only as optional distribution-boundary checks when a
target explicitly requires the strict portable format. They are not authoritative for normal catalog authoring.

### Invocation Control

Reference: <https://code.claude.com/docs/en/skills#control-who-invokes-a-skill>

Use these fields to control who can invoke a skill: the user, Claude, or both.

| Field                      | Type      | Default | Effect                                                                            | Use when                                                                       |
| -------------------------- | --------- | ------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `user-invocable`           | `boolean` | `true`  | Controls visibility in the `/` slash-command menu                                 | Claude should auto-load background knowledge without exposing a slash command  |
| `disable-model-invocation` | `boolean` | `false` | Prevents Claude from auto-loading the skill. Removes its description from context | The skill is a side-effect workflow that should run only when invoked manually |

Combined behavior:

| Frontmatter                      | `/` menu | Claude auto-invokes | Description in context |
| -------------------------------- | -------- | ------------------- | ---------------------- |
| Defaults                         | Yes      | Yes                 | Yes                    |
| `disable-model-invocation: true` | Yes      | No                  | No                     |
| `user-invocable: false`          | No       | Yes                 | Yes                    |
| Both disabled                    | No       | No                  | No                     |

Do not treat `agents/openai.yaml` as authoritative. Do not treat `user-invocable` as Codex's implicit-invocation
equivalent. Claude documents `user-invocable` as slash-menu visibility. `disable-model-invocation` controls whether
Claude can load the skill automatically. Codex has no equivalent metadata bit for `user-invocable`.

Omit default-valued invocation fields. Absent `disable-model-invocation` means `false`. Absent `user-invocable` means
`true`. When behavior differs from those defaults, add only `disable-model-invocation: true` or `user-invocable: false`.

### Execution Context

Use `context` to control where a skill runs.

| Value   | Behavior                                                                  |
| ------- | ------------------------------------------------------------------------- |
| Default | Runs inline in the current conversation                                   |
| `fork`  | Runs in an isolated subagent without access to prior conversation history |

When `context: fork` is set, `agent` selects the subagent type. Both `agent` and `background` are invalid without
`context: fork`.

| `agent` value | Description                                        |
| ------------- | -------------------------------------------------- |
| Default       | `general-purpose`, with full read/write tools      |
| `Explore`     | Read-only tools optimized for codebase exploration |
| `Plan`        | Read-only tools for implementation plans           |
| Custom agent  | Any subagent defined in `.claude/agents/`          |

### Coordination Exemption

`coordination: exempt` is a repository-specific field, not a Claude Code or Codex feature. The agent reads it from the
skill body at invocation time. The global agent instructions define its meaning.

Set it only when the skill's declared default workflow writes no repository files or only repository metadata.
Explicitly authorized work that escalates beyond that declared behavior must enter the ai-coord gate.

An exempt skill skips the ai-coord gate for its declared work. Pair the field with one standard body sentence in
ordinary Markdown prose near the top of the skill. This lets the executing agent see the exemption without consulting
the frontmatter:

```markdown
This skill is coordination-exempt: skip the ai-coord gate for its declared work.
```

Explicitly authorized escalation beyond the declared write behavior re-enters the gate.

Inline code, fenced or indented code, blockquotes, and sections headed `Example` or `Examples` do not count as the
declaration. They do not trigger a missing-frontmatter error.

## Platform Targets

Skills may target Claude Code, Codex, or both. Declare exceptions in `SKILL.md` frontmatter with the repository-specific
string metadata field `metadata.install-targets`:

| Value               | Installation target   |
| ------------------- | --------------------- |
| Omitted             | Claude Code and Codex |
| `claude-code`       | Claude Code only      |
| `codex`             | Codex only            |
| `claude-code codex` | Claude Code and Codex |

Use only these exact values and canonical order. `scripts/publish-skills.ts` and `~/.agents/justfile` consume the field.
Agent clients and the generic `skills` CLI do not interpret this field themselves.

- Add `compatibility` for the human-facing product and environment requirement.
- Write the skill only for its supported clients. Do not add fallback branches for unsupported agents.
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
`SKILL.md`. Codex reads `policy.allow_implicit_invocation` from `agents/openai.yaml`.

How: Create an `agents/` directory next to `SKILL.md`. Add `openai.yaml` with the invocation policy. When
`disable-model-invocation` is absent or `false`, use `policy.allow_implicit_invocation: true`. Use `false` only when
`disable-model-invocation: true`. If the file later needs UI metadata or tool dependencies, merge those fields into the
same file. Do not remove the policy.

Codex `allow_implicit_invocation` defaults to `true`. When set to `false`, Codex will not choose the skill from the
prompt. Explicit `$skill` invocation still works. The Claude-equivalent implicit-invocation gate is the inverse of
`disable-model-invocation`.

`SKILL.md` is authoritative for invocation policy. When changing `disable-model-invocation`, run `just skill-fix` to
apply `ai-skillet doctor --fix-safe`, which rewrites `agents/openai.yaml`'s `policy.allow_implicit_invocation` from the
`SKILL.md` value.

## Skill Dependencies

In the custom top-level `skill-dependencies` array, declare every skill required, invoked, or receiving a handoff on a
supported branch. Conditional operational branches count. Suggestions, examples, related-skill references, and
underlying tool capabilities do not.

```yaml
skill-dependencies:
  - local-skill
  - OrgName/RepositoryName#external-skill
```

- When there are no dependencies, omit the field. Empty arrays are invalid.
- Use unique string entries. Bare names must resolve anywhere in the same repository and must not name the owning skill.
  External dependencies must use `ORG/REPO#SKILL`. Validation checks their shape without fetching the repository.
- Sort by target skill name — the bare name or substring after `#` — then by the complete identifier as a tie-breaker.

## Other Frontmatter Rules

- Sort `SKILL.md` frontmatter fields alphabetically, but always place `description` last.
- Run `ai-skillet doctor --root '<skill-directory>'` after authoring or changing a catalog skill. Require exit 0 for the
  changed skill before publishing it.
- Every `skills/cli-*` skill must maintain `references/version.txt` with exactly one normalized semver for the CLI
  version the docs were last refreshed against. The file must contain no leading `v`, prose, comments, ranges,
  prerelease labels, or extra lines. The weekly automation maps `skills/cli-<name>` to binary `<name>`. When the
  installed binary is newer than the version in this file, the automation refreshes the skill.
