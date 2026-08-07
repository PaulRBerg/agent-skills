#!/bin/bash

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
helper=$repo_root/skills/copy-transcript-path/scripts/copy-transcript-path.sh
test_root=$(mktemp -d "${TMPDIR:-/tmp}/copy-transcript-path-tests.XXXXXX")
test_home=$test_root/home
clipboard_file=$test_root/clipboard
pbcopy_bin=$test_root/pbcopy
home_marker='~'

cleanup() {
  rm -rf "$test_root"
}
trap cleanup EXIT

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

assert_equal() {
  [ "$1" = "$2" ] || fail "$3: expected '$1', got '$2'"
}

assert_contains() {
  printf '%s\n' "$2" | grep -Fq -- "$1" || fail "$3: missing '$1'"
}

run_helper() {
  set +e
  helper_output=$(env -u CODEX_THREAD_ID -u CLAUDE_CODE_SESSION_ID \
    HOME="$test_home" \
    COPY_TRANSCRIPT_PATH_PBCOPY="$pbcopy_bin" \
    COPY_TRANSCRIPT_PATH_TEST_CLIPBOARD="$clipboard_file" \
    "$@" "$helper" 2>&1)
  helper_rc=$?
  set -e
}

mkdir -p "$test_home"
cat >"$pbcopy_bin" <<'EOF'
#!/bin/bash
if [ "${COPY_TRANSCRIPT_PATH_TEST_PBCOPY_FAIL:-}" = 1 ]; then
  exit 1
fi
cat >"${COPY_TRANSCRIPT_PATH_TEST_CLIPBOARD:?}"
EOF
chmod 755 "$pbcopy_bin"

codex_session_id=01234567-89ab-cdef-0123-456789abcdef
codex_transcript=$test_home/.codex/sessions/2026/08/07/rollout-2026-08-07T14-29-43-$codex_session_id.jsonl
mkdir -p "$(dirname "$codex_transcript")"
: >"$codex_transcript"
run_helper "CODEX_HOME=$test_home/.codex" "CODEX_THREAD_ID=$codex_session_id"
assert_equal 0 "$helper_rc" 'Codex copy exit status'
assert_equal "Copied: ~/.codex/sessions/2026/08/07/rollout-2026-08-07T14-29-43-$codex_session_id.jsonl" \
  "$helper_output" 'Codex output'
assert_equal "$home_marker/.codex/sessions/2026/08/07/rollout-2026-08-07T14-29-43-$codex_session_id.jsonl" \
  "$(cat "$clipboard_file")" 'Codex clipboard bytes'

claude_session_id=fedcba98-7654-3210-fedc-ba9876543210
claude_home=$test_home/claude-config
claude_transcript=$claude_home/projects/-tmp-project/$claude_session_id.jsonl
mkdir -p "$(dirname "$claude_transcript")"
: >"$claude_transcript"
run_helper "CLAUDE_CONFIG_DIR=$claude_home" "CLAUDE_CODE_SESSION_ID=$claude_session_id"
assert_equal 0 "$helper_rc" 'Claude Code copy exit status'
assert_equal "Copied: ~/claude-config/projects/-tmp-project/$claude_session_id.jsonl" "$helper_output" 'Claude Code output'
assert_equal "$home_marker/claude-config/projects/-tmp-project/$claude_session_id.jsonl" "$(cat "$clipboard_file")" \
  'Claude Code clipboard bytes'

printf 'unchanged' >"$clipboard_file"
run_helper
assert_equal 1 "$helper_rc" 'unsupported host exit status'
assert_contains 'This skill only works in Claude Code or Codex CLI.' "$helper_output" 'unsupported host error'
assert_equal unchanged "$(cat "$clipboard_file")" 'unsupported host preserves clipboard'

run_helper "CODEX_HOME=$test_home/.codex" "CODEX_THREAD_ID=$codex_session_id" \
  "CLAUDE_CONFIG_DIR=$claude_home" "CLAUDE_CODE_SESSION_ID=$claude_session_id"
assert_equal 1 "$helper_rc" 'ambiguous host exit status'
assert_contains 'current chat host is ambiguous' "$helper_output" 'ambiguous host error'

missing_session_id=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee
run_helper "CODEX_HOME=$test_home/.codex" "CODEX_THREAD_ID=$missing_session_id"
assert_equal 1 "$helper_rc" 'missing transcript exit status'
assert_contains 'current transcript was not found' "$helper_output" 'missing transcript error'

duplicate_transcript=$test_home/.codex/sessions/duplicate/rollout-duplicate-$codex_session_id.jsonl
mkdir -p "$(dirname "$duplicate_transcript")"
: >"$duplicate_transcript"
run_helper "CODEX_HOME=$test_home/.codex" "CODEX_THREAD_ID=$codex_session_id"
assert_equal 1 "$helper_rc" 'duplicate transcript exit status'
assert_contains 'found 2 current transcript files' "$helper_output" 'duplicate transcript error'
rm "$duplicate_transcript"

run_helper "CODEX_HOME=$test_home/.codex" "CODEX_THREAD_ID=$codex_session_id" \
  'COPY_TRANSCRIPT_PATH_TEST_PBCOPY_FAIL=1'
assert_equal 1 "$helper_rc" 'clipboard failure exit status'
assert_contains 'clipboard copy failed' "$helper_output" 'clipboard failure error'
