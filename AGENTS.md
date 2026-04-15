# Agent Skills

Collection of agent skills for Claude Code, Codex, etc. See @README for full details.

## Rules

- When the user asks to create, edit, or remove a skill while the current working directory is this repo, modify the skill(s) here only (not globally)
- When a skill is added or removed, update the skills table in @README
- Bash scripts must be compatible with Bash v3.2 (`/bin/bash`), because Codex uses the built-in Bash by default
- In SKILL.md frontmatter, sort fields alphabetically but always place `description` last
