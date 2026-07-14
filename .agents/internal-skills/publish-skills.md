---
name: publish-skills
description:
  Commit and push catalog skills changed in the current chat, surgically propagate only those global installations, then
  commit and push the affected global skill paths.
---

# Publish Skills

Publish and propagate only the catalog skills changed in the current chat.

## Workflow

### 1. Resolve the Skill Sets

Use the chat transcript as the source of ownership and Git state as supporting evidence. Inspect session-modified paths
under `skills/<name>/` and classify every attributable skill as introduced, modified, or deleted.

- Ignore changes outside `skills/`, including internal skills and documentation.
- Treat a rename as one deletion plus one introduction.
- De-duplicate names and accept only kebab-case names matching `[a-z0-9]+(-[a-z0-9]+)*`.
- Stop before committing if no catalog skills changed in this chat or if ownership or classification is ambiguous.

For each introduced or modified skill, read `metadata.install-targets` from its current `SKILL.md` and group it as:

- Shared: omitted or `claude-code codex`.
- Claude Code only: `claude-code`.
- Codex only: `codex`.

Stop on any other value. Record the deleted, shared, Claude-only, and Codex-only sets before committing.

### 2. Commit and Push

From the repository root, invoke the commit skill exactly as:

```text
$commit --push
```

Do not add `--all`. Wait for both the commit and push to succeed. On failure, stop without changing global skill
installations.

### 3. Propagate Only the Recorded Skills

Run from the global agents repository:

```bash
cd "$HOME/.agents"
```

Run only the nonempty command groups below. Substitute the recorded names for the illustrative names; never use `*`,
`--all`, `skills update`, `just install-all`, or `just sync`.

First remove every deleted skill and every surviving affected skill with a restricted target. Removing restricted skills
before reinstalling clears stale universal or opposite-client installations.

```bash
bunx skills remove --global --skill "skill-a" "skill-b" --yes
```

Add or refresh shared skills:

```bash
bunx skills add PaulRBerg/agent-skills --global --agent claude-code codex --skill "skill-a" "skill-b" --yes
```

Add or refresh Claude-only skills:

```bash
bunx skills add PaulRBerg/agent-skills --global --agent claude-code --skill "skill-a" "skill-b" --yes
```

Add or refresh Codex-only skills:

```bash
bunx skills add PaulRBerg/agent-skills --global --agent codex --skill "skill-a" "skill-b" --yes
```

`skills add` refreshes an existing named installation and also handles newly introduced skills. If any command fails,
stop and report the failed command plus the sets already completed; do not fall back to a catalog-wide reinstall.

### 4. Verify the Installations

Verify every recorded skill:

- Shared: `SKILL.md` exists under both `~/.agents/skills/<name>/` and `~/.claude/skills/<name>/`.
- Claude-only: it exists under `~/.claude/skills/<name>/` and is absent from `~/.agents/skills/<name>/` and
  `~/.codex/skills/<name>/`.
- Codex-only: it exists under `~/.agents/skills/<name>/` or `~/.codex/skills/<name>/` and is absent from
  `~/.claude/skills/<name>/`.
- Deleted: it is absent from all three locations.

Treat a path as absent only when both `test ! -e` and `test ! -L` pass, so dangling symlinks fail verification.

### 5. Commit and Push the Global Repositories

Inspect `~/.agents` and `~/.claude` separately. In each Git worktree with a diff under a recorded skill path, invoke the
commit skill from that worktree as:

```text
$commit --push
```

Treat only `skills/<recorded-name>/` paths as session-modified paths. Include deletions with `git add -A` semantics, and
do not stage or commit unrelated changes. Skip a worktree when none of its recorded skill paths changed. Wait for every
required commit and push to succeed; if one fails, stop and report which global worktrees were already pushed.

### 6. Report

Report the source and global-repository commit summaries plus the introduced, refreshed, and deleted skill names.
Completion requires every recorded skill to match its declared target state and every resulting diff under
`~/.agents/skills/<name>/` or `~/.claude/skills/<name>/` to be committed and pushed.
