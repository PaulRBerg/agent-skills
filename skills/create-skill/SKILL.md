---
argument-hint: <skill-name> [--global]
disable-model-invocation: false
name: create-skill
user-invocable: true
description:
  Use to create/scaffold/init a new agent skill in `.agents/skills` by default or `~/.agents/skills` with `--global`.
---

# Create Skill

Bootstrap a skill with a small observable contract, then symlink it into `.claude/skills/` so Claude Code can discover
it. Keep invariant workflow guidance in `SKILL.md`; move deterministic mechanics and conditional detail into scripts and
references.

## Model Optimization

Optimize every new skill and its content for GPT-5.6 and Claude Fable 5. The summaries below are reminders, not
substitutes for the live guides. Read both guides before designing or writing a complex, long-running, multi-tool, or
orchestration-heavy skill because their recommendations may evolve.

- [GPT-5.6 prompting guidance](https://developers.openai.com/api/docs/guides/prompt-guidance-gpt-5p6): Prefer lean,
  outcome-first prompts that specify the goal, success and stopping criteria, constraints, evidence, permission
  boundaries, tool routing, output shape, and validation. Remove redundant scaffolding and evaluate changes on
  representative tasks.
- [Claude Fable 5 prompting guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5):
  Use concise instructions that explain intent and boundaries; avoid over-prescription and scope creep; tune effort
  deliberately; ground progress claims in tool evidence; and make long-run verification and scaffolding explicit when
  needed.

## Arguments

- **skill-name** (required): kebab-case name (e.g., `my-skill`). Stop if missing or invalid.
- `--global` (optional): install under `~` instead of the current repo.

## Resolved Paths

| Mode            | Skill source               | Claude Code symlink       |
| --------------- | -------------------------- | ------------------------- |
| local (default) | `.agents/skills/<name>/`   | `.claude/skills/<name>`   |
| `--global`      | `~/.agents/skills/<name>/` | `~/.claude/skills/<name>` |

For local mode, `<scope>` is the chosen project directory. It may be the repository root or a nested project/workspace
directory under a larger repo; a local `.agents/skills/` below the repo root is valid when that is the intended project
scope.

The symlink target is always the relative path `../../.agents/skills/<name>` so it resolves correctly in both scopes.

## Skill Layout

```
<name>/
├── SKILL.md       # Required: frontmatter + lean workflow (aim for <500 lines)
├── agents/
│   └── openai.yaml # Required: Codex metadata; disables implicit invocation
├── scripts/       # Optional: helper code (prefer TypeScript via bun run; Python via uv)
├── references/    # Optional: long-form docs loaded on demand
└── assets/        # Optional: templates / fonts / images used in OUTPUT (never loaded into context)
```

Agents load skills via **progressive disclosure**, in three stages:

1. **Discovery** — only `name` + `description` are visible at startup. Front-load triggers in `description`.
2. **Activation** — the full `SKILL.md` body is read once a task matches.
3. **Execution** — `scripts/` run without being read into context; `references/` are read only when `SKILL.md`
   explicitly links to them.

Keep `SKILL.md` focused on workflow. Push bulk into `scripts/` (deterministic logic) or `references/` (documentation).

## Authoring Contract

Before choosing a layout, separate the content into:

- **Invariants** that every valid execution must preserve.
- **Preferred defaults** that explicit user intent or repository evidence may override.
- **Conditional examples and references** loaded only when their branch is active.

Define the outcome, authority boundaries, stopping conditions, and completion evidence. Do not prescribe an identical
execution path when several safe paths satisfy the same contract. For user-facing workflows, also define which kickoff,
progress, decision, blocker, and completion events deserve a message and the smallest useful shape for each.

## When to Split Content

### Use `scripts/` when

- The same code would be rewritten on every invocation (e.g., PDF rotate, JSON transform, curl wrapper).
- Determinism matters more than flexibility (parsing, validation, codegen, idempotent setup).
- A shell pipeline grows past ~5 lines or needs real error handling.
- A long heredoc keeps appearing inside `SKILL.md`.

Scripts are token-efficient: the agent invokes them without reading them. Document the CLI signature in `SKILL.md` and
leave the implementation in `scripts/`.

Prefer `scripts/*.ts` run with `bun run scripts/<name>.ts`, unless there is a good reason TypeScript is the wrong fit
for the helper. Python is also a good choice for data, text, and file processing; run Python helpers through
`uv run scripts/<name>.py`, not raw `python` or `python3`.

### Use `references/` when

- A topic exceeds ~100 lines of prose, examples, or schemas.
- Content is conditionally relevant (variant-, framework-, or domain-specific) — splitting keeps irrelevant context out.
- Detailed API surfaces, DB schemas, policies, or large templates would otherwise dominate `SKILL.md`.
- A long explanation is needed only on one branch.

Rules of thumb:

- **One level deep** — link `references/placeholder.md` directly from `SKILL.md`, never reference-to-reference.
- Files >100 lines: include a table of contents at the top.
- Files >10k words: document grep patterns in `SKILL.md` so the agent can locate sections without reading the whole
  file.
- **No duplication** — each fact lives in `SKILL.md` _or_ a reference, never both. Keep skills self-contained rather
  than sharing references across independently installed skills.
- For every reference, write one line in `SKILL.md` that says _when_ to read it.

### Reference organization patterns

**Pattern A — High-level guide + topical references**

```
SKILL.md
references/
├── forms.md
├── api.md
└── examples.md
```

`SKILL.md` teaches the happy path; references hold deep-dive material.

**Pattern B — Domain or variant split**

```
SKILL.md           # workflow + selection logic
references/
├── aws.md
├── gcp.md
└── azure.md
```

The agent reads only the variant the user picked — irrelevant providers never enter context.

**Pattern C — Conditional details**

Inline the basic case in `SKILL.md`, link advanced files for edge cases (`tracked-changes.md`, `ooxml.md`, etc.).

### Do NOT add to a skill

- `README.md`, `INSTALLATION.md`, `CHANGELOG.md`, `QUICK_REFERENCE.md` — extraneous.
- Notes about how the skill was authored, test logs, scratch files.
- Anything the agent will not use at runtime.

## Workflow

### 1. Fetch Agent Skills Docs

Always fetch the latest spec before authoring frontmatter or content:

- https://agentskills.io

Use `WebFetch` to confirm the current frontmatter schema, naming rules, and progressive-disclosure conventions. Do not
guess — the spec evolves.

### 2. Validate

- Reject names that are not kebab-case or collide with an existing skill at the resolved path.
- Accept local scopes nested under a larger repo; do not force the skill into the repo root when the task targets a
  nested project directory.
- Stop if `<scope>/.agents/skills/<name>/` or `<scope>/.claude/skills/<name>` already exists.

### 3. Define the Contract and Layout

Before writing anything, define the observable outcome, invariants, preferred defaults, authority, routing, stop
conditions, and completion evidence. Then decide what belongs where:

- Will the workflow invoke helper code? → Prefer `scripts/<name>.ts` run with `bun run`; use `scripts/<name>.py` through
  `uv run` when Python is a better fit.
- Schemas, long examples, variant guides, domain knowledge? → `references/<topic>.md`
- Templates or files the skill writes into the user's output? → `assets/`
- None of the above? → ship just `SKILL.md`.

Sketch the directory tree first, then create only the subdirectories the layout actually needs.

### 4. Create the Skill

Before writing the frontmatter `description` or any body prose, read
[references/writing-great-skills.md](references/writing-great-skills.md) — the predictability levers that shape every
wording choice: invocation loads, one-trigger-per-branch descriptions, leading words, completion criteria, and the
no-op/pruning tests.

```bash
mkdir -p "<scope>/.agents/skills/<name>/agents"
# Add only the subdirectories the layout calls for:
# mkdir -p "<scope>/.agents/skills/<name>/scripts"
# mkdir -p "<scope>/.agents/skills/<name>/references"
```

Write `<scope>/.agents/skills/<name>/SKILL.md` with:

- Frontmatter sorted alphabetically, with `description` last. The `description` is the only field seen at discovery time
  — front-load trigger phrases there, not in the body.
- A short `# Title`.
- A one-line summary of what the skill does.
- `disable-model-invocation` and `user-invocable` fields set for Claude behavior. Omit only when intentionally relying
  on Claude defaults: `disable-model-invocation: false`, `user-invocable: true`.
- `## Arguments` (if any) and a lean imperative workflow. Use fixed steps only when order matters; otherwise state the
  contract and let repository evidence guide execution.
- Explicit links to every `references/` file the workflow may need, each with a one-line note describing _when_ to read
  it.
- CLI signatures for any bundled scripts, including the runtime command (`bun run scripts/<name>.ts` or
  `uv run scripts/<name>.py`), so the agent can call them without reading them.

Aim for `SKILL.md` under 500 lines. If a section grows past ~50 lines and is not core workflow, move it to `references/`
and link it.

Write `<scope>/.agents/skills/<name>/agents/openai.yaml` with:

```yaml
policy:
  allow_implicit_invocation: true
```

Set `allow_implicit_invocation` to the inverse of `SKILL.md` `disable-model-invocation`. If later adding Codex UI
metadata or MCP/tool dependencies, merge them into the same file and keep the policy.

### 5. Create the Claude Code Symlink

Always create a relative symlink so Claude Code picks the skill up from its own discovery path:

```bash
mkdir -p "<scope>/.claude/skills"
ln -s "../../.agents/skills/<name>" "<scope>/.claude/skills/<name>"
```

### 6. Verify

- `test -f "<scope>/.agents/skills/<name>/SKILL.md"`
- `test -f "<scope>/.agents/skills/<name>/agents/openai.yaml"`
- `readlink "<scope>/.claude/skills/<name>"` resolves to the source directory.
- Finish with `### 🧩 Skill created: <name>`, a tree of created paths, and `### ✅ Verified` with the exact checks. Link
  both absolute source and symlink paths.

## Notes

- Frontmatter rule: sort fields alphabetically, but always place `description` last.
- Codex parses `SKILL.md` frontmatter as YAML before loading a skill. Avoid unquoted colon-space tokens in scalar values
  such as `Triggers: "foo"` inside `description`; either omit the label or quote the whole value.
- "When to use" information belongs in `description` (discovery-time), not in the body (activation-time only).
- Use imperative / infinitive form throughout `SKILL.md`.
- All paths inside `SKILL.md` (e.g., `references/placeholder.md`, `scripts/example.sh`) are relative to the skill
  directory.
- Every new skill must include `agents/openai.yaml` with `policy.allow_implicit_invocation` derived from `SKILL.md`,
  never the other way around.
- Prefer TypeScript helper scripts run with `bun run`; use Python through `uv run`, never raw `python` or `python3`.
- Bash scripts inside the skill must be compatible with Bash 3.2 (`/bin/bash`), since Codex uses the built-in Bash by
  default.
- Keep helper stdout, commands, paths, frontmatter, and generated skill content undecorated unless that skill's own
  output contract requires otherwise.
