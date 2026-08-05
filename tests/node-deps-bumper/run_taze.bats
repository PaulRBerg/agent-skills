#!/usr/bin/env bats

# shellcheck disable=SC2154 # Bats provides BATS_TEST_DIRNAME and BATS_TEST_TMPDIR.

setup_file() {
  bats_require_minimum_version 1.5.0
}

setup() {
  repo_root="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  helper="$repo_root/skills/node-deps-bumper/scripts/run-taze.sh"
  project="$BATS_TEST_TMPDIR/project"
  mock_bin="$BATS_TEST_TMPDIR/bin"
  taze_log="$BATS_TEST_TMPDIR/taze.log"
  uv_log="$BATS_TEST_TMPDIR/uv.log"

  mkdir -p "$project" "$mock_bin" "$BATS_TEST_TMPDIR/home"
  export HOME="$BATS_TEST_TMPDIR/home"
  export PATH="$mock_bin:/usr/bin:/bin"
  export TMPDIR="$BATS_TEST_TMPDIR"
  export TAZE_LOG="$taze_log"
  export UV_LOG="$uv_log"

  install_taze_mock
  install_uv_mock
}

install_taze_mock() {
  cat >"$mock_bin/taze" <<'EOF'
#!/usr/bin/env bash
set -eu

if [ "${1:-}" = "--help" ]; then
  printf '%s\n' "${TAZE_HELP_TEXT:-}"
  exit 0
fi

printf '%s\n' "$@" >"$TAZE_LOG"
printf '%s\n' "${TAZE_OUTPUT:-fixture - up to date}"
EOF
  chmod 755 "$mock_bin/taze"
}

install_uv_mock() {
  cat >"$mock_bin/uv" <<'EOF'
#!/usr/bin/env bash
set -eu

printf '%s\n' "$@" >"$UV_LOG"
printf '%s\n' '{"updates":[]}'
EOF
  chmod 755 "$mock_bin/uv"
}

write_package_json() {
  printf '%s\n' "$1" >"$project/package.json"
}

assert_taze_args() {
  [ "$(cat "$taze_log")" = "$1" ]
}

@test "fails when the target has no package manifest" {
  rm -f "$mock_bin/taze"

  run "$helper" "$project"

  [ "$status" -eq 2 ]
  [[ "$output" == *"ERROR: No package.json found in $project"* ]]
}

@test "fails with installation guidance when taze is unavailable" {
  write_package_json '{"name":"fixture"}'
  rm -f "$mock_bin/taze"

  run "$helper" "$project"

  [ "$status" -eq 1 ]
  [[ "$output" == *"ERROR: taze CLI is not installed."* ]]
  [[ "$output" == *"npm install -g taze"* ]]
}

@test "rejects unknown, incomplete, and duplicate target arguments" {
  write_package_json '{"name":"fixture"}'

  run "$helper" --unknown "$project"
  [ "$status" -eq 64 ]
  [[ "$output" == *"ERROR: Unknown option: --unknown"* ]]

  run "$helper" --include
  [ "$status" -ne 0 ]
  [[ "$output" == *"ERROR: --include requires a value"* ]]

  run "$helper" "$project" "$project"
  [ "$status" -eq 64 ]
  [[ "$output" == *"ERROR: Only one target path is supported"* ]]
}

@test "recurses workspaces and forwards include and concurrency while scanning locked versions" {
  write_package_json '{"name":"fixture","workspaces":["packages/*"]}'

  run "$helper" --include eslint,react --concurrency 4 "$project"

  [ "$status" -eq 0 ]
  assert_taze_args $'major\n-r\n--include\neslint,react\n--concurrency\n4\n--include-locked'
}

@test "uses pnpm workspace files to enable recursive scans" {
  write_package_json '{"name":"fixture"}'
  : >"$project/pnpm-workspace.yaml"

  run "$helper" "$project"

  [ "$status" -eq 0 ]
  assert_taze_args $'major\n-r\n--include-locked'
}

@test "requires an explicit selection before writing and forbids plan and write together" {
  write_package_json '{"name":"fixture"}'

  run "$helper" --write "$project"
  [ "$status" -eq 64 ]
  [[ "$output" == *"ERROR: --write requires --include with the selected package list"* ]]

  run "$helper" --write --plan --include react "$project"
  [ "$status" -eq 64 ]
  [[ "$output" == *"ERROR: --plan and --write are mutually exclusive"* ]]

  run "$helper" --write --include react "$project"
  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--include\nreact\n--write'
}

@test "mirrors Bun maturity period and supported exclusions" {
  write_package_json '{"name":"fixture"}'
  : >"$project/bun.lock"
  cat >"$project/bunfig.toml" <<'EOF'
minimumReleaseAge = 90000
minimumReleaseAgeExcludes = ["react", "@types/node"]
EOF
  export TAZE_HELP_TEXT='  --maturity-period-exclude <packages>'

  run "$helper" "$project"

  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--maturity-period\n2\n--maturity-period-exclude\nreact,@types/node\n--include-locked'
}

@test "does not pass unsupported Bun maturity exclusions" {
  write_package_json '{"name":"fixture"}'
  : >"$project/bun.lockb"
  cat >"$project/bunfig.toml" <<'EOF'
minimumReleaseAge = 1
minimumReleaseAgeExcludes = ["react"]
EOF

  run "$helper" "$project"

  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--maturity-period\n1\n--include-locked'
}

@test "runs the plan parser against captured noninteractive taze output" {
  write_package_json '{"name":"fixture"}'
  export TAZE_OUTPUT='react dependencies ^18.2.0 → ^19.0.0'

  run "$helper" --plan --include react "$project"

  [ "$status" -eq 0 ]
  [ "$output" = '{"updates":[]}' ]
  assert_taze_args $'major\n--include\nreact\n--include-locked\n--no-group\n--no-timediff\n--no-nodecompat\n--sort\nname-asc'
  [ "$(sed -n '1p' "$uv_log")" = 'run' ]
  [ "$(sed -n '2p' "$uv_log")" = "$repo_root/skills/node-deps-bumper/scripts/parse-taze-plan.py" ]
  [ "$(sed -n '3p' "$uv_log")" = '--input' ]
  [ -n "$(sed -n '4p' "$uv_log")" ]
}
