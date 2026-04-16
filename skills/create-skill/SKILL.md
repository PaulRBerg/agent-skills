---
argument-hint: <skill-name> [--global]
disable-model-invocation: false
name: create-skill
user-invocable: true
description: This skill should be used when the user asks to "create a skill", "new skill", "scaffold a skill", "make a skill", "init a skill", or wants to bootstrap a new agent skill in `.agents/skills` (default) or `~/.agents/skills` (with `--global`).
---

# Create Skill

Bootstrap a new agent skill, then symlink it into `.claude/skills/` so Claude Code can discover it.

## Arguments

- **skill-name** (required): kebab-case name (e.g., `my-skill`). Stop if missing or invalid.
- `--global` (optional): install under `~` instead of the current repo.

## Resolved Paths

| Mode              | Skill source                | Claude Code symlink             |
| ----------------- | --------------------------- | ------------------------------- |
| local (default)   | `.agents/skills/<name>/`    | `.claude/skills/<name>`         |
| `--global`        | `~/.agents/skills/<name>/`  | `~/.claude/skills/<name>`       |

The symlink target is always the relative path `../../.agents/skills/<name>` so it resolves correctly in both scopes.

## Workflow

### 1. Fetch Agent Skills Docs

Always fetch the latest spec before authoring frontmatter or content:

- https://agentskills.io

Use `WebFetch` to confirm the current SKILL.md frontmatter schema, naming rules, and conventions. Do not guess — the spec evolves.

### 2. Validate

- Reject names that are not kebab-case or collide with an existing skill at the resolved path.
- Stop if `<scope>/.agents/skills/<name>/` or `<scope>/.claude/skills/<name>` already exists.

### 3. Create the Skill

```bash
mkdir -p "<scope>/.agents/skills/<name>"
```

Write `<scope>/.agents/skills/<name>/SKILL.md` with:

- Frontmatter sorted alphabetically, with `description` last.
- A short `# Title`.
- A one-line summary of what the skill does.
- `## Arguments` (if any) and `## Workflow` sections with concrete steps.

Keep it minimal. Add `scripts/` or `references/` subdirectories only if the workflow needs them.

### 4. Create the Claude Code Symlink

Always create a relative symlink so Claude Code picks the skill up from its own discovery path:

```bash
mkdir -p "<scope>/.claude/skills"
ln -s "../../.agents/skills/<name>" "<scope>/.claude/skills/<name>"
```

### 5. Verify

- `test -f "<scope>/.agents/skills/<name>/SKILL.md"`
- `readlink "<scope>/.claude/skills/<name>"` resolves to the source directory.
- Print both absolute paths to the user.

## Notes

- Frontmatter rule: sort fields alphabetically, but always place `description` last.
- Bash scripts inside the skill should be compatible with Bash 3.2 (Codex's default shell).
- Do not commit the new skill — leave that to the user.
