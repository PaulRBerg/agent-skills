#!/usr/bin/env bash

set -u

usage() {
  printf 'Usage: bash <skill-dir>/scripts/prepare-commit.sh [--all] [--staged] [--natural] [--diff summary|full] -- [session_modified_paths...]\n' >&2
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

# Runs a git command that takes the index lock, retrying if another agent is
# mid-operation. Retries up to 5 attempts (4 retries) with a 1s pause between
# attempts, but only when the failure is an index.lock contention; any other
# failure is reported immediately.
run_with_lock_retry() {
  _lock_attempt=1
  while :; do
    _lock_output=$("$@" 2>&1)
    _lock_rc=$?
    [ "$_lock_rc" -eq 0 ] && return 0
    case "$_lock_output" in
      *index.lock*)
        [ "$_lock_attempt" -lt 5 ] || break
        _lock_attempt=$((_lock_attempt + 1))
        sleep 1
        ;;
      *)
        break
        ;;
    esac
  done
  printf '%s\n' "$_lock_output" >&2
  return 1
}

resolve_message_format() {
  if [ "$force_natural" = true ]; then
    printf 'natural\n'
    return 0
  fi

  [ -n "${HOME:-}" ] || die 'HOME is not set'

  _repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || die 'cannot resolve git repository root'
  _repo_root=$(physical_dir "$_repo_root") || die 'cannot resolve git repository root'

  _always_natural_language_repos=(
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

  for _natural_repo in "${_always_natural_language_repos[@]}"; do
    _natural_repo_root=$(physical_dir "$_natural_repo") || continue
    if [ "$_repo_root" = "$_natural_repo_root" ]; then
      printf 'natural\n'
      return 0
    fi
  done

  printf 'conventional\n'
}

all=false
staged=false
force_natural=false
diff_mode=summary
session_paths=()
preview_output=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --all)
      all=true
      shift
      ;;
    --staged)
      staged=true
      shift
      ;;
    --natural)
      force_natural=true
      shift
      ;;
    --diff)
      [ "$#" -ge 2 ] || {
        usage
        die '--diff requires summary or full'
      }
      diff_mode=$2
      case "$diff_mode" in
        summary | full) ;;
        *)
          usage
          die '--diff must be summary or full'
          ;;
      esac
      shift 2
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
      break
      ;;
  esac
done

while [ "$#" -gt 0 ]; do
  session_paths[${#session_paths[@]}]=$1
  shift
done

if [ "$all" = true ] && [ "$staged" = true ]; then
  usage
  die '--all and --staged are mutually exclusive'
fi

inside_work_tree=$(git rev-parse --is-inside-work-tree 2>/dev/null) || die 'not inside a git work tree'
[ "$inside_work_tree" = true ] || die 'not inside a git work tree'

# Anchor to the repository root so repo-root-relative session pathspecs resolve
# correctly regardless of the caller's current directory. Without this, running
# from a subdirectory makes git interpret pathspecs relative to that subdir,
# doubling the prefix and matching nothing.
repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || die 'cannot resolve git repository root'
cd "$repo_root" || die 'cannot enter git repository root'

merge_head=$(git rev-parse --git-path MERGE_HEAD 2>/dev/null) || die 'cannot resolve git state'
cherry_pick_head=$(git rev-parse --git-path CHERRY_PICK_HEAD 2>/dev/null) || die 'cannot resolve git state'
rebase_merge=$(git rev-parse --git-path rebase-merge 2>/dev/null) || die 'cannot resolve git state'
rebase_apply=$(git rev-parse --git-path rebase-apply 2>/dev/null) || die 'cannot resolve git state'

[ ! -f "$merge_head" ] || die 'merge in progress; resolve or abort it before committing'
[ ! -f "$cherry_pick_head" ] || die 'cherry-pick in progress; resolve or abort it before committing'
[ ! -d "$rebase_merge" ] || die 'rebase in progress; resolve or abort it before committing'
[ ! -d "$rebase_apply" ] || die 'rebase in progress; resolve or abort it before committing'

branch=$(git symbolic-ref --quiet --short HEAD 2>/dev/null) || die 'detached HEAD; check out a branch before committing'
message_format=$(resolve_message_format) || exit 1

if [ "$all" = true ]; then
  if [ -z "$(git status --porcelain=v1 --untracked-files=all)" ]; then
    die 'No changes to commit'
  fi
  run_with_lock_retry git add -A || die 'failed to stage all changes'
elif [ "$staged" = true ]; then
  : # commit the current index as-is; the empty-index guard below applies
else
  [ "${#session_paths[@]}" -gt 0 ] || die 'No files modified in this session'
  script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P) || die 'cannot resolve commit helper directory'
  preview_output=$(
    bash "$script_dir/commit-paths.sh" preview --diff "$diff_mode" -- "${session_paths[@]}"
  ) || exit 1
fi

if [ "$all" = true ] || [ "$staged" = true ]; then
  if git diff --cached --quiet --exit-code; then
    die 'No staged changes to commit'
  fi
fi

printf '## message format\n'
printf '%s\n\n' "$message_format"

printf '## branch\n'
printf '%s\n\n' "$branch"

if [ "$all" = true ] || [ "$staged" = true ]; then
  printf '## staged name-status\n'
  git diff --cached --name-status --no-ext-diff --no-textconv -- || die 'failed to print staged name-status'
  printf '\n'

  printf '## shortstat\n'
  git diff --cached --shortstat --no-ext-diff --no-textconv -- || die 'failed to print staged shortstat'

  if [ "$diff_mode" = full ]; then
    printf '\n## staged diff\n'
    git diff --cached --no-ext-diff --no-textconv -- || die 'failed to print staged diff'
  fi
else
  printf '%s\n' "$preview_output"
fi
