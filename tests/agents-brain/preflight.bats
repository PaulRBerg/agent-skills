#!/usr/bin/env bats

setup() {
  # shellcheck disable=SC2154
  repo_root=$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)
  helper=$repo_root/skills/agents-brain/scripts/preflight.sh
  test_root=$(mktemp -d "${TMPDIR:-/tmp}/agents-brain-preflight.XXXXXX")
  test_root=$(cd "$test_root" && pwd -P)
  fake_bin=$test_root/bin
  fake_home=$test_root/home
  mkdir -p "$fake_bin" "$fake_home"
  export HOME=$fake_home
  export PATH=$fake_bin:$PATH
  export GIT_CONFIG_GLOBAL=/dev/null
  export GIT_CONFIG_SYSTEM=/dev/null
}

teardown() {
  rm -rf "$test_root"
}

fake_claude() {
  cat >"$fake_bin/claude" <<SCRIPT
#!/bin/bash
$1
SCRIPT
  chmod 755 "$fake_bin/claude"
}

init_repo() {
  mkdir -p "$1"
  git -C "$1" init -q
}

@test "prints repository facts with a native Claude Code" {
  init_repo "$test_root/project"
  fake_claude "printf '2.1.319 (Claude Code)\n'"
  cd "$test_root/project"
  run "$helper"
  [ "$status" -eq 0 ]
  [ "${lines[0]}" = "repo_root=$test_root/project" ]
  [ "${lines[1]}" = 'managed_skill_root=' ]
  [ "${lines[2]}" = 'claude_version=2.1.319' ]
  [ "${lines[3]}" = 'agents_md_native=true' ]
}

@test "treats the minimum native version as native" {
  init_repo "$test_root/project"
  fake_claude "printf '2.1.277 (Claude Code)\n'"
  cd "$test_root/project"
  run "$helper"
  [ "$status" -eq 0 ]
  [ "${lines[3]}" = 'agents_md_native=true' ]
}

@test "reports an older Claude Code as not native" {
  init_repo "$test_root/project"
  fake_claude "printf '2.1.200 (Claude Code)\n'"
  cd "$test_root/project"
  run "$helper"
  [ "$status" -eq 0 ]
  [ "${lines[2]}" = 'claude_version=2.1.200' ]
  [ "${lines[3]}" = 'agents_md_native=false' ]
}

@test "reports a missing Claude Code as not native" {
  init_repo "$test_root/project"
  fake_claude 'exit 127'
  cd "$test_root/project"
  run "$helper"
  [ "$status" -eq 0 ]
  [ "${lines[2]}" = 'claude_version=' ]
  [ "${lines[3]}" = 'agents_md_native=false' ]
}

@test "sets managed_skill_root at an agent configuration repository root" {
  init_repo "$fake_home/.agents"
  fake_claude 'exit 127'
  cd "$fake_home/.agents"
  run "$helper"
  [ "$status" -eq 0 ]
  [ "${lines[1]}" = "managed_skill_root=$fake_home/.agents/skills" ]
}

@test "aborts inside a managed skills tree" {
  init_repo "$fake_home/.claude"
  mkdir -p "$fake_home/.claude/skills/demo"
  cd "$fake_home/.claude/skills/demo"
  run "$helper"
  [ "$status" -eq 1 ]
  [ "$output" = "abort: installed skills must be edited in their source catalog: $fake_home/.claude/skills/demo" ]
}

@test "aborts when the repository is nested under an agent configuration repository" {
  init_repo "$fake_home/.codex/nested"
  cd "$fake_home/.codex/nested"
  run "$helper"
  [ "$status" -eq 1 ]
  [ "$output" = "abort: repo root is nested under an agent configuration repository: $fake_home/.codex/nested" ]
}

@test "aborts when the home directory is the repository root" {
  init_repo "$fake_home"
  cd "$fake_home"
  run "$helper"
  [ "$status" -eq 1 ]
  [ "$output" = "abort: unsupported repo root: $fake_home" ]
}

@test "aborts outside a git repository" {
  mkdir -p "$test_root/plain"
  cd "$test_root/plain"
  run "$helper"
  [ "$status" -eq 1 ]
  [ "$output" = 'abort: not inside a git repository' ]
}

@test "aborts at the filesystem root" {
  cd /
  run "$helper"
  [ "$status" -eq 1 ]
  [ "$output" = 'abort: refusing to run at the filesystem root' ]
}
