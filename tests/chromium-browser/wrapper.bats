#!/usr/bin/env bats

# shellcheck disable=SC2154 # Bats provides BATS_TEST_DIRNAME and BATS_TEST_TMPDIR.

setup_file() {
  bats_require_minimum_version 1.5.0
}

setup() {
  wrapper="$BATS_TEST_DIRNAME/../../skills/chromium-browser/scripts/chrome-devtools-mcp.sh"
  export BUN_INSTALL="$BATS_TEST_TMPDIR/bun install"
  export XDG_CACHE_HOME="$BATS_TEST_TMPDIR/cache"
  export WRAPPER_ARGS="$BATS_TEST_TMPDIR/args"
  export WRAPPER_ENV="$BATS_TEST_TMPDIR/env"
  unset PRB_AGENT_CHROMIUM_PORT NODE_OPTIONS CHROME_DEVTOOLS_MCP_NO_CONFIG_DISCOVERY
  mkdir -p "$BUN_INSTALL/bin"
  cat >"$BUN_INSTALL/bin/chrome-devtools-mcp" <<'EOF'
#!/bin/bash
set -eu
printf '%s\n' "$@" >"$WRAPPER_ARGS"
printf '%s\n' "$CHROME_DEVTOOLS_MCP_NO_CONFIG_DISCOVERY" "$NODE_OPTIONS" >"$WRAPPER_ENV"
exit "${WRAPPER_EXIT:-0}"
EOF
  chmod +x "$BUN_INSTALL/bin/chrome-devtools-mcp"
}

@test "attaches with privacy and screenshot options" {
  run /bin/bash "$wrapper"

  [ "$status" -eq 0 ]
  expected_args="$(cat <<'EOF'
--browser-url=http://127.0.0.1:9222
--page-id-routing
--memory-debugging
--experimental-vision
--redact-network-headers
--no-performance-crux
--no-usage-statistics
--workspace=/
--screenshot-format=jpeg
--screenshot-quality=80
--screenshot-max-width=1600
--screenshot-max-height=1600
EOF
)"
  [ "$(sed '$d' "$WRAPPER_ARGS")" = "$expected_args" ]
  log_arg="$(tail -n 1 "$WRAPPER_ARGS")"
  [[ "$log_arg" == "--log-file=$XDG_CACHE_HOME/chrome-devtools-mcp/logs/server-"*.log ]]
  [ -d "$XDG_CACHE_HOME/chrome-devtools-mcp/logs" ]
  [ "$(cat "$WRAPPER_ENV")" = $'1\n--max-old-space-size=8192' ]
}

@test "preserves custom port and Node options while disabling config discovery" {
  export PRB_AGENT_CHROMIUM_PORT=9333
  export NODE_OPTIONS='--trace-warnings'
  export CHROME_DEVTOOLS_MCP_NO_CONFIG_DISCOVERY=0
  export WRAPPER_EXIT=23

  run /bin/bash "$wrapper"

  [ "$status" -eq 23 ]
  [ "$(head -n 1 "$WRAPPER_ARGS")" = '--browser-url=http://127.0.0.1:9333' ]
  [ "$(cat "$WRAPPER_ENV")" = $'1\n--trace-warnings --max-old-space-size=8192' ]
}

@test "fails before launching when the installed executable is missing" {
  rm "$BUN_INSTALL/bin/chrome-devtools-mcp"

  run -127 /bin/bash "$wrapper"

  [ "$status" -eq 127 ]
  [[ "$output" == *"chrome-devtools-mcp is not installed at $BUN_INSTALL/bin/chrome-devtools-mcp"* ]]
  [ ! -e "$WRAPPER_ARGS" ]
}
