#!/usr/bin/env bash

set -o pipefail

umask 077

usage() {
  cat >&2 <<'EOF'
Usage:
  bash <skill-dir>/scripts/task-handoff.sh prepare \
    --repo <candidate-repository> [--repo <candidate-repository> ...] \
    --plan <owner-candidate> <PLAN_NAME.md> <concise-task> [--plan ...]
  bash <skill-dir>/scripts/task-handoff.sh finalize <run-dir>
  bash <skill-dir>/scripts/task-handoff.sh cancel <run-dir>
EOF
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

usage_error() {
  usage
  printf 'error: %s\n' "$*" >&2
  exit 64
}

has_line_break() {
  case $1 in
    *$'\n'* | *$'\r'*) return 0 ;;
    *) return 1 ;;
  esac
}

shell_quote() {
  printf "'"
  printf '%s' "$1" | sed "s/'/'\\\\''/g"
  printf "'"
}

valid_plan_filename() {
  printf '%s\n' "$1" | LC_ALL=C grep -Eq '^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*[.]md$'
}

contains_value() {
  _needle=$1
  shift
  for _candidate in "$@"; do
    [ "$_candidate" = "$_needle" ] && return 0
  done
  return 1
}

canonical_git_root() {
  _candidate=$1
  has_line_break "$_candidate" && return 1
  [ -d "$_candidate" ] || return 1

  _physical_candidate=$(cd "$_candidate" 2>/dev/null && pwd -P) || return 1
  _inside=$(git -C "$_physical_candidate" rev-parse --is-inside-work-tree 2>/dev/null) || return 1
  [ "$_inside" = true ] || return 1
  _root=$(git -C "$_physical_candidate" rev-parse --show-toplevel 2>/dev/null) || return 1
  _physical_root=$(cd "$_root" 2>/dev/null && pwd -P) || return 1
  has_line_break "$_physical_root" && return 1
  printf '%s\n' "$_physical_root"
}

directory_mode() {
  stat -f '%Lp' "$1" 2>/dev/null || stat -c '%a' "$1" 2>/dev/null
}

entry_count() {
  find "$1" -mindepth 1 -maxdepth 1 -print 2>/dev/null | wc -l | tr -d '[:space:]'
}

read_state_value() {
  _state_file=$1
  [ -f "$_state_file" ] && [ ! -L "$_state_file" ] || return 1
  [ "$(wc -l <"$_state_file" | tr -d '[:space:]')" = 1 ] || return 1
  _state_value=$(sed -n '1p' "$_state_file") || return 1
  has_line_break "$_state_value" && return 1
  printf '%s' "$_state_value"
}

write_state_value() {
  printf '%s\n' "$2" >"$1"
}

resolve_tools() {
  pbcopy_bin=${TASK_HANDOFF_TEST_PBCOPY:-/usr/bin/pbcopy}
  pbpaste_bin=${TASK_HANDOFF_TEST_PBPASTE:-/usr/bin/pbpaste}
  trash_bin=${TASK_HANDOFF_TEST_TRASH:-/usr/bin/trash}
  test_hook=${TASK_HANDOFF_TEST_HOOK:-}

  for _tool in "$pbcopy_bin" "$pbpaste_bin" "$trash_bin"; do
    has_line_break "$_tool" && die 'tool paths must not contain line breaks'
    [ -x "$_tool" ] || die "required executable is unavailable: $_tool"
  done
  if [ -n "$test_hook" ]; then
    has_line_break "$test_hook" && die 'test hook path must not contain line breaks'
    [ -x "$test_hook" ] || die "test hook is not executable: $test_hook"
  fi
}

validate_owner_parent() {
  _owner=$1
  for _directory in "$_owner/.ai" "$_owner/.ai/task-handoffs"; do
    if [ -e "$_directory" ] || [ -L "$_directory" ]; then
      [ -d "$_directory" ] && [ ! -L "$_directory" ] ||
        die "handoff parent must be a physical directory: $_directory"
    fi
  done
}

