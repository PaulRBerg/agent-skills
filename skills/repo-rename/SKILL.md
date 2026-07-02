---
argument-hint: <new-name>
disable-model-invocation: true
name: repo-rename
user-invocable: true
description: 'Rename a GitHub upstream repo and matching local project folder, update the local origin remote, and preserve Codex CLI and Claude Code thread continuity references.'
---

# Repo Rename

Rename the current GitHub repo, matching local folder, and local thread references for Codex CLI and Claude Code.

## Arguments

- `<new-name>`: Required GitHub repository name. Pass only the repo name, not an owner, path, or URL.

## Scope

- Handle only Codex CLI and Claude Code continuity files.
- Use `~/.claude/projects` for Claude Code. If `~/claude/projects` exists, inspect it too, but do not assume it is authoritative.
- Use `~/.codex/sessions` for Codex CLI. Do not update other Codex archives unless the user explicitly asks.
- Update `~/.codex/config.toml` when it contains the old repo path or repo name.
- Treat "local git ref" as the local `origin` remote URL.

## Workflow

Stop on the first unexpected failure. Do not proceed if any computed path or name is empty.

### 1) Preflight

Verify the target is a GitHub repo, the working tree is safe to move, and the folder name matches the upstream repo name.

```sh
new_name='<new-name>'

old_root="$(git rev-parse --show-toplevel)"
cd "$old_root"
old_name="$(basename "$old_root")"
parent_dir="$(dirname "$old_root")"
new_root="${parent_dir}/${new_name}"

case "$new_name" in
  ''|.*|*/*)
    printf 'invalid repo name: %s\n' "$new_name" >&2
    exit 64
    ;;
esac
if [ -e "$new_root" ]; then
  printf 'target folder already exists: %s\n' "$new_root" >&2
  exit 1
fi

git_status="$(git status --short)"
if [ -n "$git_status" ]; then
  printf '%s\n' "$git_status" >&2
  exit 1
fi

old_repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
repo_name="${old_repo##*/}"
owner="${old_repo%/*}"
if [ "$repo_name" != "$old_name" ]; then
  printf 'repo/folder mismatch: %s vs %s\n' "$repo_name" "$old_name" >&2
  exit 1
fi
```

- Stop if `git status --short` is non-empty unless the user explicitly wants the rename mixed with local edits.
- Stop if `$new_name` contains `/`, starts with `.`, or is empty.
- Stop if `$repo_name` differs from `$old_name`; the old repo name and local folder name need separate handling.
- Stop if `$new_root` already exists.
- Run `gh auth status` if GitHub auth is uncertain.

### 2) Rename GitHub and Update Origin

Rename the current upstream repo first, then update `origin` to the new URL while preserving SSH vs HTTPS style.

```sh
gh repo rename "$new_name" --yes

remote_url="$(git remote get-url origin)"
case "$remote_url" in
  git@github.com:*)
    git remote set-url origin "git@github.com:${owner}/${new_name}.git"
    ;;
  https://github.com/*)
    git remote set-url origin "https://github.com/${owner}/${new_name}.git"
    ;;
  *)
    git remote set-url origin "$(gh repo view "${owner}/${new_name}" --json sshUrl --jq .sshUrl)"
    ;;
esac
```

### 3) Move the Local Folder

Move from outside the repo, then continue from the new path.

```sh
cd "$parent_dir"
mv "$old_name" "$new_name"
cd "$new_root"
```

### 4) Preserve Claude Code Continuity

Claude Code stores project transcripts under a path-derived directory name. Rename that directory when present, then update literal path references inside Claude Code project files.

```sh
claude_projects="${HOME}/.claude/projects"
old_claude_dir="${claude_projects}/$(printf '%s' "$old_root" | sed 's#/#-#g')"
new_claude_dir="${claude_projects}/$(printf '%s' "$new_root" | sed 's#/#-#g')"

if [ -d "$old_claude_dir" ]; then
  test ! -e "$new_claude_dir"
  mv "$old_claude_dir" "$new_claude_dir"
fi
```

### 5) Preserve Codex CLI Continuity

Update literal old absolute paths in the active Codex CLI and Claude Code transcript stores, and update Codex CLI config when it references the old repo. Inspect matches before replacing.

```sh
export OLD_ROOT="$old_root" NEW_ROOT="$new_root"
export OLD_NAME="$old_name" NEW_NAME="$new_name"

for dir in "${HOME}/.claude/projects" "${HOME}/claude/projects" "${HOME}/.codex/sessions"; do
  test -d "$dir" || continue
  rg -n -F "$OLD_ROOT" "$dir" || true
  if rg -q -F "$OLD_ROOT" "$dir"; then
    rg -l -F "$OLD_ROOT" "$dir" | while IFS= read -r file; do
      perl -0pi -e 's/\Q$ENV{OLD_ROOT}\E/$ENV{NEW_ROOT}/g' "$file"
    done
  fi
done

codex_config="${HOME}/.codex/config.toml"
if [ -f "$codex_config" ]; then
  rg -n -F "$old_root" "$codex_config" || true
  rg -n -F "$old_name" "$codex_config" || true
  if rg -q -F "$old_root" "$codex_config"; then
    perl -0pi -e 's/\Q$ENV{OLD_ROOT}\E/$ENV{NEW_ROOT}/g' "$codex_config"
  fi
  if rg -q -F "$old_name" "$codex_config"; then
    perl -0pi -e 's/\Q$ENV{OLD_NAME}\E/$ENV{NEW_NAME}/g' "$codex_config"
  fi
fi
```

### 6) Update Repo-Local References

Inside the renamed repo, update references from the old repo/folder name to the new name. Inspect before replacing, and skip generated or dependency directories.

```sh
export OLD_NAME="$old_name" NEW_NAME="$new_name"

rg -n -F --hidden --glob '!.git' --glob '!node_modules' "$OLD_NAME" . || true
if rg -q -F --hidden --glob '!.git' --glob '!node_modules' "$OLD_NAME" .; then
  rg -l -F --hidden --glob '!.git' --glob '!node_modules' "$OLD_NAME" . | while IFS= read -r file; do
    perl -0pi -e 's/\Q$ENV{OLD_NAME}\E/$ENV{NEW_NAME}/g' "$file"
  done
fi
```

If the match set includes unrelated prose or data, replace only the files that clearly refer to the repo identity.

### 7) Verify

Confirm the GitHub repo, local remote, folder, and continuity references all point to the new name.

```sh
gh repo view "${owner}/${new_name}" --json nameWithOwner --jq .nameWithOwner
git remote get-url origin
pwd

for dir in "${HOME}/.claude/projects" "${HOME}/claude/projects" "${HOME}/.codex/sessions"; do
  test -d "$dir" || continue
  rg -n -F "$old_root" "$dir" || true
done
test ! -f "${HOME}/.codex/config.toml" || rg -n -F "$old_root" "${HOME}/.codex/config.toml" || true
test ! -f "${HOME}/.codex/config.toml" || rg -n -F "$old_name" "${HOME}/.codex/config.toml" || true
rg -n -F --hidden --glob '!.git' --glob '!node_modules' "$old_name" . || true
```

Report:

- The old and new GitHub repo names.
- The old and new local paths.
- Whether the Claude Code project directory was renamed.
- How many Claude Code and Codex CLI files were updated.
- Whether `~/.codex/config.toml` was updated.
- Any remaining old-name matches that were intentionally left alone.
