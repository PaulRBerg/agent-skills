#!/bin/bash

set -euo pipefail

unsupported_host_error='This skill only works in Claude Code or Codex CLI.'

die() {
  printf 'copy-transcript-path: %s\n' "$*" >&2
  exit 1
}

is_uuid() {
  [[ "$1" =~ ^[[:xdigit:]]{8}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{12}$ ]]
}

resolve_single_transcript() {
  local search_root=$1
  local file_name=$2
  local candidate

  [ -d "$search_root" ] || die "transcript storage is unavailable: $search_root"

  transcript_paths=()
  while IFS= read -r -d '' candidate; do
    transcript_paths+=("$candidate")
  done < <(find "$search_root" -type f -name "$file_name" -print0)

  case ${#transcript_paths[@]} in
    0) die 'current transcript was not found' ;;
    1) ;;
    *) die "found ${#transcript_paths[@]} current transcript files" ;;
  esac
}

codex_session_id=${CODEX_THREAD_ID:-}
claude_session_id=${CLAUDE_CODE_SESSION_ID:-}

if [ -n "$codex_session_id" ] && [ -n "$claude_session_id" ]; then
  die 'current chat host is ambiguous'
fi

if [ -n "$codex_session_id" ]; then
  is_uuid "$codex_session_id" || die 'invalid Codex session identifier'
  codex_home=${CODEX_HOME:-"$HOME/.codex"}
  resolve_single_transcript "$codex_home/sessions" "rollout-*-$codex_session_id.jsonl"
elif [ -n "$claude_session_id" ]; then
  is_uuid "$claude_session_id" || die 'invalid Claude Code session identifier'
  claude_home=${CLAUDE_CONFIG_DIR:-${CLAUDE_HOME:-"$HOME/.claude"}}
  resolve_single_transcript "$claude_home/projects" "$claude_session_id.jsonl"
else
  die "$unsupported_host_error"
fi

transcript_dir=$(cd "$(dirname "${transcript_paths[0]}")" && pwd -P) || die 'cannot resolve transcript directory'
transcript_path=$transcript_dir/$(basename "${transcript_paths[0]}")
home_dir=$(cd "$HOME" && pwd -P) || die 'cannot resolve home directory'

case $transcript_path in
  "$home_dir"/*) display_path="~${transcript_path#"$home_dir"}" ;;
  *) display_path=$transcript_path ;;
esac

pbcopy_bin=${COPY_TRANSCRIPT_PATH_PBCOPY:-/usr/bin/pbcopy}
[ -x "$pbcopy_bin" ] || die "required executable is unavailable: $pbcopy_bin"
printf '%s' "$display_path" | "$pbcopy_bin" || die 'clipboard copy failed'
printf 'Copied: %s\n' "$display_path"
