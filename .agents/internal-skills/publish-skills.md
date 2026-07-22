---
name: publish-skills
description:
  Commit and push catalog skills changed in the current chat or a user-specified commit range, surgically propagate only
  those global installations, then commit and push the affected global skill paths.
---

# Publish Skills

Publish and propagate only the catalog skills changed in the current chat, or in a commit range the user specifies.

## Workflow

### 1. Resolve the Skill Sets

Run in one of three modes, chosen by how the user invoked this skill:

- **Chat mode** (default when the working tree has session-modified paths): use the chat transcript as the source of
  ownership and Git state as supporting evidence. Inspect session-modified paths under `skills/<name>/`.
- **Unpushed-commits mode** (default when the working tree is clean and the user did not name commits): use the commits
  on the current branch that are ahead of its upstream (`git log @{upstream}..HEAD`) as the source of ownership, in
  place of the transcript, via `git show --stat` per commit. If the branch has no upstream configured or is not ahead of
  it, fall back to chat mode.
- **Commit-range mode**: the user names commits instead of relying on this chat's own edits — a date ("today"), a range,
  specific SHAs, or similar. Resolve the range with Git (e.g. `git log --since/--until`, explicit SHAs) and use
  `git show --stat` per commit as the source of ownership in place of the transcript. Confirm every resolved commit is
  reachable from the current branch; if some are and some aren't, stop and report the mismatch rather than guessing
  which the user meant.

In every mode, inspect the resolved paths under `skills/<name>/` and classify every attributable skill as introduced,
modified, or deleted.

- Ignore changes outside `skills/`, including internal skills and documentation.
- Treat a rename as one deletion plus one introduction.
- De-duplicate names and accept only kebab-case names matching `[a-z0-9]+(-[a-z0-9]+)*`.
- Stop before committing if no catalog skills changed in the resolved scope or if ownership or classification is
  ambiguous.

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

In commit-range mode and unpushed-commits mode the resolved commits are usually already committed, sometimes already
pushed. Skip creating a new commit when the working tree is clean; still push if the branch is ahead of its upstream. If
matching uncommitted changes exist locally, commit them as in chat mode before pushing.

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

`~/.claude/skills/<name>` is normally a symlink into `~/.agents/skills/<name>`, so a shared skill routinely has no
separate diff to commit there. This is expected, established repo layout, not a finding — skip that worktree silently
and do not explain the symlink relationship in the report.

### 6. Report

Report the source and global-repository commit summaries plus the introduced, refreshed, and deleted skill names. Omit
worktrees that were skipped because they had no diff; do not narrate why they had none. Completion requires every
recorded skill to match its declared target state and every resulting diff under `~/.agents/skills/<name>/` or
`~/.claude/skills/<name>/` to be committed and pushed.
