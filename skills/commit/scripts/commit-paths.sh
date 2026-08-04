#!/usr/bin/env bash

set -u

usage() {
  cat >&2 <<'EOF'
Usage:
  bash <skill-dir>/scripts/commit-paths.sh preview [--diff summary|full] -- <session_paths...>
  bash <skill-dir>/scripts/commit-paths.sh commit [-m <message>]... [--no-verify] [--no-gpg-sign] \
    [--push] [--exclude-baseline <path>=<oid>]... -- <resolved_paths...>
  bash <skill-dir>/scripts/commit-paths.sh push
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

append_unique_head_path() {
  _candidate=$1
  for _existing in ${head_paths[@]+"${head_paths[@]}"}; do
    [ "$_existing" = "$_candidate" ] && return 0
  done
  head_paths[${#head_paths[@]}]=$_candidate
}

append_unique_worktree_path() {
  _candidate=$1
  for _existing in ${worktree_paths[@]+"${worktree_paths[@]}"}; do
    [ "$_existing" = "$_candidate" ] && return 0
  done
  worktree_paths[${#worktree_paths[@]}]=$_candidate
}

append_unique_resolved_path() {
  _candidate=$1
  for _existing in ${resolved_paths[@]+"${resolved_paths[@]}"}; do
    [ "$_existing" = "$_candidate" ] && return 0
  done
  resolved_paths[${#resolved_paths[@]}]=$_candidate
}

cleanup() {
  _cleanup_rc=$?
  trap - EXIT INT TERM

  if [ -n "$alternate_index" ]; then
    rm -f "$alternate_index" "$alternate_index.lock"
  fi
  if [ -n "$index_info" ]; then
    rm -f "$index_info"
  fi
  if [ -n "$baseline_index" ]; then
    rm -f "$baseline_index" "$baseline_index.lock"
  fi
  if [ -n "$baseline_patch" ]; then
    rm -f "$baseline_patch"
  fi
  if [ -n "$temp_dir" ]; then
    rmdir "$temp_dir" 2>/dev/null || :
  fi
  if [ "$shared_lock_owned" = true ]; then
    rm -f "$shared_index_lock"
  fi

  exit "$_cleanup_rc"
}

normalize_path() {
  _path=$1

  case "$_path" in
    /*)
      case "$_path" in
        "$repo_root"/*) _path=${_path#"$repo_root"/} ;;
        *) die "path is outside the repository: $_path" ;;
      esac
      ;;
  esac

  while [ "${_path#./}" != "$_path" ]; do
    _path=${_path#./}
  done

  case "$_path" in
    '' | . | .. | ../* | */../* | */.. | */./* | */. | .git | .git/*)
      die "invalid repository path: $_path"
      ;;
  esac

  case "$_path" in
    *'
'*) die 'paths containing newlines are not supported' ;;
  esac

  printf '%s\n' "$_path"
}

# Test each component against directory entries instead of relying on `test -e`.
# On a case-insensitive filesystem, `test -e old.txt` also succeeds when the
# actual entry is Old.txt; Git needs the exact on-disk spelling for case renames.
exact_path_exists() {
  _remaining=$1
  _current=.

  while [ -n "$_remaining" ]; do
    case "$_remaining" in
      */*)
        _component=${_remaining%%/*}
        _remaining=${_remaining#*/}
        ;;
      *)
        _component=$_remaining
        _remaining=
        ;;
    esac

    _found=false
    while IFS= read -r -d '' _entry; do
      if [ "${_entry##*/}" = "$_component" ]; then
        _found=true
        break
      fi
    done < <(find "$_current" -mindepth 1 -maxdepth 1 -print0 2>/dev/null)

    [ "$_found" = true ] || return 1
    _current=$_current/$_component
  done

  [ -e "$_current" ] || [ -L "$_current" ]
}

collect_head_paths() {
  for _path in "${input_paths[@]}"; do
    while IFS= read -r -d '' _head_path; do
      append_unique_head_path "$_head_path"
    done < <(git ls-tree -r -z --name-only "$base_commit" -- ":(literal)$_path")
  done
}

collect_worktree_paths() {
  for _path in "${input_paths[@]}"; do
    exact_path_exists "$_path" || continue

    if [ -d "$_path" ] && [ ! -L "$_path" ]; then
      while IFS= read -r -d '' _worktree_path; do
        _worktree_path=${_worktree_path#./}
        if ! git check-ignore --quiet -- "$_worktree_path"; then
          append_unique_worktree_path "$_worktree_path"
        fi
      done < <(find "$_path" \( -type f -o -type l \) -print0)
    elif ! git check-ignore --quiet -- "$_path"; then
      append_unique_worktree_path "$_path"
    fi
  done
}

make_alternate_index() {
  _tmp_root=${TMPDIR:-/tmp}
  temp_dir=$(mktemp -d "$_tmp_root/commit-paths.XXXXXX") || die 'cannot create temporary directory'
  alternate_index=$temp_dir/index
  index_info=$temp_dir/index-info

  GIT_INDEX_FILE=$alternate_index git read-tree "$base_commit" || die 'cannot initialize isolated Git index'
}

validate_baseline_exclusions() {
  _baseline_number=0
  while [ "$_baseline_number" -lt "${#baseline_specs[@]}" ]; do
    _baseline_spec=${baseline_specs[$_baseline_number]}
    _baseline_path=${_baseline_spec%=*}
    _baseline_oid=${_baseline_spec##*=}
    _baseline_path=$(normalize_path "$_baseline_path") || exit 1

    _is_intended=false
    for _input_path in "${input_paths[@]}"; do
      if [ "$_input_path" = "$_baseline_path" ]; then
        _is_intended=true
        break
      fi
    done
    [ "$_is_intended" = true ] ||
      die "baseline path is not among intended paths: $_baseline_path"

    for _existing_path in ${baseline_paths[@]+"${baseline_paths[@]}"}; do
      [ "$_existing_path" != "$_baseline_path" ] ||
        die "duplicate baseline path: $_baseline_path"
    done

    case "$_baseline_oid" in
      '' | *[!0-9a-fA-F]*)
        die "invalid baseline blob OID for path $_baseline_path: $_baseline_oid"
        ;;
    esac
    _baseline_type=$(git cat-file -t "$_baseline_oid" 2>/dev/null) ||
      die "invalid baseline blob OID for path $_baseline_path: $_baseline_oid"
    [ "$_baseline_type" = blob ] ||
      die "baseline OID is not a blob for path $_baseline_path: $_baseline_oid"
    _baseline_oid=$(git rev-parse --verify "$_baseline_oid^{blob}" 2>/dev/null) ||
      die "cannot resolve baseline blob OID for path $_baseline_path: $_baseline_oid"

    baseline_paths[${#baseline_paths[@]}]=$_baseline_path
    baseline_oids[${#baseline_oids[@]}]=$_baseline_oid
    _baseline_number=$((_baseline_number + 1))
  done
}

baseline_mode() {
  _baseline_mode_path=$1
  _baseline_tree_entry=$(git ls-tree "$base_commit" -- ":(literal)$_baseline_mode_path") ||
    die "cannot inspect HEAD for baseline path: $_baseline_mode_path"

  if [ -n "$_baseline_tree_entry" ]; then
    _baseline_tree_tail=${_baseline_tree_entry#* }
    _baseline_tree_type=${_baseline_tree_tail%% *}
    [ "$_baseline_tree_type" = blob ] ||
      die "baseline path is not a file in HEAD: $_baseline_mode_path"
    printf '%s\n' "${_baseline_tree_entry%% *}"
  elif [ -L "$_baseline_mode_path" ]; then
    printf '120000\n'
  elif [ -f "$_baseline_mode_path" ]; then
    if [ -x "$_baseline_mode_path" ]; then
      printf '100755\n'
    else
      printf '100644\n'
    fi
  else
    die "baseline path is not a file in HEAD or the worktree: $_baseline_mode_path"
  fi
}

reset_isolated_path_to_head() {
  _reset_path=$1
  GIT_INDEX_FILE=$alternate_index git update-index --force-remove -- "$_reset_path" ||
    die "cannot reset isolated Git index for baseline path: $_reset_path"
  : > "$index_info" || die 'cannot prepare baseline index update'
  git ls-tree -r -z "$base_commit" -- ":(literal)$_reset_path" > "$index_info" ||
    die "cannot read HEAD entry for baseline path: $_reset_path"
  GIT_INDEX_FILE=$alternate_index git update-index -z --index-info < "$index_info" ||
    die "cannot restore HEAD entry for baseline path: $_reset_path"
}

apply_baseline_exclusions() {
  [ "${#baseline_paths[@]}" -gt 0 ] || return 0

  baseline_index=$temp_dir/baseline-index
  baseline_patch=$temp_dir/baseline.patch
  GIT_INDEX_FILE=$baseline_index git read-tree "$base_commit" ||
    die 'cannot initialize baseline Git index'

  _baseline_number=0
  while [ "$_baseline_number" -lt "${#baseline_paths[@]}" ]; do
    _baseline_path=${baseline_paths[$_baseline_number]}
    _baseline_oid=${baseline_oids[$_baseline_number]}
    _baseline_mode=$(baseline_mode "$_baseline_path") || exit 1

    GIT_INDEX_FILE=$baseline_index git update-index --add --cacheinfo \
      "$_baseline_mode" "$_baseline_oid" "$_baseline_path" ||
      die "cannot prepare baseline blob for path: $_baseline_path"
    if ! GIT_INDEX_FILE=$baseline_index git diff --binary --no-ext-diff --no-textconv \
      -- ":(literal)$_baseline_path" > "$baseline_patch"; then
      die "cannot diff baseline blob against the worktree for path: $_baseline_path"
    fi

    reset_isolated_path_to_head "$_baseline_path"
    if [ -s "$baseline_patch" ]; then
      if ! GIT_INDEX_FILE=$alternate_index git apply --cached --check --whitespace=nowarn \
        "$baseline_patch"; then
        die "baseline changes do not apply cleanly to HEAD for path: $_baseline_path"
      fi
      GIT_INDEX_FILE=$alternate_index git apply --cached --whitespace=nowarn "$baseline_patch" ||
        die "cannot apply baseline changes to HEAD for path: $_baseline_path"
    fi

    _baseline_number=$((_baseline_number + 1))
  done
}

build_isolated_index() {
  collect_head_paths
  collect_worktree_paths

  [ "${#head_paths[@]}" -gt 0 ] || [ "${#worktree_paths[@]}" -gt 0 ] || die 'No files modified in this session'

  make_alternate_index

  if [ "${#head_paths[@]}" -gt 0 ]; then
    GIT_INDEX_FILE=$alternate_index git update-index --force-remove -- "${head_paths[@]}" ||
      die 'cannot remove prior path entries from isolated Git index'
  fi

  if [ "${#worktree_paths[@]}" -gt 0 ]; then
    GIT_INDEX_FILE=$alternate_index git add -- "${worktree_paths[@]}" ||
      die 'cannot add working-tree paths to isolated Git index'
  fi

  apply_baseline_exclusions

  while IFS= read -r -d '' _resolved_path; do
    append_unique_resolved_path "$_resolved_path"
  done < <(
    GIT_INDEX_FILE=$alternate_index git diff --cached --name-only --no-renames -z \
      --no-ext-diff --no-textconv "$base_commit" --
  )

  [ "${#resolved_paths[@]}" -gt 0 ] || die 'No files modified in this session'
}

acquire_shared_index_lock() {
  _attempt=1
  while :; do
    if (umask 077 && set -C && : > "$shared_index_lock") 2>/dev/null; then
      shared_lock_owned=true
      break
    fi

    if [ ! -e "$shared_index_lock" ] && [ ! -L "$shared_index_lock" ]; then
      die "cannot create default Git index lock: $shared_index_lock"
    fi

    if [ "$_attempt" -ge 5 ]; then
      die "default Git index remains locked after $_attempt attempts: $shared_index_lock"
    fi
    _attempt=$((_attempt + 1))
    sleep 1
  done

  [ -f "$shared_index" ] || die "default Git index does not exist: $shared_index"
  cp -p "$shared_index" "$shared_index_lock" || die 'cannot snapshot the locked shared Git index'
}

reconcile_shared_index() {
  _created_commit=$1
  _zero_oid=$(git hash-object --stdin </dev/null | sed 's/./0/g') || return 1

  : > "$index_info" || return 1
  for _path in "${resolved_paths[@]}"; do
    printf '0 %s\t%s\0' "$_zero_oid" "$_path" >> "$index_info" || return 1
  done

  _tree_pathspecs=()
  for _path in "${resolved_paths[@]}"; do
    _tree_pathspecs[${#_tree_pathspecs[@]}]=":(literal)$_path"
  done
  git ls-tree -r -z "$_created_commit" -- "${_tree_pathspecs[@]}" >> "$index_info" || return 1

  GIT_INDEX_FILE=$shared_index_lock git update-index -z --index-info < "$index_info" || return 1
  mv "$shared_index_lock" "$shared_index" || return 1
  shared_lock_owned=false
}

reject_inherited_index() {
  if [ "${GIT_INDEX_FILE+x}" = x ]; then
    die 'GIT_INDEX_FILE is already set; run the helper from the default Git index context'
  fi
}

fetch_for_push() {
  push_git_output=$(git fetch 2>&1)
  push_git_rc=$?
  if [ "$push_git_rc" -ne 0 ]; then
    [ -z "$push_git_output" ] || printf '%s\n' "$push_git_output" >&2
    return "$push_git_rc"
  fi
}

check_push_behind() {
  push_compare_ref=
  if [ "$push_has_upstream" = true ]; then
    push_compare_ref='@{upstream}'
  elif git show-ref --verify --quiet "refs/remotes/origin/$push_branch"; then
    push_compare_ref=refs/remotes/origin/$push_branch
  fi

  push_behind=0
  [ -n "$push_compare_ref" ] || return 0

  push_counts=$(git rev-list --left-right --count "HEAD...$push_compare_ref" 2>&1)
  push_git_rc=$?
  if [ "$push_git_rc" -ne 0 ]; then
    [ -z "$push_counts" ] || printf '%s\n' "$push_counts" >&2
    return "$push_git_rc"
  fi

  read -r push_ahead push_behind <<EOF
$push_counts
EOF
  case "$push_behind" in
    '' | *[!0-9]*)
      printf 'error: cannot parse upstream comparison: %s\n' "$push_counts" >&2
      return 1
      ;;
  esac
}

attempt_push() {
  if [ "$push_has_upstream" = true ]; then
    push_git_output=$(git push 2>&1)
  else
    push_git_output=$(git push -u origin HEAD 2>&1)
  fi
  push_git_rc=$?
  return "$push_git_rc"
}

push_current_branch() {
  push_repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
    printf 'error: not inside a Git work tree\n' >&2
    return 1
  }
  cd "$push_repo_root" || {
    printf 'error: cannot enter Git repository root\n' >&2
    return 1
  }

  push_branch=$(git symbolic-ref --quiet --short HEAD 2>/dev/null) || {
    printf 'error: detached HEAD; cannot push without a current branch\n' >&2
    return 1
  }

  fetch_for_push || return $?

  push_has_upstream=false
  if git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' >/dev/null 2>&1; then
    push_has_upstream=true
  fi

  push_new_branch=false
  if [ "$push_has_upstream" = false ] &&
    ! git show-ref --verify --quiet "refs/remotes/origin/$push_branch"; then
    push_new_branch=true
  fi

  check_push_behind || return $?
  if [ "$push_behind" -ne 0 ]; then
    printf 'behind %s — push skipped\n' "$push_behind"
    return 0
  fi

  if attempt_push; then
    if [ "$push_new_branch" = true ]; then
      printf 'pushed-new %s\n' "$push_branch"
    else
      printf 'pushed %s\n' "$push_branch"
    fi
    return 0
  fi

  case "$push_git_output" in
    *non-fast-forward* | *'(fetch first)'*) ;;
    *)
      [ -z "$push_git_output" ] || printf '%s\n' "$push_git_output" >&2
      return "$push_git_rc"
      ;;
  esac

  fetch_for_push || return $?
  check_push_behind || return $?
  if [ "$push_behind" -ne 0 ]; then
    printf 'behind %s — push skipped\n' "$push_behind"
    return 0
  fi

  if ! attempt_push; then
    [ -z "$push_git_output" ] || printf '%s\n' "$push_git_output" >&2
    return "$push_git_rc"
  fi

  if [ "$push_new_branch" = true ]; then
    printf 'pushed-new %s\n' "$push_branch"
  else
    printf 'pushed %s\n' "$push_branch"
  fi
}

preflight_repository() {
  _inside_work_tree=$(git rev-parse --is-inside-work-tree 2>/dev/null) || die 'not inside a Git work tree'
  [ "$_inside_work_tree" = true ] || die 'not inside a Git work tree'

  repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || die 'cannot resolve Git repository root'
  cd "$repo_root" || die 'cannot enter Git repository root'

  _merge_head=$(git rev-parse --git-path MERGE_HEAD 2>/dev/null) || die 'cannot resolve Git state'
  _cherry_pick_head=$(git rev-parse --git-path CHERRY_PICK_HEAD 2>/dev/null) || die 'cannot resolve Git state'
  _rebase_merge=$(git rev-parse --git-path rebase-merge 2>/dev/null) || die 'cannot resolve Git state'
  _rebase_apply=$(git rev-parse --git-path rebase-apply 2>/dev/null) || die 'cannot resolve Git state'

  [ ! -f "$_merge_head" ] || die 'merge in progress; resolve or abort it before committing'
  [ ! -f "$_cherry_pick_head" ] || die 'cherry-pick in progress; resolve or abort it before committing'
  [ ! -d "$_rebase_merge" ] || die 'rebase in progress; resolve or abort it before committing'
  [ ! -d "$_rebase_apply" ] || die 'rebase in progress; resolve or abort it before committing'

  git symbolic-ref --quiet --short HEAD >/dev/null 2>&1 || die 'detached HEAD; check out a branch before committing'
  base_commit=$(git rev-parse --verify 'HEAD^{commit}' 2>/dev/null) || die 'repository has no HEAD commit'
}

command_name=${1:-}
[ -n "$command_name" ] || {
  usage
  exit 1
}
shift

diff_mode=summary
commit_args=()
baseline_specs=()
baseline_paths=()
baseline_oids=()
input_paths=()
head_paths=()
worktree_paths=()
resolved_paths=()
push_after_commit=false
repo_root=
base_commit=
temp_dir=
alternate_index=
index_info=
baseline_index=
baseline_patch=
shared_index=
shared_index_lock=
shared_lock_owned=false

case "$command_name" in
  preview)
    while [ "$#" -gt 0 ]; do
      case "$1" in
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
        *)
          usage
          die "unknown preview option: $1"
          ;;
      esac
    done
    ;;
  commit)
    while [ "$#" -gt 0 ]; do
      case "$1" in
        -m)
          [ "$#" -ge 2 ] || {
            usage
            die '-m requires a commit message'
          }
          commit_args[${#commit_args[@]}]=-m
          commit_args[${#commit_args[@]}]=$2
          shift 2
          ;;
        --no-verify | --no-gpg-sign)
          commit_args[${#commit_args[@]}]=$1
          shift
          ;;
        --push)
          push_after_commit=true
          shift
          ;;
        --exclude-baseline)
          [ "$#" -ge 2 ] || {
            usage
            die '--exclude-baseline requires <path>=<oid>'
          }
          _baseline_spec=$2
          case "$_baseline_spec" in
            *=*) ;;
            *)
              usage
              die '--exclude-baseline requires <path>=<oid>'
              ;;
          esac
          _baseline_path=${_baseline_spec%=*}
          _baseline_oid=${_baseline_spec##*=}
          [ -n "$_baseline_path" ] && [ -n "$_baseline_oid" ] || {
            usage
            die '--exclude-baseline requires non-empty <path>=<oid>'
          }
          baseline_specs[${#baseline_specs[@]}]=$_baseline_spec
          shift 2
          ;;
        --)
          shift
          break
          ;;
        *)
          usage
          die "unknown commit option: $1"
          ;;
      esac
    done
    ;;
  push)
    [ "$#" -eq 0 ] || {
      usage
      die 'push takes no arguments'
    }
    ;;
  *)
    usage
    die "unknown command: $command_name"
    ;;
esac

if [ "$command_name" = push ]; then
  push_current_branch
  exit $?
fi

reject_inherited_index
preflight_repository
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

[ "$#" -gt 0 ] || die 'No files modified in this session'
while [ "$#" -gt 0 ]; do
  _normalized_path=$(normalize_path "$1") || exit 1
  input_paths[${#input_paths[@]}]=$_normalized_path
  shift
done

validate_baseline_exclusions

if [ "$command_name" = commit ]; then
  _git_index_path=$(git rev-parse --git-path index 2>/dev/null) || die 'cannot resolve default Git index'
  case "$_git_index_path" in
    /*) shared_index=$_git_index_path ;;
    *) shared_index=$repo_root/$_git_index_path ;;
  esac
  shared_index_lock=$shared_index.lock

  acquire_shared_index_lock
  base_commit=$(git rev-parse --verify 'HEAD^{commit}' 2>/dev/null) || die 'cannot read locked HEAD commit'
fi

build_isolated_index

if [ "$command_name" = preview ]; then
  printf '## staged name-status\n'
  GIT_INDEX_FILE=$alternate_index git diff --cached --name-status --no-renames \
    --no-ext-diff --no-textconv "$base_commit" -- || die 'failed to print name-status'
  printf '\n## shortstat\n'
  GIT_INDEX_FILE=$alternate_index git diff --cached --shortstat \
    --no-ext-diff --no-textconv "$base_commit" -- || die 'failed to print shortstat'

  if [ "$diff_mode" = full ]; then
    printf '\n## staged diff\n'
    GIT_INDEX_FILE=$alternate_index git diff --cached --no-renames \
      --no-ext-diff --no-textconv "$base_commit" -- || die 'failed to print diff'
  fi

  printf '\n## commit paths\n'
  for _path in "${resolved_paths[@]}"; do
    printf '%s\n' "$_path"
  done
  exit 0
fi

GIT_INDEX_FILE=$alternate_index git commit "${commit_args[@]}"
commit_rc=$?
[ "$commit_rc" -eq 0 ] || exit "$commit_rc"

created_commit=$(git rev-parse --verify 'HEAD^{commit}' 2>/dev/null) ||
  die 'commit was created, but its commit ID could not be resolved; do not retry'

if ! reconcile_shared_index "$created_commit"; then
  printf 'error: commit %s was created, but committed paths could not be reconciled into the shared index\n' \
    "$created_commit" >&2
  printf 'error: the shared index was left unchanged; do not retry this commit\n' >&2
  exit 1
fi

if [ "$push_after_commit" = true ]; then
  if ! push_current_branch; then
    printf 'error: commit %s was created and reconciled, but push failed\n' "$created_commit" >&2
    exit 1
  fi
fi
