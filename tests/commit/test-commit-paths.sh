#!/usr/bin/env bash

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
script_dir=$repo_root/skills/commit/scripts
helper=$script_dir/commit-paths.sh
test_root=$(mktemp -d "${TMPDIR:-/tmp}/commit-paths-tests.XXXXXX")

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

assert_contains_line() {
  _haystack=$1
  _needle=$2
  _label=$3
  printf '%s\n' "$_haystack" | rg -F -x -q -- "$_needle" || fail "$_label: missing $_needle"
}

init_fixture() {
  _repo=$1
  mkdir -p "$_repo"
  git -C "$_repo" init --quiet
  git -C "$_repo" config user.name 'Commit Paths Test'
  git -C "$_repo" config user.email 'commit-paths@example.com'
  git -C "$_repo" config commit.gpgsign false
  git -C "$_repo" config core.hooksPath .git/hooks
}

commit_fixture() {
  _repo=$1
  _message=$2
  git -C "$_repo" add -A
  git -C "$_repo" commit --quiet -m "$_message"
}

advance_head_with_content() {
  _repo=$1
  _content=$2
  _message=$3
  _old_head=$(git -C "$_repo" rev-parse HEAD)
  _blob=$(printf '%s' "$_content" | git -C "$_repo" hash-object -w --stdin)
  _tree=$(printf '100644 blob %s\tintended.txt\n' "$_blob" | git -C "$_repo" mktree)
  _commit=$(printf '%s\n' "$_message" | git -C "$_repo" commit-tree "$_tree" -p "$_old_head")
  git -C "$_repo" update-ref HEAD "$_commit" "$_old_head"
  printf '%s\n' "$_commit"
}

test_content_and_shared_index() {
  _repo=$test_root/content
  init_fixture "$_repo"

  printf 'base\n' > "$_repo/modified.txt"
  printf 'delete me\n' > "$_repo/deleted.txt"
  printf 'base\n' > "$_repo/unrelated.txt"
  commit_fixture "$_repo" base

  printf 'modified\n' > "$_repo/modified.txt"
  rm "$_repo/deleted.txt"
  printf 'added and staged\n' > "$_repo/added.txt"
  git -C "$_repo" add -- added.txt
  printf 'untracked\n' > "$_repo/untracked.txt"
  printf 'base\nstaged elsewhere\n' > "$_repo/unrelated.txt"
  git -C "$_repo" add -- unrelated.txt

  _unrelated_oid=$(git -C "$_repo" rev-parse :unrelated.txt)
  _index_before=$(git hash-object "$_repo/.git/index")
  _preview=$(
    cd "$_repo"
    bash "$helper" preview --diff summary -- modified.txt added.txt deleted.txt untracked.txt
  )
  _index_after=$(git hash-object "$_repo/.git/index")

  assert_equal "$_index_before" "$_index_after" 'preview changed the shared index'
  assert_contains_line "$_preview" '## commit paths' 'preview output'
  for _path in modified.txt added.txt deleted.txt untracked.txt; do
    assert_contains_line "$_preview" "$_path" 'resolved commit paths'
  done

  if _inherited_output=$(
    cd "$_repo"
    GIT_INDEX_FILE="$_repo/.git/index" bash "$helper" preview -- modified.txt 2>&1
  ); then
    fail 'inherited GIT_INDEX_FILE was accepted'
  fi
  printf '%s\n' "$_inherited_output" | rg -F -q 'GIT_INDEX_FILE is already set' ||
    fail 'inherited index rejection lacked a diagnostic'

  (
    cd "$_repo"
    bash "$helper" commit -m 'Commit intended files' -- modified.txt added.txt deleted.txt untracked.txt
  ) >/dev/null

  _committed_paths=$(git -C "$_repo" diff-tree --no-commit-id --name-only -r HEAD | LC_ALL=C sort)
  assert_equal $'added.txt\ndeleted.txt\nmodified.txt\nuntracked.txt' "$_committed_paths" 'committed path set'
  assert_equal 'modified' "$(git -C "$_repo" show HEAD:modified.txt)" 'modified file content'
  assert_equal 'added and staged' "$(git -C "$_repo" show HEAD:added.txt)" 'added file content'
  assert_equal 'untracked' "$(git -C "$_repo" show HEAD:untracked.txt)" 'untracked file content'
  if git -C "$_repo" cat-file -e HEAD:deleted.txt 2>/dev/null; then
    fail 'deleted path remains in commit'
  fi

  assert_equal "$_unrelated_oid" "$(git -C "$_repo" rev-parse :unrelated.txt)" 'unrelated staged blob changed'
  assert_equal 'unrelated.txt' "$(git -C "$_repo" diff --cached --name-only)" 'unrelated stage was not preserved alone'
  assert_equal '' "$(git -C "$_repo" diff --name-only)" 'worktree differs from reconciled shared index'
}

