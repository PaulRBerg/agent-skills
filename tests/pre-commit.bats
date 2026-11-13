#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

# shellcheck disable=SC2154 # Bats provides BATS_TEST_DIRNAME and BATS_TEST_TMPDIR.
setup() {
  local repo_root
  repo_root="$(cd "$BATS_TEST_DIRNAME/.." && pwd)"
  export HOOK="$repo_root/.husky/pre-commit"
  export BUN_ARGS="$BATS_TEST_TMPDIR/bun-args"
  export BUN_EXIT=0
  mkdir "$BATS_TEST_TMPDIR/bin" "$BATS_TEST_TMPDIR/repo"
  cat >"$BATS_TEST_TMPDIR/bin/bun" <<'STUB'
#!/bin/sh
printf '%s\n' "$@" >"$BUN_ARGS"
exit "$BUN_EXIT"
STUB
  chmod +x "$BATS_TEST_TMPDIR/bin/bun"
  export PATH="$BATS_TEST_TMPDIR/bin:$PATH"
  cd "$BATS_TEST_TMPDIR/repo" || exit
  git init -q -b main
  git config user.email fixture@example.com
  git config user.name Fixture
  git config core.hooksPath /dev/null
  printf 'base\n' >"file with spaces.txt"
  printf 'base\n' >stash-seed
  git add .
  git commit -qm fixture
  printf 'stashed\n' >stash-seed
  git stash push -qm fixture-stash
}

@test "partial staging rejects a spaced filename and preserves worktree, index, and stash" {
  printf 'staged\n' >"file with spaces.txt"
  git add -- "file with spaces.txt"
  printf 'unstaged\n' >>"file with spaces.txt"
  cp "file with spaces.txt" "$BATS_TEST_TMPDIR/worktree-before"
  cp .git/index "$BATS_TEST_TMPDIR/index-before"
  git stash list --format=%H >"$BATS_TEST_TMPDIR/stash-before"

  run /bin/sh "$HOOK"

  [ "$status" -eq 1 ]
  [[ "$output" == *"unsafe in the shared worktree"* ]]
  [[ "$output" == *"file with spaces.txt"* ]]
  [ ! -e "$BUN_ARGS" ]
  cmp "file with spaces.txt" "$BATS_TEST_TMPDIR/worktree-before"
  cmp .git/index "$BATS_TEST_TMPDIR/index-before"
  git stash list --format=%H >"$BATS_TEST_TMPDIR/stash-after"
  cmp "$BATS_TEST_TMPDIR/stash-before" "$BATS_TEST_TMPDIR/stash-after"
}

@test "partial staging rejects Unicode, tab, and newline filenames without mutation" {
  local file
  for file in "café.txt" $'tab\tname.txt' $'line\nname.txt'; do
    printf 'staged\n' >"$file"
    git add -- "$file"
    printf 'unstaged\n' >>"$file"
    cp "$file" "$BATS_TEST_TMPDIR/worktree-before"
    cp .git/index "$BATS_TEST_TMPDIR/index-before"
    git stash list --format=%H >"$BATS_TEST_TMPDIR/stash-before"

    run /bin/sh "$HOOK"

    [ "$status" -eq 1 ]
    [[ "$output" == *"unsafe in the shared worktree"* ]]
    [ ! -e "$BUN_ARGS" ]
    cmp "$file" "$BATS_TEST_TMPDIR/worktree-before"
    cmp .git/index "$BATS_TEST_TMPDIR/index-before"
    git stash list --format=%H >"$BATS_TEST_TMPDIR/stash-after"
    cmp "$BATS_TEST_TMPDIR/stash-before" "$BATS_TEST_TMPDIR/stash-after"
    git reset -q HEAD -- "$file"
    rm "$file"
  done
}

@test "unmerged files reject before lint" {
  printf 'base\n' >conflict.txt
  git add conflict.txt
  git commit -qm conflict-base
  git checkout -qb other
  printf 'other\n' >conflict.txt
  git commit -qam other
  git checkout -q main
  printf 'main\n' >conflict.txt
  git commit -qam main
  if git merge --no-ff other >/dev/null 2>&1; then
    return 1
  fi

  run /bin/sh "$HOOK"

  [ "$status" -eq 1 ]
  [[ "$output" == *"unsafe in the shared worktree"* ]]
  [ ! -e "$BUN_ARGS" ]
}

@test "fully staged files dispatch to the lint-staged package script" {
  printf 'staged\n' >"file with spaces.txt"
  git add -- "file with spaces.txt"

  run /bin/sh "$HOOK"

  [ "$status" -eq 0 ]
  printf '%s\n' run lint-staged >"$BATS_TEST_TMPDIR/expected-args"
  cmp "$BUN_ARGS" "$BATS_TEST_TMPDIR/expected-args"
}

@test "no staged files skip lint" {
  printf 'unstaged\n' >>"file with spaces.txt"

  run /bin/sh "$HOOK"

  [ "$status" -eq 0 ]
  [ ! -e "$BUN_ARGS" ]
}

@test "lint failures propagate their exit status" {
  printf 'staged\n' >"file with spaces.txt"
  git add -- "file with spaces.txt"
  export BUN_EXIT=23

  run /bin/sh "$HOOK"

  [ "$status" -eq 23 ]
  [ -e "$BUN_ARGS" ]
}
