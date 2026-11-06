#!/bin/bash
# Print repository and Claude Code facts for agents-brain workflows as key=value lines.
# Exits 1 with an abort message on stderr when the working directory is unsupported.

set -euo pipefail

die() {
  printf 'abort: %s\n' "$1" >&2
  exit 1
}

cwd=$(pwd -P)
[ "$cwd" != '/' ] || die 'refusing to run at the filesystem root'

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || die 'not inside a git repository'

home=${HOME:-}
managed_skill_root=''
case "$repo_root" in
  /|"$home") die "unsupported repo root: $repo_root" ;;
  "$home/.agents"|"$home/.codex"|"$home/.claude") managed_skill_root=$repo_root/skills ;;
  "$home/.agents/"*|"$home/.codex/"*|"$home/.claude/"*)
    die "repo root is nested under an agent configuration repository: $repo_root" ;;
esac

if [ -n "$managed_skill_root" ]; then
  case "$cwd" in
    "$managed_skill_root"|"$managed_skill_root/"*)
      die "installed skills must be edited in their source catalog: $cwd" ;;
  esac
fi

claude_version=$(claude --version 2>/dev/null | head -n 1 | cut -d ' ' -f 1 || true)
agents_md_native=false
if [ -n "$claude_version" ] &&
  [ "$(printf '%s\n' 2.1.277 "$claude_version" | sort -V | head -n 1)" = 2.1.277 ]; then
  agents_md_native=true
fi

printf 'repo_root=%s\n' "$repo_root"
printf 'managed_skill_root=%s\n' "$managed_skill_root"
printf 'claude_version=%s\n' "$claude_version"
printf 'agents_md_native=%s\n' "$agents_md_native"