validate_plan_metadata() {
  [ "${#repo_roots[@]}" -gt 0 ] || die 'at least one repository is required'
  [ "${#plan_owners[@]}" -gt 0 ] || die 'at least one plan is required'

  _seen_roots=()
  for _root in "${repo_roots[@]}"; do
    _resolved_root=$(canonical_git_root "$_root") || die "not a Git worktree: $_root"
    [ "$_resolved_root" = "$_root" ] || die "repository root is no longer canonical: $_root"
    contains_value "$_root" "${_seen_roots[@]}" && die "duplicate repository root: $_root"
    _seen_roots[${#_seen_roots[@]}]=$_root
  done

  _seen_filenames=()
  _index=0
  while [ "$_index" -lt "${#plan_owners[@]}" ]; do
    _owner=${plan_owners[$_index]}
    _filename=${plan_filenames[$_index]}
    _task=${plan_tasks[$_index]}
    _relative=${plan_relpaths[$_index]}
    _target=${plan_targets[$_index]}

    contains_value "$_owner" "${repo_roots[@]}" || die "plan owner is not an involved repository: $_owner"
    valid_plan_filename "$_filename" || die "invalid plan filename: $_filename"
    contains_value "$_filename" "${_seen_filenames[@]}" && die "duplicate plan filename: $_filename"
    _seen_filenames[${#_seen_filenames[@]}]=$_filename

    [ -n "$_task" ] || die "concise task is empty for $_filename"
    has_line_break "$_task" && die "concise task must be one line for $_filename"
    [ "$_relative" = ".ai/task-handoffs/$_filename" ] || die "invalid relative target for $_filename"
    [ "$_target" = "$_owner/$_relative" ] || die "invalid absolute target for $_filename"

    _index=$((_index + 1))
  done
}

preflight_plan_set() {
  validate_plan_metadata
  resolve_tools

  _index=0
  while [ "$_index" -lt "${#plan_owners[@]}" ]; do
    _owner=${plan_owners[$_index]}
    _relative=${plan_relpaths[$_index]}
    _target=${plan_targets[$_index]}

    validate_owner_parent "$_owner"
    if [ -e "$_target" ] || [ -L "$_target" ]; then
      die "plan target already exists: $_target"
    fi
    git -C "$_owner" check-ignore -q -- "$_relative" || die "plan target is not ignored: $_target"

    _index=$((_index + 1))
  done
}

repo_roots=()
plan_owners=()
plan_filenames=()
plan_tasks=()
plan_relpaths=()
plan_targets=()
plan_drafts=()
plan_state_dirs=()

prepare_run_dir=
prepare_done=false

cleanup_failed_prepare() {
  _prepare_rc=$?
  trap - EXIT
  if [ "$prepare_done" != true ] && [ -n "$prepare_run_dir" ]; then
    case "$prepare_run_dir" in
      /*/task-handoff.*) rm -rf "$prepare_run_dir" ;;
    esac
  fi
  exit "$_prepare_rc"
}

prepare() {
  _raw_repos=()
  _raw_owners=()
  _raw_filenames=()
  _raw_tasks=()

  while [ "$#" -gt 0 ]; do
    case $1 in
      --repo)
        [ "$#" -ge 2 ] || usage_error '--repo requires one repository candidate'
        _raw_repos[${#_raw_repos[@]}]=$2
        shift 2
        ;;
      --plan)
        [ "$#" -ge 4 ] || usage_error '--plan requires an owner, filename, and concise task'
        _raw_owners[${#_raw_owners[@]}]=$2
        _raw_filenames[${#_raw_filenames[@]}]=$3
        _raw_tasks[${#_raw_tasks[@]}]=$4
        shift 4
        ;;
      -h | --help)
        usage
        exit 0
        ;;
      *) usage_error "unknown prepare option: $1" ;;
    esac
  done

  [ "${#_raw_repos[@]}" -gt 0 ] || usage_error 'prepare requires at least one --repo'
  [ "${#_raw_owners[@]}" -gt 0 ] || usage_error 'prepare requires at least one --plan'

  for _candidate in "${_raw_repos[@]}"; do
    _root=$(canonical_git_root "$_candidate") || die "not a Git worktree: $_candidate"
    if ! contains_value "$_root" "${repo_roots[@]}"; then
      repo_roots[${#repo_roots[@]}]=$_root
    fi
  done

  _index=0
  while [ "$_index" -lt "${#_raw_owners[@]}" ]; do
    _owner=$(canonical_git_root "${_raw_owners[$_index]}") ||
      die "plan owner is not a Git worktree: ${_raw_owners[$_index]}"
    contains_value "$_owner" "${repo_roots[@]}" ||
      die "plan owner is not among the involved repositories: ${_raw_owners[$_index]}"

    _filename=${_raw_filenames[$_index]}
    _task=${_raw_tasks[$_index]}
    plan_owners[${#plan_owners[@]}]=$_owner
    plan_filenames[${#plan_filenames[@]}]=$_filename
    plan_tasks[${#plan_tasks[@]}]=$_task
    plan_relpaths[${#plan_relpaths[@]}]=".ai/task-handoffs/$_filename"
    plan_targets[${#plan_targets[@]}]="$_owner/.ai/task-handoffs/$_filename"

    _index=$((_index + 1))
  done

  preflight_plan_set

  _tmp_root=${TASK_HANDOFF_TEST_TMPDIR:-${TMPDIR:-/tmp}}
  [ -d "$_tmp_root" ] || die "temporary directory does not exist: $_tmp_root"
  prepare_run_dir=$(mktemp -d "$_tmp_root/task-handoff.XXXXXX") || die 'cannot create temporary run directory'
  prepare_run_dir=$(cd "$prepare_run_dir" && pwd -P) || die 'cannot canonicalize temporary run directory'
  chmod 700 "$prepare_run_dir" || die 'cannot set temporary run directory mode'
  trap cleanup_failed_prepare EXIT

  mkdir "$prepare_run_dir/repos" "$prepare_run_dir/plans" || die 'cannot initialize temporary run state'
  printf 'task-handoff-run-v1\n%s\n%s\n%s\n' \
    "$prepare_run_dir" "${#repo_roots[@]}" "${#plan_owners[@]}" >"$prepare_run_dir/.task-handoff-run" ||
    die 'cannot write temporary run marker'

  _index=0
  while [ "$_index" -lt "${#repo_roots[@]}" ]; do
    printf -v _id '%04d' "$((_index + 1))"
    write_state_value "$prepare_run_dir/repos/$_id" "${repo_roots[$_index]}" ||
      die 'cannot write repository state'
    _index=$((_index + 1))
  done

  _index=0
  while [ "$_index" -lt "${#plan_owners[@]}" ]; do
    printf -v _id '%04d' "$((_index + 1))"
    _plan_dir=$prepare_run_dir/plans/$_id
    mkdir "$_plan_dir" || die 'cannot create plan draft state'
    write_state_value "$_plan_dir/owner" "${plan_owners[$_index]}" || die 'cannot write plan owner state'
    write_state_value "$_plan_dir/filename" "${plan_filenames[$_index]}" || die 'cannot write plan filename state'
    write_state_value "$_plan_dir/task" "${plan_tasks[$_index]}" || die 'cannot write plan task state'
    write_state_value "$_plan_dir/relative" "${plan_relpaths[$_index]}" || die 'cannot write plan path state'
    write_state_value "$_plan_dir/target" "${plan_targets[$_index]}" || die 'cannot write plan target state'
    : >"$_plan_dir/draft.md" || die 'cannot create plan draft'
    plan_drafts[${#plan_drafts[@]}]=$_plan_dir/draft.md
    _index=$((_index + 1))
  done

  prepare_done=true
  trap - EXIT

  printf 'run_dir %s\n' "$(shell_quote "$prepare_run_dir")"
  for _root in "${repo_roots[@]}"; do
    printf 'repo %s\n' "$(shell_quote "$_root")"
  done
  _index=0
  while [ "$_index" -lt "${#plan_owners[@]}" ]; do
    printf 'plan %s %s %s %s\n' \
      "$(shell_quote "${plan_owners[$_index]}")" \
      "$(shell_quote "${plan_relpaths[$_index]}")" \
      "$(shell_quote "${plan_targets[$_index]}")" \
      "$(shell_quote "${plan_drafts[$_index]}")"
    _index=$((_index + 1))
  done
}

validate_run_dir() {
  _candidate_run_dir=$1
  _allow_lock=$2

  has_line_break "$_candidate_run_dir" && die 'run directory must not contain line breaks'
  case $_candidate_run_dir in
    /*/task-handoff.*) ;;
    *) die "not a task-handoff run directory: $_candidate_run_dir" ;;
  esac
  [ -d "$_candidate_run_dir" ] && [ ! -L "$_candidate_run_dir" ] ||
    die "run directory does not exist: $_candidate_run_dir"
  [ -O "$_candidate_run_dir" ] || die "run directory is not owned by the current user: $_candidate_run_dir"
  [ "$(directory_mode "$_candidate_run_dir")" = 700 ] ||
    die "run directory mode is not 0700: $_candidate_run_dir"

  _physical_run_dir=$(cd "$_candidate_run_dir" && pwd -P) || die 'cannot canonicalize run directory'
  [ "$_physical_run_dir" = "$_candidate_run_dir" ] || die 'run directory path must be canonical and absolute'

  _marker=$_candidate_run_dir/.task-handoff-run
  [ -f "$_marker" ] && [ ! -L "$_marker" ] || die 'run marker is missing or invalid'
  [ "$(wc -l <"$_marker" | tr -d '[:space:]')" = 4 ] || die 'run marker has an invalid shape'
  [ "$(sed -n '1p' "$_marker")" = task-handoff-run-v1 ] || die 'run marker version is invalid'
  [ "$(sed -n '2p' "$_marker")" = "$_candidate_run_dir" ] || die 'run marker path does not match'
  _repo_count=$(sed -n '3p' "$_marker")
  _plan_count=$(sed -n '4p' "$_marker")
  case $_repo_count in '' | *[!0-9]*) die 'run marker repository count is invalid' ;; esac
  case $_plan_count in '' | *[!0-9]*) die 'run marker plan count is invalid' ;; esac
  [ "$_repo_count" -gt 0 ] || die 'run marker has no repositories'
  [ "$_plan_count" -gt 0 ] || die 'run marker has no plans'

  [ -d "$_candidate_run_dir/repos" ] && [ ! -L "$_candidate_run_dir/repos" ] || die 'repository state is invalid'
  [ -d "$_candidate_run_dir/plans" ] && [ ! -L "$_candidate_run_dir/plans" ] || die 'plan state is invalid'
  _expected_top_entries=3
  if [ "$_allow_lock" = true ]; then
    [ -d "$_candidate_run_dir/.lock" ] && [ ! -L "$_candidate_run_dir/.lock" ] || die 'run lock is invalid'
    [ "$(entry_count "$_candidate_run_dir/.lock")" = 0 ] || die 'run lock is not empty'
    _expected_top_entries=4
  elif [ -e "$_candidate_run_dir/.lock" ] || [ -L "$_candidate_run_dir/.lock" ]; then
    die 'run directory is already in use'
  fi
  [ "$(entry_count "$_candidate_run_dir")" = "$_expected_top_entries" ] ||
    die 'run directory contains unexpected state'
  [ "$(entry_count "$_candidate_run_dir/repos")" = "$_repo_count" ] || die 'repository state count does not match'
  [ "$(entry_count "$_candidate_run_dir/plans")" = "$_plan_count" ] || die 'plan state count does not match'

  repo_roots=()
  plan_owners=()
  plan_filenames=()
  plan_tasks=()
  plan_relpaths=()
  plan_targets=()
  plan_drafts=()
  plan_state_dirs=()

  _index=0
  while [ "$_index" -lt "$_repo_count" ]; do
    printf -v _id '%04d' "$((_index + 1))"
    _repo_state=$_candidate_run_dir/repos/$_id
    _root=$(read_state_value "$_repo_state") || die "invalid repository state: $_repo_state"
    [ -n "$_root" ] || die "empty repository state: $_repo_state"
    repo_roots[${#repo_roots[@]}]=$_root
    _index=$((_index + 1))
  done

  _index=0
  while [ "$_index" -lt "$_plan_count" ]; do
    printf -v _id '%04d' "$((_index + 1))"
    _plan_dir=$_candidate_run_dir/plans/$_id
    [ -d "$_plan_dir" ] && [ ! -L "$_plan_dir" ] || die "invalid plan state directory: $_plan_dir"
    [ "$(entry_count "$_plan_dir")" = 6 ] || die "plan state contains unexpected entries: $_plan_dir"
    _owner=$(read_state_value "$_plan_dir/owner") || die "invalid plan owner state: $_plan_dir"
    _filename=$(read_state_value "$_plan_dir/filename") || die "invalid plan filename state: $_plan_dir"
    _task=$(read_state_value "$_plan_dir/task") || die "invalid plan task state: $_plan_dir"
    _relative=$(read_state_value "$_plan_dir/relative") || die "invalid plan path state: $_plan_dir"
    _target=$(read_state_value "$_plan_dir/target") || die "invalid plan target state: $_plan_dir"
    _draft=$_plan_dir/draft.md
    [ -f "$_draft" ] && [ ! -L "$_draft" ] || die "invalid plan draft: $_draft"

    plan_owners[${#plan_owners[@]}]=$_owner
    plan_filenames[${#plan_filenames[@]}]=$_filename
    plan_tasks[${#plan_tasks[@]}]=$_task
    plan_relpaths[${#plan_relpaths[@]}]=$_relative
    plan_targets[${#plan_targets[@]}]=$_target
    plan_drafts[${#plan_drafts[@]}]=$_draft
    plan_state_dirs[${#plan_state_dirs[@]}]=$_plan_dir
    _index=$((_index + 1))
  done

  run_dir=$_candidate_run_dir
  validate_plan_metadata
}

run_dir=
lock_owned=false
rollback_required=false
created_dirs=()
created_targets=()
stage_files=()
run_temp_files=()

rollback_targets() {
  _index=${#created_targets[@]}
  while [ "$_index" -gt 0 ]; do
    _index=$((_index - 1))
    _target=${created_targets[$_index]}
    if [ -e "$_target" ] || [ -L "$_target" ]; then
      rm -f "$_target" 2>/dev/null || :
    fi
  done
}

rollback_directories() {
  _index=${#created_dirs[@]}
  while [ "$_index" -gt 0 ]; do
    _index=$((_index - 1))
    rmdir "${created_dirs[$_index]}" 2>/dev/null || :
  done
}

cleanup_operation() {
  _operation_rc=$?
  trap - EXIT INT TERM
  if [ "$rollback_required" = true ]; then
    rollback_targets
  fi
  for _temporary in "${stage_files[@]}" "${run_temp_files[@]}"; do
    [ -n "$_temporary" ] && rm -f "$_temporary" 2>/dev/null || :
  done
  if [ "$rollback_required" = true ]; then
    rollback_directories
  fi
  if [ "$lock_owned" = true ] && [ -n "$run_dir" ]; then
    rmdir "$run_dir/.lock" 2>/dev/null || :
  fi
  exit "$_operation_rc"
}

handle_signal() {
  case $1 in
    INT) exit 130 ;;
    TERM) exit 143 ;;
  esac
}

acquire_run_lock() {
  mkdir "$run_dir/.lock" 2>/dev/null || die "run directory is already in use: $run_dir"
  lock_owned=true
  validate_run_dir "$run_dir" true
}

remove_run_state() {
  _index=0
  while [ "$_index" -lt "${#plan_state_dirs[@]}" ]; do
    _plan_dir=${plan_state_dirs[$_index]}
    rm -f "$_plan_dir/owner" "$_plan_dir/filename" "$_plan_dir/task" \
      "$_plan_dir/relative" "$_plan_dir/target" "$_plan_dir/draft.md" || return 1
    rmdir "$_plan_dir" || return 1
    _index=$((_index + 1))
  done

  _index=0
  while [ "$_index" -lt "${#repo_roots[@]}" ]; do
    printf -v _id '%04d' "$((_index + 1))"
    rm -f "$run_dir/repos/$_id" || return 1
    _index=$((_index + 1))
  done

  rmdir "$run_dir/repos" "$run_dir/plans" || return 1
  rm -f "$run_dir/.task-handoff-run" || return 1
  rmdir "$run_dir/.lock" || return 1
  lock_owned=false
  rmdir "$run_dir" || return 1
}

validate_drafts() {
  _index=0
  while [ "$_index" -lt "${#plan_drafts[@]}" ]; do
    _draft=${plan_drafts[$_index]}
    _filename=${plan_filenames[$_index]}
    LC_ALL=C grep -q '[^[:space:]]' "$_draft" || die "plan draft is empty: $_filename"
    if grep -F -e '## Execution status' -e '## Handoff cleanup' "$_draft" >/dev/null; then
      die "plan draft contains a reserved heading: $_filename"
    fi
    _index=$((_index + 1))
  done
}

ensure_target_directory() {
  _owner=$1
  for _directory in "$_owner/.ai" "$_owner/.ai/task-handoffs"; do
    if [ -e "$_directory" ] || [ -L "$_directory" ]; then
      [ -d "$_directory" ] && [ ! -L "$_directory" ] ||
        die "handoff parent must be a physical directory: $_directory"
    elif mkdir "$_directory" 2>/dev/null; then
      created_dirs[${#created_dirs[@]}]=$_directory
    elif [ ! -d "$_directory" ] || [ -L "$_directory" ]; then
      die "cannot create handoff directory: $_directory"
    fi
  done
}

write_footer() {
  _footer=$1
  _target=$2
  _quoted_target=$(shell_quote "$_target")

  cat >"$_footer" <<'EOF'
## Execution status

Current status: No implementation attempt has been recorded.

If work stops before successful completion, replace the current status—not append an attempt history—with a concise
record of completed work, remaining work, validation commands and outcomes, the blocker, and the next concrete
action.

## Handoff cleanup

EOF
  printf '%s%s%s\n' 'Run `/usr/bin/trash ' "$_quoted_target" \
    '` only after every success criterion is satisfied and every required validation passes,' >>"$_footer"
  cat >>"$_footer" <<'EOF'
then verify the original path no longer exists. Keep this plan when work remains, implementation or validation fails,
or required validation is skipped. Never trash `.ai/task-handoffs/` or any other handoff.
EOF
}

validate_complete_file() {
  _complete=$1
  _footer=$2
  [ "$(grep -Fxc '## Execution status' "$_complete")" = 1 ] || return 1
  [ "$(grep -Fxc '## Handoff cleanup' "$_complete")" = 1 ] || return 1
  [ "$(grep -Fxc 'Current status: No implementation attempt has been recorded.' "$_complete")" = 1 ] || return 1
  _footer_bytes=$(wc -c <"$_footer" | tr -d '[:space:]')
  tail -c "$_footer_bytes" "$_complete" | cmp -s - "$_footer"
}

build_staged_plans() {
  _index=0
  while [ "$_index" -lt "${#plan_drafts[@]}" ]; do
    _owner=${plan_owners[$_index]}
    _filename=${plan_filenames[$_index]}
    _target=${plan_targets[$_index]}
    _draft=${plan_drafts[$_index]}
    _target_dir=$_owner/.ai/task-handoffs
    ensure_target_directory "$_owner"

    _stage=$(mktemp "$_target_dir/.task-handoff.$_filename.XXXXXX") || die "cannot stage plan: $_filename"
    stage_files[${#stage_files[@]}]=$_stage
    printf -v _id '%04d' "$((_index + 1))"
    _footer=$run_dir/.footer.$_id
    run_temp_files[${#run_temp_files[@]}]=$_footer
    write_footer "$_footer" "$_target" || die "cannot build plan footer: $_filename"

    cat "$_draft" >"$_stage" || die "cannot stage plan body: $_filename"
    _last_byte=$(tail -c 1 "$_draft" | od -An -tx1 | tr -d '[:space:]')
    [ "$_last_byte" = 0a ] || printf '\n' >>"$_stage" || die "cannot terminate plan body: $_filename"
    printf '\n' >>"$_stage" || die "cannot separate plan footer: $_filename"
    cat "$_footer" >>"$_stage" || die "cannot append plan footer: $_filename"
    validate_complete_file "$_stage" "$_footer" || die "staged plan failed structural validation: $_filename"

    rm -f "$_footer" || die "cannot remove temporary footer: $_filename"
    run_temp_files[_index]=
    _index=$((_index + 1))
  done
}

run_test_hook() {
  [ -n "$test_hook" ] || return 0
  "$test_hook" "$@" || die "test hook failed: $1"
}

publish_staged_plans() {
  _index=0
  while [ "$_index" -lt "${#stage_files[@]}" ]; do
    _target=${plan_targets[$_index]}
    _stage=${stage_files[$_index]}
    run_test_hook before_publish "$((_index + 1))" "$_target"
    if [ -e "$_target" ] || [ -L "$_target" ]; then
      die "plan target appeared during publication: $_target"
    fi
    ln "$_stage" "$_target" 2>/dev/null || die "cannot publish plan without overwriting: $_target"
    created_targets[${#created_targets[@]}]=$_target
    run_test_hook after_publish "$((_index + 1))" "$_target"
    _index=$((_index + 1))
  done

  _index=0
  while [ "$_index" -lt "${#stage_files[@]}" ]; do
    _target=${plan_targets[$_index]}
    _stage=${stage_files[$_index]}
    [ -f "$_target" ] && [ ! -L "$_target" ] || die "published plan is missing: $_target"
    [ "$_target" -ef "$_stage" ] || die "published plan changed during validation: $_target"
    cmp -s "$_target" "$_stage" || die "published plan bytes changed during validation: $_target"
    git -C "${plan_owners[$_index]}" check-ignore -q -- "${plan_relpaths[$_index]}" ||
      die "published plan is no longer ignored: $_target"
    _index=$((_index + 1))
  done
}

build_commands() {
  commands=()
  _index=0
  while [ "$_index" -lt "${#plan_owners[@]}" ]; do
    _prompt="A previous agent worked on ${plan_tasks[$_index]} and produced an implementation plan under ${plan_relpaths[$_index]}. Implement it."
    _command="codex -C $(shell_quote "${plan_owners[$_index]}") $(shell_quote "$_prompt")"
    commands[${#commands[@]}]=$_command
    _index=$((_index + 1))
  done
}

copy_and_verify_commands() {
  _expected=$run_dir/.clipboard.expected
  _actual=$run_dir/.clipboard.actual
  run_temp_files[${#run_temp_files[@]}]=$_expected
  run_temp_files[${#run_temp_files[@]}]=$_actual

  : >"$_expected" || die 'cannot create expected clipboard payload'
  _index=0
  while [ "$_index" -lt "${#commands[@]}" ]; do
    [ "$_index" -eq 0 ] || printf '\n' >>"$_expected" || die 'cannot build clipboard payload'
    printf '%s' "${commands[$_index]}" >>"$_expected" || die 'cannot build clipboard payload'
    _index=$((_index + 1))
  done

  "$pbcopy_bin" <"$_expected" || die 'clipboard copy failed'
  "$pbpaste_bin" >"$_actual" || die 'clipboard readback failed'
  cmp -s "$_expected" "$_actual" || die 'clipboard verification failed'
}

finalize() {
  [ "$#" -eq 1 ] || usage_error 'finalize requires exactly one run directory'
  validate_run_dir "$1" false
  trap cleanup_operation EXIT
  trap 'handle_signal INT' INT
  trap 'handle_signal TERM' TERM
  acquire_run_lock

  preflight_plan_set
  validate_drafts
  rollback_required=true
  build_staged_plans
  publish_staged_plans
  build_commands
  copy_and_verify_commands

  for _temporary in "${run_temp_files[@]}"; do
    [ -n "$_temporary" ] && rm -f "$_temporary" || :
  done
  run_temp_files=()
  for _stage in "${stage_files[@]}"; do
    rm -f "$_stage" || die "cannot remove staged plan: $_stage"
  done
  stage_files=()
  remove_run_state || die 'cannot remove completed temporary run state'

  _index=0
  while [ "$_index" -lt "${#commands[@]}" ]; do
    printf 'plan relative=%s owner=%s command=%s\n' \
      "$(shell_quote "${plan_relpaths[$_index]}")" \
      "$(shell_quote "${plan_owners[$_index]}")" \
      "${commands[$_index]}" || die 'cannot emit finalized plan record'
    _index=$((_index + 1))
  done

  rollback_required=false
  trap - EXIT INT TERM
}

cancel() {
  [ "$#" -eq 1 ] || usage_error 'cancel requires exactly one run directory'
  validate_run_dir "$1" false
  trap cleanup_operation EXIT
  trap 'handle_signal INT' INT
  trap 'handle_signal TERM' TERM
  acquire_run_lock
  remove_run_state || die 'cannot remove temporary run state'
  trap - EXIT INT TERM
  printf 'cancelled %s\n' "$(shell_quote "$1")"
}

command_name=${1:-}
[ -n "$command_name" ] || usage_error 'a command is required'
shift

case $command_name in
  prepare) prepare "$@" ;;
  finalize) finalize "$@" ;;
  cancel) cancel "$@" ;;
  -h | --help)
    usage
    ;;
  *) usage_error "unknown command: $command_name" ;;
esac