test_case_only_renames() {
  _repo=$test_root/case-renames
  init_fixture "$_repo"
  git -C "$_repo" config core.ignorecase true

  printf 'file\n' > "$_repo/case-file.txt"
  mkdir -p "$_repo/case-dir"
  printf 'nested\n' > "$_repo/case-dir/nested.txt"
  commit_fixture "$_repo" base

  mv "$_repo/case-file.txt" "$_repo/case-file.tmp"
  mv "$_repo/case-file.tmp" "$_repo/Case-File.txt"
  mv "$_repo/case-dir" "$_repo/case-dir.tmp"
  mv "$_repo/case-dir.tmp" "$_repo/Case-Dir"

  _preview=$(
    cd "$_repo"
    bash "$helper" preview --diff summary -- \
      case-file.txt Case-File.txt case-dir Case-Dir
  )
  for _path in case-file.txt Case-File.txt case-dir/nested.txt Case-Dir/nested.txt; do
    assert_contains_line "$_preview" "$_path" 'case-only resolved paths'
  done

  (
    cd "$_repo"
    bash "$helper" commit -m 'Rename paths by case' -- \
      case-file.txt Case-File.txt case-dir/nested.txt Case-Dir/nested.txt
  ) >/dev/null

  _tree=$(git -C "$_repo" ls-tree -r --name-only HEAD | LC_ALL=C sort)
  assert_equal $'Case-Dir/nested.txt\nCase-File.txt' "$_tree" 'case-only commit tree spelling'
  assert_equal '' "$(git -C "$_repo" status --short)" 'case-only rename left repository dirty'
}

test_formatter_hook() {
  _repo=$test_root/formatter
  init_fixture "$_repo"

  printf 'base\n' > "$_repo/intended.txt"
  printf 'base\n' > "$_repo/unrelated.txt"
  commit_fixture "$_repo" base

  cat > "$_repo/.git/hooks/pre-commit" <<'EOF'
#!/bin/sh
set -eu
case "${GIT_INDEX_FILE:-}" in
  '' | *.lock)
    printf 'hook did not receive a non-.lock alternate index\n' >&2
    exit 1
    ;;
esac
printf '%s\n' "$GIT_INDEX_FILE" > "$HOOK_INDEX_LOG"
printf 'formatted\n' > intended.txt
git add -- intended.txt
EOF
  chmod +x "$_repo/.git/hooks/pre-commit"

  printf 'needs formatting\n' > "$_repo/intended.txt"
  printf 'base\nstaged elsewhere\n' > "$_repo/unrelated.txt"
  git -C "$_repo" add -- unrelated.txt
  _unrelated_oid=$(git -C "$_repo" rev-parse :unrelated.txt)
  _hook_index_log=$test_root/hook-index.log

  (
    cd "$_repo"
    HOOK_INDEX_LOG=$_hook_index_log bash "$helper" commit -m 'Format intended file' -- intended.txt
  ) >/dev/null

  _hook_index=$(cat "$_hook_index_log")
  case "$_hook_index" in
    *.lock) fail 'formatter hook received a .lock index path' ;;
  esac
  assert_equal 'formatted' "$(git -C "$_repo" show HEAD:intended.txt)" 'formatter result was not committed'
  assert_equal "$_unrelated_oid" "$(git -C "$_repo" rev-parse :unrelated.txt)" 'formatter changed unrelated stage'
  assert_equal 'unrelated.txt' "$(git -C "$_repo" diff --cached --name-only)" 'formatter consumed unrelated stage'
}

