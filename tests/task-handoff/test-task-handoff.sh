#!/usr/bin/env bash

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
script_dir=$repo_root/skills/task-handoff/scripts
helper=$script_dir/task-handoff.sh
test_root=$(mktemp -d "${TMPDIR:-/tmp}/task-handoff-tests.XXXXXX")
fake_bin=$test_root/bin
runs_dir=$test_root/runs
clipboard_file=$test_root/clipboard
record_fields=()

cleanup() {
  rm -rf "$test_root"
}
trap cleanup EXIT

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

assert_equal() {
  _expected=$1
  _actual=$2
  _label=$3
  if [ "$_actual" != "$_expected" ]; then
    printf 'FAIL: %s\nexpected:\n%s\nactual:\n%s\n' "$_label" "$_expected" "$_actual" >&2
    exit 1
  fi
}

assert_exists() {
  [ -e "$1" ] || [ -L "$1" ] || fail "$2: missing $1"
}

assert_absent() {
  if [ -e "$1" ] || [ -L "$1" ]; then
    fail "$2: unexpectedly exists: $1"
  fi
}

assert_file_contains() {
  grep -Fq -- "$1" "$2" || fail "$3: missing $1"
}

assert_file_not_contains() {
  if grep -Fq -- "$1" "$2"; then
    fail "$3: unexpectedly found $1"
  fi
}

expect_failure() {
  _expected_text=$1
  shift
  set +e
  _failure_output=$("$@" 2>&1)
  _failure_rc=$?
  set -e
  [ "$_failure_rc" -ne 0 ] || fail "expected failure containing: $_expected_text"
  printf '%s\n' "$_failure_output" | grep -Fq -- "$_expected_text" ||
    fail "failure did not contain: $_expected_text"
}

shell_quote() {
  printf "'"
  printf '%s' "$1" | sed "s/'/'\\\\''/g"
  printf "'"
}

parse_record() {
  _record_line=$1
  # The helper produced this shell-quoted record; decoding it verifies that its
  # fields round-trip paths and commands containing spaces and single quotes.
  # shellcheck disable=SC2294
  eval "record_fields=( $_record_line )"
}

run_helper() {
  TASK_HANDOFF_TEST_PBCOPY=${TASK_HANDOFF_TEST_PBCOPY:-$fake_bin/pbcopy} \
    TASK_HANDOFF_TEST_PBPASTE=${TASK_HANDOFF_TEST_PBPASTE:-$fake_bin/pbpaste} \
    TASK_HANDOFF_TEST_TRASH=${TASK_HANDOFF_TEST_TRASH:-$fake_bin/trash} \
    TASK_HANDOFF_TEST_HOOK=${TASK_HANDOFF_TEST_HOOK:-$fake_bin/hook} \
    TASK_HANDOFF_TEST_TMPDIR=${TASK_HANDOFF_TEST_TMPDIR:-$runs_dir} \
    TASK_HANDOFF_TEST_CLIPBOARD=${TASK_HANDOFF_TEST_CLIPBOARD:-$clipboard_file} \
    TASK_HANDOFF_TEST_CLIPBOARD_FAIL=${TASK_HANDOFF_TEST_CLIPBOARD_FAIL:-} \
    TASK_HANDOFF_TEST_HOOK_MODE=${TASK_HANDOFF_TEST_HOOK_MODE:-} \
    /bin/bash "$helper" "$@"
}

extract_run_dir() {
  _prepare_output=$1
  _run_record=$(printf '%s\n' "$_prepare_output" | sed -n '1p')
  parse_record "$_run_record"
  [ "${record_fields[0]}" = run_dir ] || fail 'prepare output did not start with run_dir record'
  printf '%s' "${record_fields[1]}"
}

make_repo() {
  _repo=$1
  _ignored=${2:-true}
  mkdir -p "$_repo"
  git -C "$_repo" init --quiet
  git -C "$_repo" config core.excludesFile /dev/null
  if [ "$_ignored" = true ]; then
    printf '.ai/task-handoffs/\n' >"$_repo/.gitignore"
  fi
}

