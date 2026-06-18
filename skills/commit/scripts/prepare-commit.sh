#!/usr/bin/env bash

set -u

usage() {
  printf 'Usage: bash <skill-dir>/scripts/prepare-commit.sh [--all] [--diff summary|full] -- [session_modified_paths...]\n' >&2
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

all=false
diff_mode=summary
session_paths=()
session_git_paths=()
stageable_paths=()

while [ "$#" -gt 0 ]; do
  case "$1" in
    --all)
      all=true
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

inside_work_tree=$(git rev-parse --is-inside-work-tree 2>/dev/null) || die 'not inside a git work tree'
[ "$inside_work_tree" = true ] || die 'not inside a git work tree'

merge_head=$(git rev-parse --git-path MERGE_HEAD 2>/dev/null) || die 'cannot resolve git state'
cherry_pick_head=$(git rev-parse --git-path CHERRY_PICK_HEAD 2>/dev/null) || die 'cannot resolve git state'
rebase_merge=$(git rev-parse --git-path rebase-merge 2>/dev/null) || die 'cannot resolve git state'
rebase_apply=$(git rev-parse --git-path rebase-apply 2>/dev/null) || die 'cannot resolve git state'

[ ! -f "$merge_head" ] || die 'merge in progress; resolve or abort it before committing'
[ ! -f "$cherry_pick_head" ] || die 'cherry-pick in progress; resolve or abort it before committing'
[ ! -d "$rebase_merge" ] || die 'rebase in progress; resolve or abort it before committing'
[ ! -d "$rebase_apply" ] || die 'rebase in progress; resolve or abort it before committing'

branch=$(git symbolic-ref --quiet --short HEAD 2>/dev/null) || die 'detached HEAD; check out a branch before committing'

if [ "$all" = true ]; then
  if [ -z "$(git status --porcelain=v1 --untracked-files=all)" ]; then
    die 'No changes to commit'
  fi
  git add -A || die 'failed to stage all changes'
else
  [ "${#session_paths[@]}" -gt 0 ] || die 'No files modified in this session'

  git diff --cached --name-only -- "${session_paths[@]}" >/dev/null || die 'failed to inspect staged session paths'
  git ls-files --full-name --modified --deleted --others --exclude-standard -- "${session_paths[@]}" >/dev/null || die 'failed to inspect session paths'

  collect_path_output < <(git diff --cached --name-only -z -- "${session_paths[@]}") || exit 1
  collect_path_output < <(git ls-files --full-name -z --modified --deleted --others --exclude-standard -- "${session_paths[@]}") || exit 1

  git diff --cached --name-status -- >/dev/null || die 'failed to inspect staged paths'

  while IFS= read -r -d '' staged_status; do
    case "$staged_status" in
      R* | C*)
        IFS= read -r -d '' staged_old_path || die 'failed to parse staged rename/copy'
        IFS= read -r -d '' staged_new_path || die 'failed to parse staged rename/copy'
        if ! path_in_list "$staged_old_path" ${session_git_paths[@]+"${session_git_paths[@]}"} && ! path_in_list "$staged_new_path" ${session_git_paths[@]+"${session_git_paths[@]}"}; then
          git restore --staged -- "$staged_old_path" "$staged_new_path" || die "failed to unstage unrelated path: $staged_old_path -> $staged_new_path"
        fi
        ;;
      *)
        IFS= read -r -d '' staged_path || die 'failed to parse staged path'
        if ! path_in_list "$staged_path" ${session_git_paths[@]+"${session_git_paths[@]}"}; then
          git restore --staged -- "$staged_path" || die "failed to unstage unrelated path: $staged_path"
        fi
        ;;
    esac
  done < <(git diff --cached --name-status -z)

  [ "${#session_git_paths[@]}" -gt 0 ] || die 'No files modified in this session'

  collect_stageable_paths "${session_paths[@]}"
  if [ "${#stageable_paths[@]}" -gt 0 ]; then
    git add -- "${stageable_paths[@]}" || die 'failed to stage session-modified paths'
  fi
fi

if git diff --cached --quiet --exit-code; then
  die 'No staged changes to commit'
fi

printf '## branch\n'
printf '%s\n\n' "$branch"

printf '## staged name-status\n'
git diff --cached --name-status --no-ext-diff --no-textconv -- || die 'failed to print staged name-status'
printf '\n'

printf '## shortstat\n'
git diff --cached --shortstat --no-ext-diff --no-textconv -- || die 'failed to print staged shortstat'

if [ "$diff_mode" = full ]; then
  printf '\n## staged diff\n'
  git diff --cached --no-ext-diff --no-textconv -- || die 'failed to print staged diff'
fi