test_hook_failure_and_lock_ownership() {
  _repo=$test_root/failures
  init_fixture "$_repo"
  printf 'base\n' > "$_repo/intended.txt"
  commit_fixture "$_repo" base
  printf 'changed\n' > "$_repo/intended.txt"

  cat > "$_repo/.git/hooks/pre-commit" <<'EOF'
#!/bin/sh
printf 'intentional hook failure\n' >&2
exit 1
EOF
  chmod +x "$_repo/.git/hooks/pre-commit"

  _head_before=$(git -C "$_repo" rev-parse HEAD)
  _index_before=$(git hash-object "$_repo/.git/index")
  if _failure_output=$(
    cd "$_repo"
    bash "$helper" commit -m 'Must fail' -- intended.txt 2>&1
  ); then
    fail 'hook failure unexpectedly created a commit'
  fi
  assert_contains_line "$_failure_output" 'intentional hook failure' 'hook failure output'
  assert_equal "$_head_before" "$(git -C "$_repo" rev-parse HEAD)" 'hook failure changed HEAD'
  assert_equal "$_index_before" "$(git hash-object "$_repo/.git/index")" 'hook failure changed shared index'
  [ ! -e "$_repo/.git/index.lock" ] || fail 'hook failure left helper-owned index lock'

  rm "$_repo/.git/hooks/pre-commit"
  printf 'other owner\n' > "$_repo/.git/index.lock"
  if _lock_output=$(
    cd "$_repo"
    bash "$helper" commit -m 'Must remain blocked' -- intended.txt 2>&1
  ); then
    fail 'existing index lock did not block commit'
  fi
  printf '%s\n' "$_lock_output" | rg -F -q 'default Git index remains locked' ||
    fail 'existing lock failure lacked explicit lock evidence'
  assert_equal 'other owner' "$(cat "$_repo/.git/index.lock")" 'helper replaced or deleted pre-existing lock'
  assert_equal "$_head_before" "$(git -C "$_repo" rev-parse HEAD)" 'lock refusal changed HEAD'
}

test_baseline_exclusion() {
  _repo=$test_root/baseline-exclusion
  init_fixture "$_repo"
  _base_content=$'line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n'
  _baseline_content=$'line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n'
  _worktree_content=$'line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06\nline 07\nline 08\nline 09\nline 10\nline 11 agent\nline 12\nline 13\nline 14\n'
  _committed_content=$'line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06\nline 07\nline 08\nline 09\nline 10\nline 11 agent\nline 12\nline 13\nline 14'

  printf '%s' "$_base_content" > "$_repo/intended.txt"
  commit_fixture "$_repo" base
  printf '%s' "$_baseline_content" > "$_repo/intended.txt"
  _baseline_oid=$(git -C "$_repo" hash-object -w intended.txt)
  printf '%s' "$_worktree_content" > "$_repo/intended.txt"

  (
    cd "$_repo"
    bash "$helper" commit -m 'Commit only agent change' \
      --exclude-baseline "intended.txt=$_baseline_oid" -- intended.txt
  ) >/dev/null

  assert_equal "$_committed_content" "$(git -C "$_repo" show HEAD:intended.txt)" \
    'baseline exclusion committed stray content'
  assert_equal "${_worktree_content%$'\n'}" "$(cat "$_repo/intended.txt")" \
    'baseline exclusion changed the worktree'
  assert_equal ' M intended.txt' "$(git -C "$_repo" status --short)" \
    'stray baseline hunk did not remain uncommitted'
  assert_equal '' "$(git -C "$_repo" diff --cached --name-only)" \
    'baseline exclusion left staged changes'
}