write_draft() {
  _draft=$1
  _title=$2
  printf '# %s\n\nObjective, task details, and targeted validation are decision-complete.\n' \
    "$_title" >"$_draft"
}

mkdir -p "$fake_bin" "$runs_dir"

cat >"$fake_bin/pbcopy" <<'EOF'
#!/usr/bin/env bash
set -eu
[ "${TASK_HANDOFF_TEST_CLIPBOARD_FAIL:-}" != copy ] || exit 9
cat >"$TASK_HANDOFF_TEST_CLIPBOARD"
EOF

cat >"$fake_bin/pbpaste" <<'EOF'
#!/usr/bin/env bash
set -eu
if [ "${TASK_HANDOFF_TEST_CLIPBOARD_FAIL:-}" = mismatch ]; then
  printf 'mismatched clipboard bytes'
else
  cat "$TASK_HANDOFF_TEST_CLIPBOARD"
fi
EOF

cat >"$fake_bin/trash" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF

cat >"$fake_bin/hook" <<'EOF'
#!/usr/bin/env bash
set -eu
event=$1
target=$3
case "${TASK_HANDOFF_TEST_HOOK_MODE:-}:$event" in
  race:before_publish)
    printf 'raced target\n' >"$target"
    ;;
  terminate:after_publish)
    kill -TERM "$PPID"
    ;;
esac
EOF
chmod 755 "$fake_bin/pbcopy" "$fake_bin/pbpaste" "$fake_bin/trash" "$fake_bin/hook"

repo_one=$test_root/repo\ one\ with\ \'quote
repo_two=$test_root/repo\ two\ with\ spaces
repo_alias=$test_root/repo-one-alias
mkdir -p "$repo_one/subdirectory"
make_repo "$repo_one"
make_repo "$repo_two"
ln -s "$repo_one" "$repo_alias"
repo_one_root=$(git -C "$repo_one" rev-parse --show-toplevel)
repo_two_root=$(git -C "$repo_two" rev-parse --show-toplevel)

# A single-repository run deduplicates a symlink alias and preserves quoting.
single_task="fix Bob's parser"
single_category=investigation
single_prepare=$(run_helper prepare \
  --repo "$repo_one/subdirectory" \
  --repo "$repo_alias" \
  --plan "$repo_alias" SINGLE_PLAN.md "$single_category" "$single_task")
assert_equal 1 "$(printf '%s\n' "$single_prepare" | grep -c '^repo ')" 'symlink root deduplication'
single_run=$(extract_run_dir "$single_prepare")
write_draft "$single_run/plans/0001/draft.md" 'Single plan'
single_result=$(run_helper finalize "$single_run")
single_target=$repo_one_root/.ai/task-handoffs/SINGLE_PLAN.md
assert_exists "$single_target" 'single plan publication'
assert_absent "$single_run" 'successful finalize run cleanup'

single_prompt="A previous agent prepared a $single_category task handoff for $single_task under .ai/task-handoffs/SINGLE_PLAN.md. Read the handoff, then complete its requested $single_category task. Follow its stated outcome, boundaries, authority constraints, and validation requirements."
single_command="codex -C $(shell_quote "$repo_one_root") $(shell_quote "$single_prompt")"
single_prefix="plan relative=$(shell_quote '.ai/task-handoffs/SINGLE_PLAN.md') owner=$(shell_quote "$repo_one_root") category=$(shell_quote "$single_category") command="
case $single_result in
  "$single_prefix"*) ;;
  *) fail 'single finalize record omitted its relative path or canonical owner' ;;
