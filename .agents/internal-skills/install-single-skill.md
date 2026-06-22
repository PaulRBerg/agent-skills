---
name: install-single-skill
description: Commit and push this repo, then globally install exactly one named skill from the user prompt. Multiple skills are forbidden.
---

# Install Single Skill

Commit and push this repo, then install one named skill globally.

## Arguments

- `skill-name` (required): the single skill to install, taken from the surrounding user prompt.

## Workflow

### 1. Parse the Prompt

Read the surrounding user prompt that referenced this internal skill.

- Extract exactly one skill name.
- Accept only kebab-case names matching `[a-z0-9]+(-[a-z0-9]+)*`.
- Stop and ask for one skill name if the prompt is empty or ambiguous.
- Stop if the prompt requests `all`, `both`, a comma-separated list, a space-separated list, repeated `--skill` flags, globs, or any other multi-skill form.
- Do not infer additional skills or run this workflow in a loop. The user must reference this internal skill separately for each install.

### 2. Commit and Push This Repo

Run from the agent-skills repository root:

```bash
cd "$HOME/projects/agent-skills"
```

Invoke the local commit workflow exactly as:

```text
/commit --push
```

Wait for the commit and push to succeed before continuing. If the commit workflow fails, stop and report the failure; do
not install the skill.

### 3. Install the Single Skill

Run:

```bash
cd "$HOME/.agents"
bunx skills add PaulRBerg/agent-skills --global --yes --skill "<skill-name>"
```

Replace `<skill-name>` with the validated single skill name. Keep `--skill` present exactly once. Do not omit it, do not
replace it with catalog-wide install flags, and do not add any second skill.

### 4. Verify and Report

Check:

```bash
test -f "$HOME/.agents/skills/<skill-name>/SKILL.md"
```

Report the pushed commit summary and the installed skill path. If the install or verification fails, report the exact
command that failed and the relevant error output.
