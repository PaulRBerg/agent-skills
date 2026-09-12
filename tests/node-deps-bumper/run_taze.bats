#!/usr/bin/env bats

# shellcheck disable=SC2154 # Bats provides BATS_TEST_DIRNAME and BATS_TEST_TMPDIR.

setup_file() {
  bats_require_minimum_version 1.5.0
  TAZE_TEST_PYTHON="$(uv python find '>=3.11')"
  export TAZE_TEST_PYTHON
}

setup() {
  repo_root="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  helper="$repo_root/skills/node-deps-bumper/scripts/run-taze.sh"
  project="$BATS_TEST_TMPDIR/project"
  mock_bin="$BATS_TEST_TMPDIR/bin"
  taze_log="$BATS_TEST_TMPDIR/taze.log"
  taze_help="$BATS_TEST_TMPDIR/taze-help.txt"
  uv_log="$BATS_TEST_TMPDIR/uv.log"

  mkdir -p "$project" "$mock_bin" "$BATS_TEST_TMPDIR/home"
  export HOME="$BATS_TEST_TMPDIR/home"
  unset XDG_CONFIG_HOME
  export PATH="$mock_bin:/usr/bin:/bin"
  export TMPDIR="$BATS_TEST_TMPDIR"
  export TAZE_LOG="$taze_log"
  export TAZE_HELP_FILE="$taze_help"
  export UV_LOG="$uv_log"

  install_taze_mock
  install_uv_mock
  : >"$taze_help"
}

install_taze_mock() {
  cat >"$mock_bin/taze" <<'EOF'
#!/usr/bin/env bash
set -eu

if [ "${1:-}" = "--help" ]; then
  cat "$TAZE_HELP_FILE"
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
if [[ "${2:-}" == */bun-maturity.py ]]; then
  shift
  exec "$TAZE_TEST_PYTHON" "$@"
fi
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
[install]
minimumReleaseAge = 90000
minimumReleaseAgeExcludes = ["react", "@types/node"]
EOF
  printf '%s\n' '  --maturity-period-exclude <packages>' >"$taze_help"

  run "$helper" "$project"

  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--maturity-period\n2\n--maturity-period-exclude\nreact,@types/node\n--include-locked'
}

@test "does not pass unsupported Bun maturity exclusions" {
  write_package_json '{"name":"fixture"}'
  : >"$project/bun.lockb"
  cat >"$project/bunfig.toml" <<'EOF'
[install]
minimumReleaseAge = 1
minimumReleaseAgeExcludes = ["react"]
EOF

  run "$helper" "$project"

  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--maturity-period\n1\n--include-locked'
}

@test "inherits global Bun policy and multiline exclusions for both scan and write" {
  write_package_json '{"name":"fixture"}'
  : >"$project/bun.lock"
  cat >"$HOME/.bunfig.toml" <<'EOF'
[install]
minimumReleaseAge = 604_800
minimumReleaseAgeExcludes = [
  'react', # allow an independently managed package
  "@types/node",
]
EOF
  printf '%s\n' '  --maturity-period-exclude <packages>' >"$taze_help"

  run "$helper" --include react "$project"
  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--include\nreact\n--maturity-period\n7\n--maturity-period-exclude\nreact,@types/node\n--include-locked'

  printf '%s\n' '[install]' 'linker = "hoisted"' >"$project/bunfig.toml"
  run "$helper" --write --include react "$project"
  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--include\nreact\n--maturity-period\n7\n--maturity-period-exclude\nreact,@types/node\n--write'
}

@test "uses XDG global config and overlays individual local policy keys" {
  write_package_json '{"name":"fixture"}'
  : >"$project/bun.lock"
  export XDG_CONFIG_HOME="$BATS_TEST_TMPDIR/config"
  mkdir -p "$XDG_CONFIG_HOME"
  printf '%s\n' '[install]' 'minimumReleaseAge = 86400' >"$HOME/.bunfig.toml"
  cat >"$XDG_CONFIG_HOME/.bunfig.toml" <<'EOF'
[install]
minimumReleaseAge = 604800
minimumReleaseAgeExcludes = ["react"]
EOF
  printf '%s\n' '  --maturity-period-exclude <packages>' >"$taze_help"
  printf '%s\n' '[install]' 'minimumReleaseAgeExcludes = []' >"$project/bunfig.toml"

  run "$helper" "$project"
  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--maturity-period\n7\n--include-locked'

  printf '%s\n' '[install]' 'minimumReleaseAge = 90000' >"$project/bunfig.toml"
  run "$helper" "$project"
  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--maturity-period\n2\n--maturity-period-exclude\nreact\n--include-locked'

  printf '%s\n' '[install]' 'minimumReleaseAge = 0' >"$project/bunfig.toml"
  run "$helper" "$project"
  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--include-locked'
}

@test "does not apply user Bun policy to other package managers" {
  write_package_json '{"name":"fixture"}'
  printf '%s\n' '[install]' 'minimumReleaseAge = 604800' >"$HOME/.bunfig.toml"

  run "$helper" "$project"
  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--include-locked'
  [ ! -e "$uv_log" ]
}

@test "Bun projects with no policy do not gain a maturity filter" {
  write_package_json '{"name":"fixture"}'
  : >"$project/bun.lock"

  run "$helper" "$project"
  [ "$status" -eq 0 ]
  assert_taze_args $'major\n--include-locked'
}

@test "invalid Bun policy fails before Taze without exposing config contents" {
  write_package_json '{"name":"fixture"}'
  : >"$project/bun.lock"
  printf '%s\n' '[install]' 'minimumReleaseAge = "fixture-secret"' >"$project/bunfig.toml"

  run "$helper" --write --include react "$project"
  [ "$status" -ne 0 ]
  [[ "$output" == *'minimumReleaseAge must be a nonnegative integer'* ]]
  [[ "$output" != *'fixture-secret'* ]]
  [ ! -e "$taze_log" ]

  printf '%s\n' '[install]' 'registry = "fixture-secret' >"$project/bunfig.toml"
  run "$helper" "$project"
  [ "$status" -ne 0 ]
  [[ "$output" == *'cannot read Bun configuration'* ]]
  [[ "$output" != *'fixture-secret'* ]]
  [ ! -e "$taze_log" ]
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
