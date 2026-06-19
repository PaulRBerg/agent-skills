#!/usr/bin/env bash

set -u

usage() {
  printf 'Usage: bash <skill-dir>/scripts/select-message-format.sh [--natural]\n' >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

physical_dir() {
  _dir=$1
  [ -d "$_dir" ] || return 1
  (cd "$_dir" 2>/dev/null && pwd -P)
}

force_natural=false

while [ "$#" -gt 0 ]; do
  case "$1" in
    --natural)
      force_natural=true
      shift
      ;;
    --)
      shift
      break
      ;;
    -*)
      usage
      die "unknown option: $1"
      ;;
    *)
      usage
      die "unexpected argument: $1"
      ;;
  esac
done

[ "$#" -eq 0 ] || {
  usage
  die "unexpected argument: $1"
}

if [ "$force_natural" = true ]; then
  printf 'natural\n'
  exit 0
fi

[ -n "${HOME:-}" ] || die 'HOME is not set'

inside_work_tree=$(git rev-parse --is-inside-work-tree 2>/dev/null) || die 'not inside a git work tree'
[ "$inside_work_tree" = true ] || die 'not inside a git work tree'

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || die 'cannot resolve git repository root'
repo_root=$(physical_dir "$repo_root") || die 'cannot resolve git repository root'

always_natural_language_repos=(
  "$HOME/.agents"
  "$HOME/.claude"
  "$HOME/.codex"
  "$HOME/.local/share/chezmoi"
  "$HOME/projects/agent-skills"
  "$HOME/projects/evm-sweeper"
  "$HOME/projects/home-control"
  "$HOME/projects/prb-chats"
  "$HOME/projects/prb-finance"
  "$HOME/work/mailops"
)

for natural_repo in "${always_natural_language_repos[@]}"; do
  natural_repo_root=$(physical_dir "$natural_repo") || continue
  if [ "$repo_root" = "$natural_repo_root" ]; then
    printf 'natural\n'
    exit 0
  fi
done

printf 'conventional\n'