esac
single_record_command=${single_result#"$single_prefix"}
assert_equal "$single_command" "$single_record_command" 'single exact command'
/bin/bash -n -c "$single_record_command" || fail 'single command is not shell-safe'
assert_equal "$single_command" "$(cat "$clipboard_file")" 'single clipboard bytes'
[ "$(tail -c 1 "$clipboard_file" | od -An -tx1 | tr -d '[:space:]')" != 0a ] ||
  fail 'single clipboard payload has a trailing newline'

assert_equal 1 "$(grep -Fxc '## Handoff category' "$single_target")" 'handoff category count'
assert_equal 1 "$(grep -Fxc "Category: \`$single_category\`" "$single_target")" 'handoff category value'
assert_equal 1 "$(grep -Fxc '## Execution status' "$single_target")" 'execution status count'
assert_equal 1 "$(grep -Fxc '## Handoff cleanup' "$single_target")" 'handoff cleanup count'
status_line=$(grep -nFx '## Execution status' "$single_target" | cut -d: -f1)
cleanup_line=$(grep -nFx '## Handoff cleanup' "$single_target" | cut -d: -f1)
[ "$status_line" -lt "$cleanup_line" ] || fail 'execution status does not precede handoff cleanup'
between_headings=$(sed -n "${status_line},${cleanup_line}p" "$single_target")
expected_between=$'## Execution status\n\nCurrent status: No task attempt has been recorded.\n\nIf work stops before successful completion, replace the current status—not append an attempt history—with a concise\nrecord of completed work, remaining work, validation commands and outcomes, the blocker, and the next concrete\naction.\n\n## Handoff cleanup'
assert_equal "$expected_between" "$between_headings" 'exact execution status placement'
single_quoted_target=$(shell_quote "$single_target")
assert_file_contains "Run \`/usr/bin/trash $single_quoted_target\` only after the requested work is complete and" \
  "$single_target" \
  'canonical cleanup command'
assert_file_contains 'task-scoped validation passes.' "$single_target" 'task-scoped cleanup threshold'
assert_file_contains 'A broader required check may remain non-green only when evidence attributes every failure' \
  "$single_target" 'unrelated broader-check exception'
assert_file_contains 'Record each non-green command, its outcome, and that attribution in the final report,' \
  "$single_target" 'non-green check disclosure requirement'
assert_file_contains 'or any broader failure may have been caused by this task.' "$single_target" \
  'ambiguous broader failure retains handoff'
assert_file_not_contains 'every required validation passes' "$single_target" \
  'obsolete all-green cleanup threshold'

# Version 1 runs remain finalizable with an implementation-category default.
legacy_prepare=$(run_helper prepare --repo "$repo_one" --plan "$repo_one" LEGACY_RUN.md research 'legacy task')
legacy_run=$(extract_run_dir "$legacy_prepare")
legacy_marker=$legacy_run/.task-handoff-run
legacy_marker_tail=$(sed -n '2,4p' "$legacy_marker")
printf 'task-handoff-run-v1\n%s\n' "$legacy_marker_tail" >"$legacy_marker"
rm "$legacy_run/plans/0001/category"
write_draft "$legacy_run/plans/0001/draft.md" 'Legacy run'
legacy_result=$(run_helper finalize "$legacy_run")
legacy_prefix="plan relative=$(shell_quote '.ai/task-handoffs/LEGACY_RUN.md') owner=$(shell_quote "$repo_one_root") category=$(shell_quote implementation) command="
case $legacy_result in
  "$legacy_prefix"*) ;;
  *) fail 'legacy run did not default to the implementation category' ;;
esac

# Automated findings triage uppercases only the filename ID and can retry without clipboard tools.
finding_prepare=$(run_helper prepare \
  --repo "$repo_one" \
  --plan "$repo_one" FINDING_DEADBEEF.md audit 'triage finding deadbeef')
finding_run=$(extract_run_dir "$finding_prepare")
printf '# RouteMesh finding\n\n## Handoff cleanup\n' >"$finding_run/plans/0001/draft.md"
missing_pbcopy=$fake_bin/missing-pbcopy
missing_pbpaste=$fake_bin/missing-pbpaste
TASK_HANDOFF_TEST_PBCOPY=$missing_pbcopy TASK_HANDOFF_TEST_PBPASTE=$missing_pbpaste \
  expect_failure 'reserved heading' run_helper finalize --no-clipboard "$finding_run"
finding_target=$repo_one_root/.ai/task-handoffs/FINDING_DEADBEEF.md
assert_absent "$finding_target" 'failed findings triage publication'
printf '# Finding\n\nSource finding: deadbeef\n\nTriage and validate the evidence.\n' \
  >"$finding_run/plans/0001/draft.md"
finding_result=$(TASK_HANDOFF_TEST_PBCOPY=$missing_pbcopy TASK_HANDOFF_TEST_PBPASTE=$missing_pbpaste \
  run_helper finalize --no-clipboard "$finding_run")
finding_prefix="plan relative=$(shell_quote '.ai/task-handoffs/FINDING_DEADBEEF.md') owner=$(shell_quote "$repo_one_root") category=$(shell_quote audit) command="
case $finding_result in
  "$finding_prefix"*) ;;
  *) fail 'no-clipboard findings finalize omitted its plan record' ;;
esac
assert_exists "$finding_target" 'findings triage publication'
assert_file_contains 'Source finding: deadbeef' "$finding_target" 'finding provenance'
assert_absent "$finding_run" 'successful no-clipboard finalize run cleanup'

# A multi-repository run preserves plan order and exact newline-delimited clipboard bytes.
multi_prepare=$(run_helper prepare \
  --repo "$repo_one" \
  --repo "$repo_two" \
  --plan "$repo_one" FIRST_WORK.md research 'first task' \
  --plan "$repo_two" SECOND_WORK.md audit 'second task')
multi_run=$(extract_run_dir "$multi_prepare")
write_draft "$multi_run/plans/0001/draft.md" 'First work'
write_draft "$multi_run/plans/0002/draft.md" 'Second work'
multi_result=$(run_helper finalize "$multi_run")
first_line=$(printf '%s\n' "$multi_result" | sed -n '1p')
second_line=$(printf '%s\n' "$multi_result" | sed -n '2p')
first_prefix="plan relative=$(shell_quote '.ai/task-handoffs/FIRST_WORK.md') owner=$(shell_quote "$repo_one_root") category=$(shell_quote research) command="
second_prefix="plan relative=$(shell_quote '.ai/task-handoffs/SECOND_WORK.md') owner=$(shell_quote "$repo_two_root") category=$(shell_quote audit) command="
case $first_line in "$first_prefix"*) ;; *) fail 'first plan order or owner changed' ;; esac
case $second_line in "$second_prefix"*) ;; *) fail 'second plan order or owner changed' ;; esac
first_command=${first_line#"$first_prefix"}
second_command=${second_line#"$second_prefix"}
research_prompt='A previous agent prepared a research task handoff for first task under .ai/task-handoffs/FIRST_WORK.md. Read the handoff, then complete its requested research task. Follow its stated outcome, boundaries, authority constraints, and validation requirements.'
assert_equal "codex -C $(shell_quote "$repo_one_root") $(shell_quote "$research_prompt")" "$first_command" \
  'research exact command'
expected_clipboard=$first_command$'\n'$second_command
assert_equal "$expected_clipboard" "$(cat "$clipboard_file")" 'multi-command clipboard bytes'
[ "$(tail -c 1 "$clipboard_file" | od -An -tx1 | tr -d '[:space:]')" != 0a ] ||
  fail 'multi-command clipboard payload has a trailing newline'

# Prepare rejects invalid topology and targets before creating temporary state.
expect_failure 'invalid plan filename' run_helper prepare \
  --repo "$repo_one" --plan "$repo_one" invalid.md implementation 'invalid name'
expect_failure 'duplicate plan filename' run_helper prepare \
  --repo "$repo_one" --repo "$repo_two" \
  --plan "$repo_one" DUPLICATE.md implementation 'first duplicate' \
  --plan "$repo_two" DUPLICATE.md research 'second duplicate'
expect_failure 'invalid task category' run_helper prepare \
  --repo "$repo_one" --plan "$repo_one" INVALID_CATEGORY.md unclassified 'invalid category'
nongit=$test_root/not-a-repository
mkdir -p "$nongit"
expect_failure 'plan owner is not a Git worktree' run_helper prepare \
  --repo "$repo_one" --plan "$nongit" NON_GIT.md research 'non-git owner'
expect_failure 'plan owner is not among the involved repositories' run_helper prepare \
  --repo "$repo_one" --plan "$repo_two" WRONG_OWNER.md audit 'wrong owner'
unignored_repo=$test_root/unignored-repo
make_repo "$unignored_repo" false
expect_failure 'plan target is not ignored' run_helper prepare \
  --repo "$unignored_repo" --plan "$unignored_repo" UNIGNORED.md operations 'unignored target'
mkdir -p "$repo_two_root/.ai/task-handoffs"
printf 'pre-existing\n' >"$repo_two_root/.ai/task-handoffs/EXISTING.md"
expect_failure 'plan target already exists' run_helper prepare \
  --repo "$repo_two" --plan "$repo_two" EXISTING.md operations 'existing target'
assert_equal 'pre-existing' "$(cat "$repo_two_root/.ai/task-handoffs/EXISTING.md")" \
  'existing target changed during prepare'

# Empty and reserved-heading drafts fail without publishing and remain cancellable.
empty_prepare=$(run_helper prepare --repo "$repo_two" --plan "$repo_two" EMPTY_DRAFT.md research 'empty draft')
empty_run=$(extract_run_dir "$empty_prepare")
expect_failure 'plan draft is empty' run_helper finalize "$empty_run"
assert_absent "$repo_two_root/.ai/task-handoffs/EMPTY_DRAFT.md" 'empty draft target'
run_helper cancel "$empty_run" >/dev/null
assert_absent "$empty_run" 'empty draft cancellation'

reserved_prepare=$(run_helper prepare --repo "$repo_two" --plan "$repo_two" RESERVED.md audit 'reserved heading')
reserved_run=$(extract_run_dir "$reserved_prepare")
printf '# Body\n\n## Execution status\n' >"$reserved_run/plans/0001/draft.md"
expect_failure 'reserved heading' run_helper finalize "$reserved_run"
assert_absent "$repo_two_root/.ai/task-handoffs/RESERVED.md" 'reserved draft target'
run_helper cancel "$reserved_run" >/dev/null

# Cancellation validates helper state and never touches a target that appeared later.
cancel_prepare=$(run_helper prepare --repo "$repo_two" --plan "$repo_two" CANCEL_SAFE.md implementation 'cancel safely')
cancel_run=$(extract_run_dir "$cancel_prepare")
cancel_target=$repo_two_root/.ai/task-handoffs/CANCEL_SAFE.md
printf 'appeared after prepare\n' >"$cancel_target"
run_helper cancel "$cancel_run" >/dev/null
assert_absent "$cancel_run" 'cancel run cleanup'
assert_equal 'appeared after prepare' "$(cat "$cancel_target")" 'cancel touched plan target'
expect_failure 'not a task-handoff run directory' run_helper cancel "$repo_two_root"

# A target race is not overwritten or rolled back because the helper did not create it.
race_prepare=$(run_helper prepare --repo "$repo_two" --plan "$repo_two" RACE_TARGET.md audit 'race target')
race_run=$(extract_run_dir "$race_prepare")
write_draft "$race_run/plans/0001/draft.md" 'Race target'
TASK_HANDOFF_TEST_HOOK_MODE=race expect_failure 'appeared during publication' run_helper finalize "$race_run"
race_target=$repo_two_root/.ai/task-handoffs/RACE_TARGET.md
assert_equal 'raced target' "$(cat "$race_target")" 'raced target was overwritten or removed'
run_helper cancel "$race_run" >/dev/null

# Clipboard failure rolls back every plan and only the now-empty directories it created.
clipboard_repo=$test_root/clipboard-failure-repo
make_repo "$clipboard_repo"
clipboard_root=$(git -C "$clipboard_repo" rev-parse --show-toplevel)
printf 'keep me\n' >"$clipboard_root/PREEXISTING.txt"
clipboard_prepare=$(run_helper prepare \
  --repo "$clipboard_repo" \
  --plan "$clipboard_repo" CLIPBOARD_ONE.md implementation 'clipboard one' \
  --plan "$clipboard_repo" CLIPBOARD_TWO.md research 'clipboard two')
clipboard_run=$(extract_run_dir "$clipboard_prepare")
write_draft "$clipboard_run/plans/0001/draft.md" 'Clipboard one'
write_draft "$clipboard_run/plans/0002/draft.md" 'Clipboard two'
TASK_HANDOFF_TEST_CLIPBOARD_FAIL=copy expect_failure 'clipboard copy failed' run_helper finalize "$clipboard_run"
assert_absent "$clipboard_root/.ai/task-handoffs/CLIPBOARD_ONE.md" 'first clipboard rollback target'
assert_absent "$clipboard_root/.ai/task-handoffs/CLIPBOARD_TWO.md" 'second clipboard rollback target'
assert_absent "$clipboard_root/.ai" 'now-empty clipboard rollback directories'
assert_equal 'keep me' "$(cat "$clipboard_root/PREEXISTING.txt")" 'pre-existing root file changed'
run_helper cancel "$clipboard_run" >/dev/null

# TERM after the first publication rolls back helper targets but preserves prior files.
term_repo=$test_root/termination-repo
make_repo "$term_repo"
term_root=$(git -C "$term_repo" rev-parse --show-toplevel)
mkdir -p "$term_root/.ai/task-handoffs"
printf 'keep me too\n' >"$term_root/.ai/task-handoffs/KEEP.md"
term_prepare=$(run_helper prepare \
  --repo "$term_repo" \
  --plan "$term_repo" TERM_ONE.md operations 'term one' \
  --plan "$term_repo" TERM_TWO.md audit 'term two')
term_run=$(extract_run_dir "$term_prepare")
write_draft "$term_run/plans/0001/draft.md" 'Term one'
write_draft "$term_run/plans/0002/draft.md" 'Term two'
set +e
TASK_HANDOFF_TEST_HOOK_MODE=terminate run_helper finalize "$term_run" >/dev/null 2>&1
term_rc=$?
set -e
assert_equal 143 "$term_rc" 'TERM exit status'
assert_absent "$term_root/.ai/task-handoffs/TERM_ONE.md" 'first termination rollback target'
assert_absent "$term_root/.ai/task-handoffs/TERM_TWO.md" 'second termination rollback target'
assert_equal 'keep me too' "$(cat "$term_root/.ai/task-handoffs/KEEP.md")" 'termination removed pre-existing file'
run_helper cancel "$term_run" >/dev/null

# Readback mismatches use the same all-or-nothing rollback path.
mismatch_prepare=$(run_helper prepare --repo "$repo_two" --plan "$repo_two" MISMATCH.md implementation 'mismatch')
mismatch_run=$(extract_run_dir "$mismatch_prepare")
write_draft "$mismatch_run/plans/0001/draft.md" 'Mismatch'
TASK_HANDOFF_TEST_CLIPBOARD_FAIL=mismatch expect_failure 'clipboard verification failed' \
  run_helper finalize "$mismatch_run"
assert_absent "$repo_two_root/.ai/task-handoffs/MISMATCH.md" 'clipboard mismatch rollback target'
run_helper cancel "$mismatch_run" >/dev/null

printf 'task-handoff tests passed\n'