test_baseline_with_non_overlapping_head_movement() {
  _repo=$test_root/baseline-head-movement
  init_fixture "$_repo"
  _base_content=$'line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n'
  _baseline_content=$'line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n'
  _worktree_content=$'line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\nline 11 agent\nline 12\nline 13\nline 14\n'
  _moved_content=$'line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06 moved HEAD\nline 07\nline 08\nline 09\nline 10\nline 11 original\nline 12\nline 13\nline 14\n'
  _committed_content=$'line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06 moved HEAD\nline 07\nline 08\nline 09\nline 10\nline 11 agent\nline 12\nline 13\nline 14'

  printf '%s' "$_base_content" > "$_repo/intended.txt"
  commit_fixture "$_repo" base
  printf '%s' "$_baseline_content" > "$_repo/intended.txt"
  _baseline_oid=$(git -C "$_repo" hash-object -w intended.txt)
  printf '%s' "$_worktree_content" > "$_repo/intended.txt"
  _moved_head=$(advance_head_with_content "$_repo" "$_moved_content" 'Move HEAD elsewhere')

  (
    cd "$_repo"
    bash "$helper" commit -m 'Apply agent change to moved HEAD' \
      --exclude-baseline "intended.txt=$_baseline_oid" -- intended.txt
  ) >/dev/null

  assert_equal "$_moved_head" "$(git -C "$_repo" rev-parse HEAD^)" \
    'baseline commit did not retain moved HEAD as its parent'
  assert_equal "$_committed_content" "$(git -C "$_repo" show HEAD:intended.txt)" \
    'non-overlapping HEAD movement was not preserved'
  assert_equal "${_worktree_content%$'\n'}" "$(cat "$_repo/intended.txt")" \
    'non-overlapping HEAD movement changed the worktree'
}

test_baseline_conflicting_head_movement() {
  _repo=$test_root/baseline-conflict
  init_fixture "$_repo"
  _base_content=$'line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\n'
  _baseline_content=$'line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06 original\nline 07\nline 08\nline 09\nline 10\n'
  _worktree_content=$'line 01\nline 02 stray\nline 03\nline 04\nline 05\nline 06 agent\nline 07\nline 08\nline 09\nline 10\n'
  _moved_content=$'line 01\nline 02 original\nline 03\nline 04\nline 05\nline 06 moved HEAD\nline 07\nline 08\nline 09\nline 10\n'

  printf '%s' "$_base_content" > "$_repo/intended.txt"
  commit_fixture "$_repo" base
  printf '%s' "$_baseline_content" > "$_repo/intended.txt"
  _baseline_oid=$(git -C "$_repo" hash-object -w intended.txt)
  printf '%s' "$_worktree_content" > "$_repo/intended.txt"
  _moved_head=$(advance_head_with_content "$_repo" "$_moved_content" 'Move HEAD into conflict')
  _status_before=$(git -C "$_repo" status --short)
  _index_before=$(git hash-object "$_repo/.git/index")

  if _conflict_output=$(
    cd "$_repo"
    bash "$helper" commit -m 'Must conflict' \
      --exclude-baseline "intended.txt=$_baseline_oid" -- intended.txt 2>&1
  ); then
    fail 'conflicting HEAD movement unexpectedly created a commit'
  fi
  printf '%s\n' "$_conflict_output" |
    rg -F -q 'baseline changes do not apply cleanly to HEAD for path: intended.txt' ||
    fail 'baseline conflict lacked a path-specific diagnostic'
  assert_equal "$_moved_head" "$(git -C "$_repo" rev-parse HEAD)" \
    'baseline conflict changed HEAD'
  assert_equal "$_index_before" "$(git hash-object "$_repo/.git/index")" \
    'baseline conflict changed the shared index'
  assert_equal "${_worktree_content%$'\n'}" "$(cat "$_repo/intended.txt")" \
    'baseline conflict changed the worktree'
  assert_equal "$_status_before" "$(git -C "$_repo" status --short)" \
    'baseline conflict changed repository status'
  [ ! -e "$_repo/.git/index.lock" ] || fail 'baseline conflict left helper-owned index lock'
}

