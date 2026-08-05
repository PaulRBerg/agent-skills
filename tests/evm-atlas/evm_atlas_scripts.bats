#!/usr/bin/env bats

# shellcheck disable=SC2016,SC2030,SC2031,SC2154 # Bats provides its variables and runs tests in subshells.
bats_require_minimum_version 1.5.0

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  ETHERSCAN="$REPO_ROOT/skills/evm-atlas/scripts/etherscan-detect-plan.sh"
  BLOCKSCOUT="$REPO_ROOT/skills/evm-atlas/scripts/blockscout-detect-plan.sh"
  RESOLVE_CHAIN="$REPO_ROOT/skills/evm-atlas/scripts/resolve-chain.sh"

  export HOME="$BATS_TEST_TMPDIR/home"
  export MOCK_BIN="$BATS_TEST_TMPDIR/bin"
  export MOCK_CURL_LOG="$BATS_TEST_TMPDIR/curl.log"
  mkdir -p "$HOME" "$MOCK_BIN"
  export PATH="$MOCK_BIN:/usr/bin:/bin"

  printf '%s\n' \
    '#!/bin/bash' \
    'set -eu' \
    'printf "%s\\n" "$*" >> "$MOCK_CURL_LOG"' \
    'if [ "${MOCK_CURL_FAIL:-}" = "1" ]; then' \
    '  exit 22' \
    'fi' \
    'args="$*"' \
    'case "$args" in' \
    '  *getapilimit*) printf "%s" "${MOCK_API_LIMIT_RESPONSE:-}" ;;' \
    '  *chainid=8453*) printf "%s" "${MOCK_PAID_CHAIN_RESPONSE:-}" ;;' \
    '  *chains.blockscout.com/api/chains/*) printf "%s" "${MOCK_CHAIN_RESPONSE:-}" ;;' \
    '  *api.blockscout.com/1/api/v2/addresses/*) printf "%b" "${MOCK_BLOCKSCOUT_HEADERS:-}" ;;' \
    '  *) exit 99 ;;' \
    'esac' > "$MOCK_BIN/curl"
  chmod +x "$MOCK_BIN/curl"
}

@test "Etherscan rejects a missing API key" {
  run env -u ETHERSCAN_API_KEY "$ETHERSCAN"

  [ "$status" -eq 1 ]
  [[ "$output" == *"ETHERSCAN_API_KEY is not set"* ]]
}

@test "Etherscan maps Standard credits without a paid-chain probe" {
  export ETHERSCAN_API_KEY="test-key"
  export MOCK_API_LIMIT_RESPONSE='{"status":"1","creditLimit":200000,"creditsUsed":12,"creditsAvailable":199988,"limitInterval":"daily","intervalExpiryTimespan":"12:00:00"}'

  run "$ETHERSCAN"

  [ "$status" -eq 0 ]
  [ "$output" = $'plan=standard\ncredit_limit=200000\ncredits_used=12\ncredits_available=199988\nlimit_interval=daily\ninterval_expiry=12:00:00\npro_endpoints=true\npaid_chains=true' ]
  [ "$(wc -l < "$MOCK_CURL_LOG" | tr -d ' ')" -eq 1 ]
}

@test "Etherscan distinguishes Free from Lite with the paid-chain probe" {
  export ETHERSCAN_API_KEY="test-key"
  export MOCK_API_LIMIT_RESPONSE='{"status":"1","creditLimit":100000,"creditsUsed":3,"creditsAvailable":99997,"limitInterval":"daily","intervalExpiryTimespan":"23:59:59"}'
  export MOCK_PAID_CHAIN_RESPONSE='{"status":"0","message":"NOTOK","result":"Free plan"}'

  run "$ETHERSCAN"

  [ "$status" -eq 0 ]
  [[ "$output" == *$'plan=free\n'* ]]
  [[ "$output" == *"paid_chains=false"* ]]
  [ "$(wc -l < "$MOCK_CURL_LOG" | tr -d ' ')" -eq 2 ]

  export MOCK_PAID_CHAIN_RESPONSE='{"status":"1","message":"OK","result":"0"}'
  run "$ETHERSCAN"

  [ "$status" -eq 0 ]
  [[ "$output" == *$'plan=lite\n'* ]]
  [[ "$output" == *"pro_endpoints=false"* ]]
  [[ "$output" == *"paid_chains=true"* ]]
}

