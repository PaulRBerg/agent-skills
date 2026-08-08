#!/usr/bin/env bash

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
helper=$repo_root/skills/task-handoff/scripts/task-handoff.sh
test_root=$(mktemp -d "${TMPDIR:-/tmp}/task-handoff-tests.XXXXXX")
test_root=$(cd "$test_root" && pwd -P)
fake_bin=$test_root/bin
runs_dir=$test_root/runs
desktop=$test_root/Desktop
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
  # shellcheck disable=SC2294
  eval "record_fields=( $_record_line )"
}

run_helper() {
  TASK_HANDOFF_TEST_PBCOPY=${TASK_HANDOFF_TEST_PBCOPY:-$fake_bin/pbcopy} \
    TASK_HANDOFF_TEST_PBPASTE=${TASK_HANDOFF_TEST_PBPASTE:-$fake_bin/pbpaste} \
    TASK_HANDOFF_TEST_TRASH=${TASK_HANDOFF_TEST_TRASH:-$fake_bin/trash} \
    TASK_HANDOFF_TEST_HOOK=${TASK_HANDOFF_TEST_HOOK:-$fake_bin/hook} \
    TASK_HANDOFF_TEST_TMPDIR=${TASK_HANDOFF_TEST_TMPDIR:-$runs_dir} \
    TASK_HANDOFF_TEST_DESKTOP=${TASK_HANDOFF_TEST_DESKTOP:-$desktop} \
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

write_draft() {
  _draft=$1
  _title=$2
  printf '# %s\n\nObjective, task details, and targeted validation are decision-complete.\n' \
    "$_title" >"$_draft"
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

mkdir -p "$fake_bin" "$runs_dir" "$desktop"

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
handoff_dir=$desktop/.ai/task-handoffs

# A one-repository handoff stays in its repository and does not require a Desktop.
single_task="fix Bob's parser"
single_category=investigation
single_prepare=$(TASK_HANDOFF_TEST_DESKTOP=$test_root/missing-desktop run_helper prepare \
  --repo "$repo_one/subdirectory" \
  --repo "$repo_alias" \
  --plan "$repo_alias" SINGLE_PLAN.md "$single_category" "$single_task")
assert_equal 1 "$(printf '%s\n' "$single_prepare" | grep -c '^repo ')" 'symlink root deduplication'
single_run=$(extract_run_dir "$single_prepare")
write_draft "$single_run/plans/0001/draft.md" 'Single plan'
single_result=$(run_helper finalize "$single_run")
single_target=$repo_one_root/.ai/task-handoffs/SINGLE_PLAN.md
assert_exists "$single_target" 'repository-local handoff publication'
assert_absent "$handoff_dir/SINGLE_PLAN.md" 'single-repository Desktop publication'
assert_absent "$single_run" 'successful finalize run cleanup'

single_prompt="A previous agent prepared a $single_category task handoff for $single_task under .ai/task-handoffs/SINGLE_PLAN.md. Read the handoff, then complete its requested $single_category task. Follow its stated outcome, boundaries, authority constraints, and validation requirements."
single_command="codex -C $(shell_quote "$repo_one_root") $(shell_quote "$single_prompt")"
single_claude_command="cd $(shell_quote "$repo_one_root") && claude $(shell_quote "$single_prompt")"
single_prefix="plan handoff=$(shell_quote "$single_target") launch_repo=$(shell_quote "$repo_one_root") category=$(shell_quote "$single_category") command="
case $single_result in
  "$single_prefix"*) ;;
  *) fail 'single finalize record omitted its repository target or launch repository' ;;