test_invalid_baseline_arguments() {
  _repo=$test_root/baseline-invalid
  init_fixture "$_repo"
  printf 'base\n' > "$_repo/intended.txt"
  commit_fixture "$_repo" base
  printf 'agent\n' > "$_repo/intended.txt"
  _valid_oid=$(git -C "$_repo" rev-parse HEAD:intended.txt)
  _head_before=$(git -C "$_repo" rev-parse HEAD)
  _status_before=$(git -C "$_repo" status --short)
  _index_before=$(git hash-object "$_repo/.git/index")

  if _invalid_oid_output=$(
    cd "$_repo"
    bash "$helper" commit -m 'Must reject invalid OID' \
      --exclude-baseline 'intended.txt=not-an-oid' -- intended.txt 2>&1
  ); then
    fail 'invalid baseline OID was accepted'
  fi
  printf '%s\n' "$_invalid_oid_output" |
    rg -F -q 'invalid baseline blob OID for path intended.txt: not-an-oid' ||
    fail 'invalid baseline OID lacked a path-specific diagnostic'

  if _invalid_path_output=$(
    cd "$_repo"
    bash "$helper" commit -m 'Must reject unintended path' \
      --exclude-baseline "other.txt=$_valid_oid" -- intended.txt 2>&1
  ); then
    fail 'baseline path outside the intended set was accepted'
  fi
  printf '%s\n' "$_invalid_path_output" |
    rg -F -q 'baseline path is not among intended paths: other.txt' ||
    fail 'unintended baseline path lacked a diagnostic'

  assert_equal "$_head_before" "$(git -C "$_repo" rev-parse HEAD)" \
    'invalid baseline arguments changed HEAD'
  assert_equal "$_index_before" "$(git hash-object "$_repo/.git/index")" \
    'invalid baseline arguments changed the shared index'
  assert_equal "$_status_before" "$(git -C "$_repo" status --short)" \
    'invalid baseline arguments changed repository status'
  [ ! -e "$_repo/.git/index.lock" ] || fail 'invalid baseline arguments created an index lock'
}

