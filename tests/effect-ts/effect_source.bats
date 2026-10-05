#!/usr/bin/env bats

# shellcheck disable=SC2030,SC2031,SC2154 # Bats provides its variables and runs tests in subshells.
bats_require_minimum_version 1.5.0

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  SCRIPT="$REPO_ROOT/skills/effect-ts/scripts/effect-source.sh"

  export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.com
  export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.com
  export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
  export EFFECT_SOURCE_REPO="$BATS_TEST_TMPDIR/origin"
  export EFFECT_SOURCE_DIR="$BATS_TEST_TMPDIR/cache/effect"
  ORIGIN="$EFFECT_SOURCE_REPO"

  git init --quiet -b main "$ORIGIN"
  release 4.0.0
  release 4.0.1
  git -C "$ORIGIN" commit --quiet --allow-empty -m unreleased
}

release() {
  git -C "$ORIGIN" commit --quiet --allow-empty -m "Version Packages $1"
  git -C "$ORIGIN" tag "effect@$1"
  git -C "$ORIGIN" tag "@effect/vitest@$1"
}

@test "clones and checks out the newest release on main" {
  run --separate-stderr "$SCRIPT"

  [ "$status" -eq 0 ]
  [ "$output" = $'path='"$EFFECT_SOURCE_DIR"$'\nrelease=effect@4.0.1' ]
  [ "$(git -C "$EFFECT_SOURCE_DIR" rev-parse HEAD)" = "$(git -C "$ORIGIN" rev-list -n1 effect@4.0.1)" ]
}

@test "reuses the cache within 24 hours" {
  "$SCRIPT" >/dev/null
  release 4.0.2

  run --separate-stderr "$SCRIPT"

  [ "$status" -eq 0 ]
  [[ "$output" == *"release=effect@4.0.1" ]]
}

@test "fetches and advances once the cache is older than 24 hours" {
  "$SCRIPT" >/dev/null
  release 4.0.2
  echo $(($(date +%s) - 86400)) >"$EFFECT_SOURCE_DIR/.git/effect-source-fetched-at"

  run --separate-stderr "$SCRIPT"

  [ "$status" -eq 0 ]
  [[ "$output" == *"release=effect@4.0.2" ]]
}

@test "falls back to the cache when the fetch fails" {
  "$SCRIPT" >/dev/null
  rm -rf "$ORIGIN"
  echo 0 >"$EFFECT_SOURCE_DIR/.git/effect-source-fetched-at"

  run --separate-stderr "$SCRIPT"

  [ "$status" -eq 0 ]
  [[ "$output" == *"release=effect@4.0.1" ]]
  [[ "$stderr" == *"fetch failed; using cached source"* ]]
}

@test "refuses to move a checkout with local changes" {
  "$SCRIPT" >/dev/null
  release 4.0.2
  echo 0 >"$EFFECT_SOURCE_DIR/.git/effect-source-fetched-at"
  touch "$EFFECT_SOURCE_DIR/scratch"

  run --separate-stderr "$SCRIPT"

  [ "$status" -eq 1 ]
  [[ "$stderr" == *"has local changes"* ]]
}