@test "Etherscan reports unknown and failed API-limit responses" {
  export ETHERSCAN_API_KEY="test-key"
  export MOCK_API_LIMIT_RESPONSE='{"status":"1","creditLimit":123456,"creditsUsed":1,"creditsAvailable":123455,"limitInterval":"daily","intervalExpiryTimespan":"00:00:00"}'

  run "$ETHERSCAN"

  [ "$status" -eq 0 ]
  [[ "$output" == *$'plan=unknown\n'* ]]
  [[ "$output" == *"pro_endpoints=unknown"* ]]

  export MOCK_API_LIMIT_RESPONSE='{"status":"0","message":"NOTOK","result":"invalid key"}'
  run "$ETHERSCAN"

  [ "$status" -eq 1 ]
  [[ "$output" == *"getapilimit failed — message=NOTOK result=invalid key"* ]]
}

@test "Blockscout rejects a missing API key" {
  run env -u BLOCKSCOUT_API_KEY "$BLOCKSCOUT"

  [ "$status" -eq 1 ]
  [[ "$output" == *"BLOCKSCOUT_API_KEY is not set"* ]]
}

@test "Blockscout parses case-insensitive rate-limit headers" {
  export BLOCKSCOUT_API_KEY="test-key"
  export MOCK_BLOCKSCOUT_HEADERS=$'HTTP/2 200\r\nX-RateLimit-Limit: 15\r\nx-ratelimit-remaining: 9\r\nX-Ratelimit-Reset: 42\r\nX-Credits-Remaining: 123\r\n\r\n'

  run "$BLOCKSCOUT"

  [ "$status" -eq 0 ]
  [ "$output" = $'plan=standard\nrate_limit_rps=15\nrate_limit_remaining=9\nrate_limit_reset=42\ncredits_remaining=123' ]
  [[ "$(<"$MOCK_CURL_LOG")" == *"authorization: Bearer test-key"* ]]
}

@test "Blockscout returns unknown for unrecognized or missing headers" {
  export BLOCKSCOUT_API_KEY="test-key"
  export MOCK_BLOCKSCOUT_HEADERS=$'HTTP/2 200\r\nx-ratelimit-limit: 7\r\n\r\n'

  run "$BLOCKSCOUT"

  [ "$status" -eq 0 ]
  [ "$output" = $'plan=unknown\nrate_limit_rps=7\nrate_limit_remaining=\nrate_limit_reset=\ncredits_remaining=' ]
}

@test "Blockscout makes request failures explicit" {
  export BLOCKSCOUT_API_KEY="test-key"
  export MOCK_CURL_FAIL=1

  run "$BLOCKSCOUT"

  [ "$status" -eq 1 ]
  [[ "$output" == *"request failed — invalid key or network issue"* ]]
}

@test "resolve-chain accepts a target chain with a matching Chainscout response" {
  export MOCK_CHAIN_RESPONSE='{"name":"Ethereum Mainnet","native_currency":"ETH","explorers":[{"url":"https://eth.blockscout.com/"}],"hostedBy":"blockscout","isTestnet":false,"layer":1,"rollupType":""}'

  run "$RESOLVE_CHAIN" 1

  [ "$status" -eq 0 ]
  [ "$output" = $'chain_id=1\nname=Ethereum Mainnet\nnative_currency=ETH\ninstance_url=https://eth.blockscout.com/\nhosted_by=blockscout\nis_testnet=false\nlayer=1\nrollup_type=' ]
}

@test "resolve-chain rejects unsafe and out-of-scope target IDs before requesting Chainscout" {
  run "$RESOLVE_CHAIN" 250

  [ "$status" -eq 1 ]
  [[ "$output" == *"chain_id=250 is marked unsafe"* ]]
  [ ! -e "$MOCK_CURL_LOG" ]

  run "$RESOLVE_CHAIN" 999999

  [ "$status" -eq 2 ]
  [[ "$output" == *"outside the evm-atlas target list"* ]]
  [ ! -e "$MOCK_CURL_LOG" ]
}

@test "resolve-chain refuses a Chainscout name that does not match the target" {
  export MOCK_CHAIN_RESPONSE='{"name":"Base","native_currency":"ETH","explorers":[{"url":"https://base.blockscout.com/"}]}'

  run "$RESOLVE_CHAIN" 1

  [ "$status" -eq 1 ]
  [[ "$output" == *"returned name=Base for target chain_id=1"* ]]
  [[ "$output" == *"Refusing to use a non-target Chainscout registry match"* ]]
}

@test "resolve-chain rejects missing and malformed Chainscout responses" {
  export MOCK_CHAIN_RESPONSE='{}'

  run "$RESOLVE_CHAIN" 1

  [ "$status" -eq 1 ]
  [[ "$output" == *"chain_id=1 not found in Chainscout"* ]]

  export MOCK_CHAIN_RESPONSE='not-json'
  run "$RESOLVE_CHAIN" 1

  [ "$status" -eq 1 ]
  [[ "$output" == *"chain_id=1 not found in Chainscout"* ]]
}
