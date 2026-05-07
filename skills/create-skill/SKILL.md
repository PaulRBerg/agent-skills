---
argument-hint: <skill-name> [--project | --global]
disable-model-invocation: false
name: create-skill
user-invocable: true
description: This skill should be used when the user asks to "create a skill", "new skill", "scaffold a skill", "make a skill", "init a skill", or wants to bootstrap a new agent skill in `.agents/skills` (default) or `~/.agents/skills` (with `--global`).
---

# Create Skill

Bootstrap a new agent skill, then symlink it into `.claude/skills/` so Claude Code can discover it. Default to splitting bulk content into `references/` and `scripts/` so `SKILL.md` stays lean.

## Arguments

- **skill-name** (required): kebab-case name (e.g., `my-skill`). Stop if missing or invalid.
- `--global` (optional): install under `~` instead of the current repo.

## Resolved Paths

| Mode            | Skill source               | Claude Code symlink       |
| --------------- | -------------------------- | ------------------------- |
| local (default) | `.agents/skills/<name>/`   | `.claude/skills/<name>`   |
| `--global`      | `~/.agents/skills/<name>/` | `~/.claude/skills/<name>` |

The symlink target is always the relative path `../../.agents/skills/<name>` so it resolves correctly in both scopes.

## Skill Layout

```
<name>/
├── SKILL.md       # Required: frontmatter + lean workflow (aim for <500 lines)
├── scripts/       # Optional: executable code (Bash/Python/etc.) the workflow invokes
├── references/    # Optional: long-form docs loaded on demand
└── assets/        # Optional: templates / fonts / images used in OUTPUT (never loaded into context)
```

Agents load skills via **progressive disclosure**, in three stages:

1. **Discovery** — only `name` + `description` are visible at startup. Front-load triggers in `description`.
2. **Activation** — the full `SKILL.md` body is read once a task matches.
3. **Execution** — `scripts/` run without being read into context; `references/` are read only when `SKILL.md` explicitly links to them.

Keep `SKILL.md` focused on workflow. Push bulk into `scripts/` (deterministic logic) or `references/` (documentation).

## When to Split Content

### Use `scripts/` when

- The same code would be rewritten on every invocation (e.g., PDF rotate, JSON transform, curl wrapper).
- Determinism matters more than flexibility (parsing, validation, codegen, idempotent setup).
- A shell pipeline grows past ~5 lines or needs real error handling.
- A long heredoc keeps appearing inside `SKILL.md`.

Scripts are token-efficient: the agent invokes them without reading them. Document the CLI signature in `SKILL.md` and leave the implementation in `scripts/`.

### Use `references/` when

- A topic exceeds ~100 lines of prose, examples, or schemas.
- Content is conditionally relevant (variant-, framework-, or domain-specific) — splitting keeps irrelevant context out.
- Detailed API surfaces, DB schemas, policies, or large templates would otherwise dominate `SKILL.md`.
- The same explanation would be repeated across multiple skills (extract once, link from each).

Rules of thumb:

- **One level deep** — link `references/placeholder.md` directly from `SKILL.md`, never reference-to-reference.
- Files >100 lines: include a table of contents at the top.
- Files >10k words: document grep patterns in `SKILL.md` so the agent can locate sections without reading the whole file.
- **No duplication** — each fact lives in `SKILL.md` *or* a reference, never both.
- For every reference, write one line in `SKILL.md` that says *when* to read it.

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

Use `WebFetch` to confirm the current frontmatter schema, naming rules, and progressive-disclosure conventions. Do not guess — the spec evolves.

### 2. Validate

- Reject names that are not kebab-case or collide with an existing skill at the resolved path.
- Stop if `<scope>/.agents/skills/<name>/` or `<scope>/.claude/skills/<name>` already exists.

### 3. Plan the Layout

Before writing anything, decide what belongs where:

- Will the workflow invoke helper code? → `scripts/<name>.{sh,py,ts}`
- Schemas, long examples, variant guides, domain knowledge? → `references/<topic>.md`
- Templates or files the skill writes into the user's output? → `assets/`
- None of the above? → ship just `SKILL.md`.

Sketch the directory tree first, then create only the subdirectories the layout actually needs.

### 4. Create the Skill

```bash
mkdir -p "<scope>/.agents/skills/<name>"
# Add only the subdirectories the layout calls for:
# mkdir -p "<scope>/.agents/skills/<name>/scripts"
# mkdir -p "<scope>/.agents/skills/<name>/references"
```

Write `<scope>/.agents/skills/<name>/SKILL.md` with:

- Frontmatter sorted alphabetically, with `description` last. The `description` is the only field seen at discovery time — front-load trigger phrases there, not in the body.
- A short `# Title`.
- A one-line summary of what the skill does.
- `## Arguments` (if any) and `## Workflow` sections in **imperative form** with concrete steps.
- Explicit links to every `references/` file the workflow may need, each with a one-line note describing *when* to read it.
- CLI signatures for any bundled scripts so the agent can call them without reading them.

Aim for `SKILL.md` under 500 lines. If a section grows past ~50 lines and is not core workflow, move it to `references/` and link it.

### 5. Create the Claude Code Symlink

Always create a relative symlink so Claude Code picks the skill up from its own discovery path:

```bash
mkdir -p "<scope>/.claude/skills"
ln -s "../../.agents/skills/<name>" "<scope>/.claude/skills/<name>"
```

### 6. Verify

- `test -f "<scope>/.agents/skills/<name>/SKILL.md"`
- `readlink "<scope>/.claude/skills/<name>"` resolves to the source directory.
- Print both absolute paths to the user.

## Notes

- Frontmatter rule: sort fields alphabetically, but always place `description` last.
- "When to use" information belongs in `description` (discovery-time), not in the body (activation-time only).
- Use imperative / infinitive form throughout `SKILL.md`.
- All paths inside `SKILL.md` (e.g., `references/placeholder.md`, `scripts/example.sh`) are relative to the skill directory.
- Bash scripts inside the skill must be compatible with Bash 3.2 (`/bin/bash`), since Codex uses the built-in Bash by default.
- Do not commit the new skill — leave that to the user.