test_push_with_upstream() {
  _repo=$test_root/push-upstream
  _remote=$test_root/push-upstream.git
  init_fixture "$_repo"
  git init --bare --quiet "$_remote"

  printf 'base\n' > "$_repo/intended.txt"
  commit_fixture "$_repo" base
  git -C "$_repo" branch -M main
  git -C "$_repo" remote add origin "file://$_remote"
  git -C "$_repo" push --quiet -u origin HEAD

  printf 'pushed\n' > "$_repo/intended.txt"
  _push_output=$(
    cd "$_repo"
    bash "$helper" commit -m 'Push intended file' --push -- intended.txt 2>&1
  )
  _last_line=${_push_output##*$'\n'}

  assert_equal 'pushed main' "$_last_line" 'commit push outcome'
  assert_equal "$(git -C "$_repo" rev-parse HEAD)" \
    "$(git --git-dir="$_remote" rev-parse refs/heads/main)" 'upstream push did not update remote'
}

test_push_new_remote_branch() {
  _repo=$test_root/push-new
  _remote=$test_root/push-new.git
  init_fixture "$_repo"
  git init --bare --quiet "$_remote"

  printf 'base\n' > "$_repo/intended.txt"
  commit_fixture "$_repo" base
  git -C "$_repo" branch -M feature-new
  git -C "$_repo" remote add origin "file://$_remote"

  _push_output=$(
    cd "$_repo"
    bash "$helper" push 2>&1
  )

  assert_equal 'pushed-new feature-new' "$_push_output" 'new remote branch push outcome'
  assert_equal 'origin/feature-new' \
    "$(git -C "$_repo" rev-parse --abbrev-ref --symbolic-full-name '@{upstream}')" \
    'new remote branch upstream'
  assert_equal "$(git -C "$_repo" rev-parse HEAD)" \
    "$(git --git-dir="$_remote" rev-parse refs/heads/feature-new)" \
    'new remote branch push did not update remote'
}

test_push_behind_branch_skip() {
  _behind_repo=$test_root/push-behind
  _remote=$test_root/push-behind.git
  _updater=$test_root/push-behind-updater
  init_fixture "$_behind_repo"
  git init --bare --quiet "$_remote"

  printf 'base\n' > "$_behind_repo/intended.txt"
  commit_fixture "$_behind_repo" base
  git -C "$_behind_repo" branch -M main
  git -C "$_behind_repo" remote add origin "file://$_remote"
  git -C "$_behind_repo" push --quiet -u origin HEAD

  git clone --quiet --branch main "file://$_remote" "$_updater"
  git -C "$_updater" config user.name 'Commit Paths Test'
  git -C "$_updater" config user.email 'commit-paths@example.com'
  git -C "$_updater" config commit.gpgsign false
  printf 'remote\n' > "$_updater/remote.txt"
  commit_fixture "$_updater" remote
  git -C "$_updater" push --quiet
  _remote_head=$(git --git-dir="$_remote" rev-parse refs/heads/main)

  _push_output=$(
    cd "$_behind_repo"
    bash "$helper" push 2>&1
  )

  assert_equal 'behind 1 — push skipped' "$_push_output" 'behind branch push outcome'
  assert_equal "$_remote_head" "$(git --git-dir="$_remote" rev-parse refs/heads/main)" \
    'behind branch push changed remote'
}

test_non_fast_forward_retry_once() {
  _repo=$test_root/push-retry
  _remote=$test_root/push-retry.git
  _shim_dir=$test_root/push-retry-shim
  _push_attempt_log=$test_root/push-attempts.log
  _push_reject_marker=$test_root/push-rejected-once
  _real_git=$(command -v git)
  init_fixture "$_repo"
  git init --bare --quiet "$_remote"

  printf 'base\n' > "$_repo/intended.txt"
  commit_fixture "$_repo" base
  git -C "$_repo" branch -M main
  git -C "$_repo" remote add origin "file://$_remote"
  git -C "$_repo" push --quiet -u origin HEAD
  printf 'retry\n' > "$_repo/intended.txt"
  commit_fixture "$_repo" retry

  mkdir -p "$_shim_dir"
  cat > "$_shim_dir/git" <<'EOF'
#!/bin/sh
if [ "${1:-}" = push ]; then
  printf 'push\n' >> "$PUSH_ATTEMPT_LOG"
  if [ ! -e "$PUSH_REJECT_MARKER" ]; then
    : > "$PUSH_REJECT_MARKER"
    printf ' ! [rejected] main -> main (non-fast-forward)\n' >&2
    printf 'error: failed to push some refs\n' >&2
    exit 1
  fi
fi
exec "$REAL_GIT" "$@"
EOF
  chmod +x "$_shim_dir/git"

  _push_output=$(
    cd "$_repo"
    PATH="$_shim_dir:$PATH" REAL_GIT="$_real_git" \
      PUSH_ATTEMPT_LOG="$_push_attempt_log" PUSH_REJECT_MARKER="$_push_reject_marker" \
      bash "$helper" push 2>&1
  )

  assert_equal 'pushed main' "$_push_output" 'non-fast-forward retry outcome'
  assert_equal $'push\npush' "$(cat "$_push_attempt_log")" 'non-fast-forward push attempt count'
  assert_equal "$(git -C "$_repo" rev-parse HEAD)" \
    "$(git --git-dir="$_remote" rev-parse refs/heads/main)" \
    'non-fast-forward retry did not update remote'
}

test_content_and_shared_index
test_case_only_renames
test_formatter_hook
test_hook_failure_and_lock_ownership
test_baseline_exclusion
test_baseline_with_non_overlapping_head_movement
test_baseline_conflicting_head_movement
test_invalid_baseline_arguments
test_push_with_upstream
test_push_new_remote_branch
test_push_behind_branch_skip
test_non_fast_forward_retry_once

printf 'commit-paths tests passed\n'
