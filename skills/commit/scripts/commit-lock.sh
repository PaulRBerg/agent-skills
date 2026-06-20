#!/usr/bin/env bash

set -u

lock_sleep_seconds=15
lock_max_waits=10

usage() {
  printf 'Usage: bash <skill-dir>/scripts/commit-lock.sh acquire|release [lock_token]\n' >&2
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

hash_stdin() {
  if command -v shasum >/dev/null 2>&1; then
    _hash_output=$(shasum -a 256) || return 1
  elif command -v sha256sum >/dev/null 2>&1; then
    _hash_output=$(sha256sum) || return 1
  else
    die 'missing shasum or sha256sum'
  fi

  _hash_output=${_hash_output%% *}
  printf '%s\n' "$_hash_output"
}

resolve_repo_context() {
  inside_work_tree=$(git rev-parse --is-inside-work-tree 2>/dev/null) || die 'not inside a git work tree'
  [ "$inside_work_tree" = true ] || die 'not inside a git work tree'

  repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || die 'cannot resolve git repository root'
  repo_root=$(physical_dir "$repo_root") || die 'cannot resolve git repository root'

  branch=$(git symbolic-ref --quiet --short HEAD 2>/dev/null) || die 'detached HEAD; check out a branch before committing'

  git_dir=$(git rev-parse --absolute-git-dir 2>/dev/null) || die 'cannot resolve git directory'
  git_dir=$(physical_dir "$git_dir") || die 'cannot resolve git directory'
  lock_root="$git_dir/agent-skills/commit-locks"
}

resolve_lock_root() {
  inside_work_tree=$(git rev-parse --is-inside-work-tree 2>/dev/null) || die 'not inside a git work tree'
  [ "$inside_work_tree" = true ] || die 'not inside a git work tree'

  git_dir=$(git rev-parse --absolute-git-dir 2>/dev/null) || die 'cannot resolve git directory'
  git_dir=$(physical_dir "$git_dir") || die 'cannot resolve git directory'
  lock_root="$git_dir/agent-skills/commit-locks"
}

pid_is_running() {
  _pid=$1
  case "$_pid" in
    '' | *[!0123456789]*)
      return 1
      ;;
  esac
  kill -0 "$_pid" 2>/dev/null
}

metadata_value() {
  _metadata_file=$1
  _key=$2
  while IFS= read -r _line; do
    case "$_line" in
      "$_key="*)
        printf '%s\n' "${_line#*=}"
        return 0
        ;;
    esac
  done <"$_metadata_file"
  return 1
}

remove_stale_lock() {
  _lock_dir=$1
  _metadata_file="$_lock_dir/metadata"

  if [ ! -f "$_metadata_file" ]; then
    rmdir "$_lock_dir" 2>/dev/null
    return $?
  fi

  _holder_pid=$(metadata_value "$_metadata_file" "pid" 2>/dev/null) || return 1
  if pid_is_running "$_holder_pid"; then
    return 1
  fi

  rm -f "$_metadata_file" 2>/dev/null || return 1
  rmdir "$_lock_dir" 2>/dev/null
}

acquire_lock() {
  [ "$#" -eq 0 ] || {
    usage
    die "unexpected argument: $1"
  }

  resolve_repo_context

  lock_hash=$(printf '%s\n%s\n' "$repo_root" "$branch" | hash_stdin) || die 'failed to hash commit lock key'
  owner=$(printf '%s\n%s\n%s\n%s\n%s\n%s\n' "$repo_root" "$branch" "$$" "$(date +%s)" "$RANDOM" "$RANDOM" | hash_stdin) || die 'failed to create commit lock owner'
  lock_holder_pid=$PPID
  lock_token="$lock_hash:$owner"
  lock_dir="$lock_root/$lock_hash.lock"

  mkdir -p "$lock_root" || die 'failed to create commit lock directory'

  waits=0
  waiting_reported=false
  while ! mkdir "$lock_dir" 2>/dev/null; do
    if remove_stale_lock "$lock_dir"; then
      continue
    fi
    if [ "$waits" -ge "$lock_max_waits" ]; then
      die 'another agent is already committing for this repo and branch'
    fi
    if [ "$waiting_reported" = false ]; then
      printf 'waiting for another commit attempt to finish...\n' >&2
      waiting_reported=true
    fi
    sleep "$lock_sleep_seconds"
    waits=$((waits + 1))
  done

  if ! {
    printf 'owner=%s\n' "$owner"
    printf 'repo=%s\n' "$repo_root"
    printf 'branch=%s\n' "$branch"
    printf 'pid=%s\n' "$lock_holder_pid"
    printf 'helper_pid=%s\n' "$$"
    date -u '+created_at=%Y-%m-%dT%H:%M:%SZ'
  } >"$lock_dir/metadata"; then
    rm -f "$lock_dir/metadata"
    rmdir "$lock_dir" 2>/dev/null
    die 'failed to write commit lock metadata'
  fi

  printf '%s\n' "$lock_token"
}

release_lock() {
  [ "$#" -eq 1 ] || {
    usage
    die 'release requires a lock token'
  }

  lock_token=$1
  case "$lock_token" in
    *:*) ;;
    *)
      die 'invalid lock token'
      ;;
  esac

  lock_hash=${lock_token%%:*}
  owner=${lock_token#*:}
  case "$lock_hash" in
    '' | *[!0123456789abcdef]*)
      die 'invalid lock token'
      ;;
  esac
  case "$owner" in
    '' | *[!0123456789abcdef]*)
      die 'invalid lock token'
      ;;
  esac

  resolve_lock_root

  lock_dir="$lock_root/$lock_hash.lock"
  [ -d "$lock_dir" ] || exit 0

  metadata_file="$lock_dir/metadata"
  [ -f "$metadata_file" ] || die 'commit lock metadata is missing'

  IFS= read -r owner_line <"$metadata_file" || die 'failed to read commit lock metadata'
  case "$owner_line" in
    owner=*) stored_owner=${owner_line#owner=} ;;
    *) die 'commit lock metadata is invalid' ;;
  esac

  [ "$stored_owner" = "$owner" ] || die 'commit lock is owned by another process'

  rm -f "$metadata_file" || die 'failed to remove commit lock metadata'
  rmdir "$lock_dir" || die 'failed to remove commit lock directory'
}

case "${1:-}" in
  acquire)
    shift
    acquire_lock "$@"
    ;;
  release)
    shift
    release_lock "$@"
    ;;
  *)
    usage
    die 'expected acquire or release'
    ;;
esac
