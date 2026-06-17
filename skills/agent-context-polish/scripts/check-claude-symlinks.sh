#!/usr/bin/env bash
# Verify each AGENTS.md under the current repository has a sibling CLAUDE.md
# symlink that resolves to the same file.

set -euo pipefail

scan_root="$(pwd -P)"

if ! git -C "$scan_root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  printf 'ERROR: not inside a git worktree: %s\n' "$scan_root" >&2
  exit 2
fi

checked=0
failed=0
issues=""

record_issue() {
  failed=$((failed + 1))
  issues="${issues}  $1 - $2"$'\n'
}

while IFS= read -r -d '' agents_rel; do
  checked=$((checked + 1))

  agents_path="$scan_root/$agents_rel"
  agents_dir="${agents_path%/AGENTS.md}"
  claude_path="$agents_dir/CLAUDE.md"

  if [ ! -L "$claude_path" ]; then
    if [ -e "$claude_path" ]; then
      record_issue "$agents_rel" "CLAUDE.md exists but is not a symlink"
    else
      record_issue "$agents_rel" "missing sibling CLAUDE.md symlink"
    fi
    continue
  fi

  if [ ! -e "$claude_path" ]; then
    record_issue "$agents_rel" "CLAUDE.md symlink is broken"
    continue
  fi

  if [ ! "$claude_path" -ef "$agents_path" ]; then
    record_issue "$agents_rel" "CLAUDE.md symlink does not resolve to sibling AGENTS.md"
  fi
done < <(
  git -C "$scan_root" ls-files -z --cached --others --exclude-standard -- '**/AGENTS.md' 'AGENTS.md'
)

if [ "$failed" -gt 0 ]; then
  printf 'FAILED: %s of %s AGENTS.md file(s) lack valid CLAUDE.md symlink counterparts\n' "$failed" "$checked"
  printf '%s' "$issues"
  exit 1
fi

printf 'PASSED: %s AGENTS.md file(s) checked; all have valid CLAUDE.md symlink counterparts\n' "$checked"