esac
single_record_commands=${single_result#"$single_prefix"}
single_record_command=${single_record_commands%% claude_command=*}
single_record_claude_command=${single_record_commands#* claude_command=}
assert_equal "$single_command" "$single_record_command" 'single exact command'
assert_equal "$single_claude_command" "$single_record_claude_command" 'single exact Claude command'
/bin/bash -n -c "$single_record_command" || fail 'single command is not shell-safe'
/bin/bash -n -c "$single_record_claude_command" || fail 'single Claude command is not shell-safe'
assert_equal "$single_command" "$(cat "$clipboard_file")" 'single clipboard bytes'

assert_equal 1 "$(grep -Fxc '## Handoff category' "$single_target")" 'handoff category count'
assert_equal 1 "$(grep -Fxc "Category: \`$single_category\`" "$single_target")" 'handoff category value'
assert_equal 1 "$(grep -Fxc '## Execution status' "$single_target")" 'execution status count'
assert_equal 1 "$(grep -Fxc '## Handoff cleanup' "$single_target")" 'handoff cleanup count'
single_quoted_target=$(shell_quote "$single_target")
assert_file_contains "Run \`/usr/bin/trash $single_quoted_target\` only after the requested work is complete and" \
  "$single_target" \
  'repository-local cleanup command'

# Cross-repository work still produces one Desktop handoff and launches in the stated first repository.
cross_prepare=$(run_helper prepare \
  --repo "$repo_one" \
  --repo "$repo_two" \
  --plan "$repo_two" CROSS_REPOSITORY.md implementation 'coordinate both repositories')
assert_equal 2 "$(printf '%s\n' "$cross_prepare" | grep -c '^repo ')" 'cross-repository roots'
cross_run=$(extract_run_dir "$cross_prepare")
write_draft "$cross_run/plans/0001/draft.md" 'Cross repository plan'
printf '\n## Repository order\n\n1. %s — tackle first and validate its changes before the handoff.\n2. %s — complete dependent work and validate it.\n' \
  "\`$repo_two_root\`" "\`$repo_one_root\`" >>"$cross_run/plans/0001/draft.md"
cross_result=$(run_helper finalize "$cross_run")
cross_target=$handoff_dir/CROSS_REPOSITORY.md
assert_exists "$cross_target" 'cross-repository Desktop publication'
assert_absent "$repo_one_root/.ai/task-handoffs/CROSS_REPOSITORY.md" 'first repository handoff publication'
assert_absent "$repo_two_root/.ai/task-handoffs/CROSS_REPOSITORY.md" 'second repository handoff publication'
cross_prefix="plan handoff=$(shell_quote "$cross_target") launch_repo=$(shell_quote "$repo_two_root") category=$(shell_quote implementation) command="
case $cross_result in
  "$cross_prefix"*) ;;
  *) fail 'cross-repository handoff did not use its first repository as launch directory' ;;
esac
cross_record_commands=${cross_result#"$cross_prefix"}
cross_record_command=${cross_record_commands%% claude_command=*}
cross_record_claude_command=${cross_record_commands#* claude_command=}
cross_prompt="A previous agent prepared a implementation task handoff for coordinate both repositories at $cross_target. Read the handoff, then complete its requested implementation task. Start in the selected first repository and follow its stated repository order, outcome, boundaries, authority constraints, and validation requirements."
assert_equal "codex -C $(shell_quote "$repo_two_root") $(shell_quote "$cross_prompt")" \
  "$cross_record_command" 'cross exact Codex command'
assert_equal "cd $(shell_quote "$repo_two_root") && claude $(shell_quote "$cross_prompt")" \
  "$cross_record_claude_command" 'cross exact Claude command'
assert_equal "$cross_record_command" "$(cat "$clipboard_file")" 'cross clipboard remains Codex-only'

# The helper rejects topology that could publish more than one handoff.
expect_failure 'prepare creates exactly one handoff plan' run_helper prepare \
  --repo "$repo_one" --repo "$repo_two" \
  --plan "$repo_one" FIRST.md implementation 'first' \
  --plan "$repo_two" SECOND.md implementation 'second'

unordered_prepare=$(run_helper prepare \
  --repo "$repo_one" --repo "$repo_two" \
  --plan "$repo_one" CROSS_ORDER_REQUIRED.md implementation 'order is required')
unordered_run=$(extract_run_dir "$unordered_prepare")
write_draft "$unordered_run/plans/0001/draft.md" 'Missing cross-repository order'
expect_failure 'cross-repository draft is missing a Repository order section' run_helper finalize "$unordered_run"
run_helper cancel "$unordered_run" >/dev/null

# Prepare validates the conditional target and launch repository before creating temporary state.
expect_failure 'invalid plan filename' run_helper prepare \
  --repo "$repo_one" --plan "$repo_one" invalid.md implementation 'invalid name'
expect_failure 'invalid task category' run_helper prepare \
  --repo "$repo_one" --plan "$repo_one" INVALID_CATEGORY.md unclassified 'invalid category'
nongit=$test_root/not-a-repository
mkdir -p "$nongit"
expect_failure 'launch repository is not a Git worktree' run_helper prepare \
  --repo "$repo_one" --plan "$nongit" NON_GIT.md research 'non-git launch repository'
expect_failure 'launch repository is not among the involved repositories' run_helper prepare \
  --repo "$repo_one" --plan "$repo_two" WRONG_LAUNCH.md audit 'wrong launch repository'
unignored_repo=$test_root/unignored-repo
make_repo "$unignored_repo" false
expect_failure 'plan target is not ignored' run_helper prepare \
  --repo "$unignored_repo" --plan "$unignored_repo" UNIGNORED.md operations 'unignored target'
mkdir -p "$repo_two_root/.ai/task-handoffs"
printf 'pre-existing\n' >"$repo_two_root/.ai/task-handoffs/EXISTING.md"
expect_failure 'plan target already exists' run_helper prepare \
  --repo "$repo_two" --plan "$repo_two" EXISTING.md operations 'existing target'
assert_equal 'pre-existing' "$(cat "$repo_two_root/.ai/task-handoffs/EXISTING.md")" \
  'existing repository target changed during prepare'

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

# A target race is not overwritten or removed because the helper did not create it.
race_prepare=$(run_helper prepare --repo "$repo_two" --plan "$repo_two" RACE_TARGET.md audit 'race target')
race_run=$(extract_run_dir "$race_prepare")
write_draft "$race_run/plans/0001/draft.md" 'Race target'
TASK_HANDOFF_TEST_HOOK_MODE=race expect_failure 'appeared during publication' run_helper finalize "$race_run"
race_target=$repo_two_root/.ai/task-handoffs/RACE_TARGET.md
assert_equal 'raced target' "$(cat "$race_target")" 'raced target was overwritten or removed'
run_helper cancel "$race_run" >/dev/null

# A failed clipboard copy rolls back the repository-local handoff and the directories it created.
clipboard_repo=$test_root/clipboard-failure-repo
make_repo "$clipboard_repo"
clipboard_root=$(git -C "$clipboard_repo" rev-parse --show-toplevel)
clipboard_prepare=$(run_helper prepare \
  --repo "$clipboard_repo" --plan "$clipboard_repo" CLIPBOARD.md implementation 'clipboard failure')
clipboard_run=$(extract_run_dir "$clipboard_prepare")
write_draft "$clipboard_run/plans/0001/draft.md" 'Clipboard failure'
TASK_HANDOFF_TEST_CLIPBOARD_FAIL=copy \
  expect_failure 'clipboard copy failed' run_helper finalize "$clipboard_run"
assert_absent "$clipboard_root/.ai/task-handoffs/CLIPBOARD.md" 'clipboard rollback target'
assert_absent "$clipboard_root/.ai" 'now-empty repository rollback directories'
run_helper cancel "$clipboard_run" >/dev/null

# Noninteractive findings publication stays in its repository without clipboard tools.
finding_prepare=$(run_helper prepare \
  --repo "$repo_one" \
  --plan "$repo_one" FINDING_DEADBEEF.md audit 'triage finding deadbeef')
finding_run=$(extract_run_dir "$finding_prepare")
printf '# Finding\n\nSource finding: deadbeef\n\nTriage and validate the evidence.\n' \
  >"$finding_run/plans/0001/draft.md"
missing_pbcopy=$fake_bin/missing-pbcopy
missing_pbpaste=$fake_bin/missing-pbpaste
finding_result=$(TASK_HANDOFF_TEST_PBCOPY=$missing_pbcopy TASK_HANDOFF_TEST_PBPASTE=$missing_pbpaste \
  run_helper finalize --no-clipboard "$finding_run")
finding_target=$repo_one_root/.ai/task-handoffs/FINDING_DEADBEEF.md
assert_exists "$finding_target" 'findings repository publication'
assert_file_contains 'Source finding: deadbeef' "$finding_target" 'finding provenance'
case $finding_result in
  "plan handoff="*" command="*" claude_command="*) ;;
  *) fail 'no-clipboard finalize omitted its plan record' ;;
esac

printf 'task-handoff tests passed\n'
