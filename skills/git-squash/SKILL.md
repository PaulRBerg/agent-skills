---
argument-hint: '[--subject "type: summary"] [--base <branch>]'
disable-model-invocation: false
name: git-squash
user-invocable: true
description: This skill should be used when the user asks to "squash PR commits", "squash my branch", "flatten branch history", "combine all commits into one", "prepare a clean PR commit", or "squash commits relative to main/default branch". It rewrites the current branch into a single commit whose message is derived only from the net diff relative to the default branch.
---

# Git Squash

Squash the current PR branch into one commit based on net branch changes relative to the default branch. Do not create or invoke a helper script. Run the Git commands directly.

## Arguments

Parse `$ARGUMENTS` for optional flags:

- `--subject <line>`: Override the generated commit subject line
- `--base <branch>`: Override default-branch auto-detection

Defaults:

- Subject: `chore: squash <current-branch> net changes`
- Base detection order:
  1. `refs/remotes/origin/HEAD`
  2. `git remote show origin`
  3. `main`, `master`, `trunk`

## Workflow

### 1) Pre-flight

Start by confirming that history can be rewritten safely. Stop on the first failure.

- Verify inside a Git worktree: `git rev-parse --is-inside-work-tree`
- Verify not detached: `git symbolic-ref --quiet --short HEAD`
- Verify working tree is clean: `git status --porcelain`
- After base detection, stop if the current branch is the default branch

```bash
git rev-parse --is-inside-work-tree
git symbolic-ref --quiet --short HEAD
git status --porcelain
```

### 2) Resolve the Base Branch

If `--base` was provided, use that branch name directly. Otherwise, detect the default branch in this order:

1. `refs/remotes/origin/HEAD`
2. `git remote show origin`
3. `main`, `master`, `trunk`

After the branch name is resolved, normalize it to a usable ref by preferring the local branch and falling back to `origin/<branch>`. Stop if neither exists. Also stop if the current branch is the default branch, because the skill should never squash the default branch into itself.

```bash
git symbolic-ref --quiet --short refs/remotes/origin/HEAD
git remote show origin
git show-ref --verify --quiet "refs/heads/$default_branch"
git show-ref --verify --quiet "refs/remotes/origin/$default_branch"
```

### 3) Find the Squash Boundary

Compute the merge-base between `HEAD` and the resolved default ref. That merge-base is the point where the branch diverged. Count how many commits are ahead of it. If the count is zero, there is nothing to squash.

```bash
merge_base="$(git merge-base HEAD "$default_ref")"
ahead_count="$(git rev-list --count "$merge_base..HEAD")"
original_head="$(git rev-parse HEAD)"
```

### 4) Rewrite the Branch into a Single Staged Diff

Soft-reset to the merge-base. This keeps the branch's net changes staged while removing the intermediate commits from history. If the staged diff is empty after the reset, restore the original head and stop with an error, because there is no net change to commit.

```bash
git reset --soft "$merge_base"
git diff --cached --quiet
git reset --soft "$original_head"
```

### 5) Build the Commit Message from the Net Diff

Use `--subject` when provided. Otherwise, generate:

`chore: squash <current-branch> net changes`

The body should describe the net diff only, not the original commit history. Start with:

`Net changes vs <default_ref>:`

Then append:

- The staged shortstat, if present
- One line per staged path from `git diff --cached --name-status`
- Rename lines in the form `<old> -> <new>`

```bash
git diff --cached --shortstat
git diff --cached --name-status
```

Write that message to a temporary file and commit with `git commit -F`.

### 6) Report the Result

After the commit succeeds:

- Report how many commits were squashed
- Report which default ref was used
- If the branch already exists on remote, remind the user to force-push with lease

```bash
git commit -F "$message_file"
git push --force-with-lease
```

## Behavior

- Stop immediately if not inside a git repository.
- Stop immediately if the current branch is the default branch.
- Stop if the working tree is dirty, to avoid mixing unrelated local edits.
- Reset softly to the merge-base with the default branch.
- Commit staged net changes as a single commit.
- Generate the commit message from the staged net diff only (shortstat + name-status lines).

## Output

- Prints how many commits were squashed.
- Prints the resolved default branch reference.
- Prints a reminder to force-push with lease when the branch already exists on remote.
