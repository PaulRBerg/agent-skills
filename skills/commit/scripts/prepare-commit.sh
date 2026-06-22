#!/usr/bin/env bash

set -u

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd -P) || {
  printf 'error: cannot resolve commit skill script directory\n' >&2
  exit 1
}
commit_lock_token=
should_release_commit_lock=false

release_commit_lock() {
  [ "$should_release_commit_lock" = true ] || return 0
  [ -n "$commit_lock_token" ] || return 0
  bash "$script_dir/commit-lock.sh" release "$commit_lock_token" >/dev/null 2>&1 || true
}

trap release_commit_lock EXIT

usage() {
  printf 'Usage: bash <skill-dir>/scripts/prepare-commit.sh [--all] [--staged] [--natural] [--diff summary|full] -- [session_modified_paths...]\n' >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

add_unique_path() {
  _candidate=$1
  shift
  for _existing in "$@"; do
    if [ "$_existing" = "$_candidate" ]; then
      return 1
    fi
  done
  return 0
}

path_in_list() {
  _needle=$1
  shift
  for _candidate in "$@"; do
    if [ "$_candidate" = "$_needle" ]; then
      return 0
    fi
  done
  return 1
}

physical_dir() {
  _dir=$1
  [ -d "$_dir" ] || return 1
  (cd "$_dir" 2>/dev/null && pwd -P)
}

collect_path_output() {
  while IFS= read -r -d '' _path; do
    if add_unique_path "$_path" ${session_git_paths[@]+"${session_git_paths[@]}"}; then
      session_git_paths[${#session_git_paths[@]}]=$_path
    fi
  done
}

collect_stageable_paths() {
  for _path in "$@"; do
    if [ -e "$_path" ] || [ -L "$_path" ] || git ls-files --error-unmatch -- "$_path" >/dev/null 2>&1; then
      if add_unique_path "$_path" ${stageable_paths[@]+"${stageable_paths[@]}"}; then
        stageable_paths[${#stageable_paths[@]}]=$_path
      fi
    fi
  done
}

collect_unstage_path() {
  _path=$1
  if add_unique_path "$_path" ${unstage_paths[@]+"${unstage_paths[@]}"}; then
    unstage_paths[${#unstage_paths[@]}]=$_path
  fi
}

unstage_collected_paths() {
  [ "${#unstage_paths[@]}" -gt 0 ] || return 0

  _chunk=()
  for _path in "${unstage_paths[@]}"; do
    _chunk[${#_chunk[@]}]=$_path
    if [ "${#_chunk[@]}" -ge "$unstage_chunk_size" ]; then
      git restore --staged -- "${_chunk[@]}" || return 1
      _chunk=()
    fi
  done

  if [ "${#_chunk[@]}" -gt 0 ]; then
    git restore --staged -- "${_chunk[@]}" || return 1
  fi
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
unstage_chunk_size=100
session_paths=()
session_git_paths=()
stageable_paths=()
unstage_paths=()

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
commit_lock_token=$(bash "$script_dir/commit-lock.sh" acquire) || exit 1
should_release_commit_lock=true
message_format=$(resolve_message_format) || exit 1

if [ "$all" = true ]; then
  if [ -z "$(git status --porcelain=v1 --untracked-files=all)" ]; then
    die 'No changes to commit'
  fi
  git add -A || die 'failed to stage all changes'
elif [ "$staged" = true ]; then
  : # commit the current index as-is; the empty-index guard below applies
else
  [ "${#session_paths[@]}" -gt 0 ] || die 'No files modified in this session'

  collect_path_output < <(git diff --cached --name-only -z --no-ext-diff --no-textconv -- "${session_paths[@]}") || exit 1
  collect_path_output < <(git ls-files --full-name -z --modified --deleted --others --exclude-standard -- "${session_paths[@]}") || exit 1

  while IFS= read -r -d '' staged_status; do
    case "$staged_status" in
      R* | C*)
        IFS= read -r -d '' staged_old_path || die 'failed to parse staged rename/copy'
        IFS= read -r -d '' staged_new_path || die 'failed to parse staged rename/copy'
        if ! path_in_list "$staged_old_path" ${session_git_paths[@]+"${session_git_paths[@]}"} && ! path_in_list "$staged_new_path" ${session_git_paths[@]+"${session_git_paths[@]}"}; then
          collect_unstage_path "$staged_old_path"
          collect_unstage_path "$staged_new_path"
        fi
        ;;
      *)
        IFS= read -r -d '' staged_path || die 'failed to parse staged path'
        if ! path_in_list "$staged_path" ${session_git_paths[@]+"${session_git_paths[@]}"}; then
          collect_unstage_path "$staged_path"
        fi
        ;;
    esac
  done < <(git diff --cached --name-status -z --no-ext-diff --no-textconv)

  unstage_collected_paths || die 'failed to unstage unrelated staged paths'

  [ "${#session_git_paths[@]}" -gt 0 ] || die 'No files modified in this session'

  collect_stageable_paths "${session_paths[@]}"
  if [ "${#stageable_paths[@]}" -gt 0 ]; then
    git add -- "${stageable_paths[@]}" || die 'failed to stage session-modified paths'
  fi
fi

if git diff --cached --quiet --exit-code; then
  die 'No staged changes to commit'
fi

printf '## message format\n'
printf '%s\n\n' "$message_format"

printf '## branch\n'
printf '%s\n\n' "$branch"

printf '## commit lock token\n'
printf '%s\n\n' "$commit_lock_token"

printf '## staged name-status\n'
git diff --cached --name-status --no-ext-diff --no-textconv -- || die 'failed to print staged name-status'
printf '\n'

printf '## shortstat\n'
git diff --cached --shortstat --no-ext-diff --no-textconv -- || die 'failed to print staged shortstat'

if [ "$diff_mode" = full ]; then
  printf '\n## staged diff\n'
  git diff --cached --no-ext-diff --no-textconv -- || die 'failed to print staged diff'
fi

should_release_commit_lock=false
