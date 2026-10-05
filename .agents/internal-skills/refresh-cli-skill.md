---
name: refresh-cli-skill
description:
  Refresh stale cli-* skills after installed CLI versions advance. Update docs from official upstream release docs and
  set references/version.txt.
---

# Refresh CLI Skill

Refresh one or more `skills/cli-*` entries after the installed CLI version is newer than the version in
`references/version.txt`. Circadian's daily maintenance job (`jobs/refresh-cli-skills.sh` in `~/projects/circadian`)
runs this runbook headlessly in an isolated clone of this repository. The job then commits, pushes, and publishes the
result.

## Arguments

- Accept arguments as `<cli-skill>=<version>`, for example `cli-just=1.43.0`.
- Treat each version as an already-normalized semver with no leading `v`.
- Stop if no valid skill/version pairs are provided.

## Workflow

1. Work in the current checkout. When started in an isolated clone, never touch the shared `~/projects/agent-skills`
   worktree.
2. Confirm the worktree is clean before editing. If it is dirty, stop and report the dirty paths.
3. For each requested skill:
   - Confirm `skills/<cli-skill>/SKILL.md` exists.
   - Map `cli-<name>` to binary `<name>`. `cli-coingecko` maps to `cg`. Confirm `<name> --version` still reports the
     requested version.
   - Read the current skill docs and references that mention versioned features, command flags, or upstream links.
   - Consult official upstream release notes, manuals, changelogs, or CLI docs for the requested version.
   - Update only stale or missing facts in the skill docs. Keep edits terse. Preserve local safety rules.
   - Write `skills/<cli-skill>/references/version.txt` with exactly the requested semver and one trailing newline.
4. Do not edit unrelated skills, generated lockfiles, installed copies under `~/.agents`, or global Claude/Codex config.
5. Run `just prettier-write <changed Markdown files>`. Then run `just prettier-check <changed files>` and
   `just skill-check`.
6. When the daily job invoked you, stop without committing or publishing. The job does both. Otherwise, commit and
   publish per `AGENTS.md`.

## Version Metadata

`references/version.txt` is the daily maintenance job's contract. `ai-skillet doctor` also enforces it. The file must
contain exactly one normalized semver:

```text
1.2.3
```

The file must have no leading `v`, prose, comments, ranges, prerelease labels, or extra lines.
