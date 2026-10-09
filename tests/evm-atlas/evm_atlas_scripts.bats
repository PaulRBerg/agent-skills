#!/usr/bin/env bats

# shellcheck disable=SC2016,SC2030,SC2031,SC2154 # Bats provides its variables and runs tests in subshells.
bats_require_minimum_version 1.5.0

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  ETHERSCAN="$REPO_ROOT/skills/evm-atlas/scripts/etherscan-detect-plan.sh"
  BLOCKSCOUT="$REPO_ROOT/skills/evm-atlas/scripts/blockscout-detect-plan.sh"
  RESOLVE_CHAIN="$REPO_ROOT/skills/evm-atlas/scripts/resolve-chain.sh"
  CHAIN_LOOKUP="$REPO_ROOT/skills/evm-atlas/scripts/chain-lookup.sh"

  export HOME="$BATS_TEST_TMPDIR/home"
  export MOCK_BIN="$BATS_TEST_TMPDIR/bin"
  export MOCK_CURL_LOG="$BATS_TEST_TMPDIR/curl.log"
  export MOCK_ETHERSCAN_LOG="$BATS_TEST_TMPDIR/etherscan.log"
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
    '  *chains.blockscout.com/api/chains/*) printf "%s" "${MOCK_CHAIN_RESPONSE:-}" ;;' \
    '  *api.blockscout.com/1/api/v2/addresses/*) printf "%b" "${MOCK_BLOCKSCOUT_HEADERS:-}" ;;' \
    '  *) exit 99 ;;' \
    'esac' > "$MOCK_BIN/curl"
  chmod +x "$MOCK_BIN/curl"

  cat > "$MOCK_BIN/etherscan" <<'EOF'
#!/bin/bash
set -eu
printf '%s\n' "$*" >> "$MOCK_ETHERSCAN_LOG"
case "$*" in
  'apilimit --chain 1 --output json')
    printf '%s' "${MOCK_API_LIMIT_RESPONSE:-}"
    printf '%s' "${MOCK_API_LIMIT_ERROR:-}" >&2
    exit "${MOCK_API_LIMIT_EXIT:-0}"
    ;;
  'account balance --chain 8453 --output json --address 0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe --tag latest')
    printf '%s' "${MOCK_PAID_CHAIN_RESPONSE:-}"
    printf '%s' "${MOCK_PAID_CHAIN_ERROR:-}" >&2
    exit "${MOCK_PAID_CHAIN_EXIT:-0}"
    ;;
  *) exit 99 ;;
esac
EOF
  chmod +x "$MOCK_BIN/etherscan"
  export MOCK_API_LIMIT_RESPONSE='{"creditLimit":100000,"creditsUsed":3,"creditsAvailable":99997,"limitInterval":"daily","intervalExpiryTimespan":"23:59:59"}'
}

@test "Etherscan rejects an absent CLI without calling curl" {
  rm "$MOCK_BIN/etherscan"
  run "$ETHERSCAN"

  [ "$status" -eq 1 ]
  [ "$output" = "Error: etherscan CLI is not installed" ]
  [ ! -e "$MOCK_CURL_LOG" ]
}

@test "Etherscan maps unwrapped pretty Standard credits without an environment key or probe" {
  export MOCK_API_LIMIT_RESPONSE='{
    "creditLimit": 200000,
    "creditsUsed": 12,
    "creditsAvailable": 199988,
    "limitInterval": "daily",
    "intervalExpiryTimespan": "12:00:00"
  }'

  run env -u ETHERSCAN_API_KEY "$ETHERSCAN"

  [ "$status" -eq 0 ]
  [ "$output" = $'plan=standard\ncredit_limit=200000\ncredits_used=12\ncredits_available=199988\nlimit_interval=daily\ninterval_expiry=12:00:00\npro_endpoints=true\npaid_chains=true' ]
  [ "$(<"$MOCK_ETHERSCAN_LOG")" = 'apilimit --chain 1 --output json' ]
  [ ! -e "$MOCK_CURL_LOG" ]
}

@test "Etherscan preserves the credit plan mapping without paid-chain probes" {
  local limit expected
  for mapping in '500000 advanced' '1000000 professional' '1500000 pro_plus' '2000000 enterprise' '123456 unknown'; do
    read -r limit expected <<< "$mapping"
    export MOCK_API_LIMIT_RESPONSE="{\"creditLimit\":$limit,\"creditsUsed\":1,\"creditsAvailable\":1,\"limitInterval\":\"daily\",\"intervalExpiryTimespan\":\"00:00:00\"}"
    run "$ETHERSCAN"

    [ "$status" -eq 0 ]
    [[ "$output" == "plan=$expected"$'\n'* ]]
    if [ "$expected" = unknown ]; then
      [[ "$output" == *"pro_endpoints=unknown"* ]]
      [[ "$output" == *"paid_chains=unknown"* ]]
    else
      [[ "$output" == *"pro_endpoints=true"* ]]
      [[ "$output" == *"paid_chains=true"* ]]
    fi
  done
  [ "$(wc -l < "$MOCK_ETHERSCAN_LOG" | tr -d ' ')" -eq 5 ]
}

@test "Etherscan distinguishes Free from Lite through an explicit CLI denial" {
  export MOCK_PAID_CHAIN_EXIT=1
  export MOCK_PAID_CHAIN_ERROR='Error: API error: Free API access is not supported for this chain. Please upgrade your api plan for full chain coverage.'

  run "$ETHERSCAN"

  [ "$status" -eq 0 ]
  [[ "$output" == *$'plan=free\n'* ]]
  [[ "$output" == *"paid_chains=false"* ]]
  [ "$(wc -l < "$MOCK_ETHERSCAN_LOG" | tr -d ' ')" -eq 2 ]

  export MOCK_PAID_CHAIN_EXIT=0
  export MOCK_PAID_CHAIN_ERROR=''
  export MOCK_PAID_CHAIN_RESPONSE='"0"'
  run "$ETHERSCAN"

  [ "$status" -eq 0 ]
  [[ "$output" == *$'plan=lite\n'* ]]
  [[ "$output" == *"pro_endpoints=false"* ]]
  [[ "$output" == *"paid_chains=true"* ]]
  [ ! -e "$MOCK_CURL_LOG" ]
}

@test "Etherscan rejects malformed, missing, and invalid API-limit fields" {
  for response in \
    'not-json' \
    '{}' \
    '{"creditLimit":200000,"creditsUsed":12,"creditsAvailable":199988,"limitInterval":"daily"}' \
    '{"creditLimit":"200000","creditsUsed":12,"creditsAvailable":199988,"limitInterval":"daily","intervalExpiryTimespan":"12:00:00"}' \
    '{"creditLimit":200000,"creditsUsed":-1,"creditsAvailable":199988,"limitInterval":"daily","intervalExpiryTimespan":"12:00:00"}' \
    '{"creditLimit":200000,"creditsUsed":1.5,"creditsAvailable":199988,"limitInterval":"daily","intervalExpiryTimespan":"12:00:00"}' \
    '{"creditLimit":200000,"creditsUsed":12,"creditsAvailable":199988,"limitInterval":"daily\nplan=free","intervalExpiryTimespan":"12:00:00"}' \
    "$MOCK_API_LIMIT_RESPONSE $MOCK_API_LIMIT_RESPONSE"; do
    export MOCK_API_LIMIT_RESPONSE="$response"
    run "$ETHERSCAN"

    [ "$status" -eq 1 ]
    [ "$output" = "Error: etherscan apilimit returned invalid credit data" ]
  done
}

@test "Etherscan reports CLI authentication and request failures without leaking credentials" {
  export ETHERSCAN_API_KEY='test-secret-key'
  export MOCK_API_LIMIT_EXIT=1
  export MOCK_API_LIMIT_ERROR='Error: Get "https://api.etherscan.io/v2/api?apikey=test-secret-key": network failure'

  run "$ETHERSCAN"

  [ "$status" -eq 1 ]
  [ "$output" = "Error: etherscan apilimit failed. Check CLI credentials, quota, and network access" ]
  [[ "$(<"$MOCK_ETHERSCAN_LOG")" != *"test-secret-key"* ]]
  [[ "$(<"$MOCK_ETHERSCAN_LOG")" != *"--api-key"* ]]
  [ "$(wc -l < "$MOCK_ETHERSCAN_LOG" | tr -d ' ')" -eq 1 ]

  export MOCK_API_LIMIT_RESPONSE='{"creditLimit":200000,"creditsUsed":12,"creditsAvailable":199988,"limitInterval":"daily","intervalExpiryTimespan":"12:00:00"}'
  run env -u ETHERSCAN_API_KEY "$ETHERSCAN"

  [ "$status" -eq 1 ]
  [[ "$output" == *"Check CLI credentials"* ]]
  [[ "$output" != *"plan="* ]]
}

@test "Etherscan does not infer Free from quota, rate, credential, or transport failures" {
  export ETHERSCAN_API_KEY='test-secret-key'
  export MOCK_PAID_CHAIN_EXIT=1
  for error in \
    'Error: Community Free API limit reached. Resets 2026-10-06 00:00:00 UTC.' \
    'Error: Max rate limit reached, please use API Key for higher rate limit' \
    'Error: Invalid API Key' \
    'Error: Get "https://api.etherscan.io/v2/api?apikey=test-secret-key": network failure'; do
    export MOCK_PAID_CHAIN_ERROR="$error"
    run "$ETHERSCAN"

    [ "$status" -eq 0 ]
    [[ "$output" == *$'plan=unknown\n'* ]]
    [[ "$output" == *"paid_chains=unknown"* ]]
    [[ "$output" == *"pro_endpoints=false"* ]]
    [[ "$output" == *"probe inconclusive"* ]]
    [[ "$output" != *"test-secret-key"* ]]
  done
  [[ "$(<"$MOCK_ETHERSCAN_LOG")" != *"test-secret-key"* ]]
  [[ "$(<"$MOCK_ETHERSCAN_LOG")" != *"--api-key"* ]]
}

@test "Etherscan keeps malformed and non-quantity successful probes unknown" {
  for response in 'not-json' '0' '{}' '""' '"NOTOK"' '"0" "1"' '"Free API access is not supported for this chain."'; do
    export MOCK_PAID_CHAIN_RESPONSE="$response"
    run "$ETHERSCAN"

    [ "$status" -eq 0 ]
    [[ "$output" == *$'plan=unknown\n'* ]]
    [[ "$output" == *"paid_chains=unknown"* ]]
    [[ "$output" == *"pro_endpoints=false"* ]]
    [[ "$output" == *"probe inconclusive"* ]]
  done
}

@test "Etherscan discards partial success and stdout denial after a failed probe" {
  export MOCK_PAID_CHAIN_EXIT=1
  for response in '"0"' 'Free API access is not supported for this chain.'; do
    export MOCK_PAID_CHAIN_RESPONSE="$response"
    run "$ETHERSCAN"

    [ "$status" -eq 0 ]
    [[ "$output" == *$'plan=unknown\n'* ]]
    [[ "$output" == *"paid_chains=unknown"* ]]
    [[ "$output" == *"pro_endpoints=false"* ]]
  done
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
  [ "$output" = $'plan=builder\nrate_limit_rps=15\nrate_limit_remaining=9\nrate_limit_reset=42\ncredits_remaining=123' ]
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
  [ "$output" = $'chain_id=1\nname=Ethereum Mainnet\nnative_currency=ETH\ninstance_url=https://eth.blockscout.com/\napi_url=https://eth.blockscout.com/api\nhosted_by=blockscout\nis_testnet=false\nlayer=1\nrollup_type=' ]
}

@test "resolve-chain overrides Morph's stale Chainscout UI and API hosts" {
  export MOCK_CHAIN_RESPONSE='{"name":"Morph","native_currency":"ETH","explorers":[{"url":"https://explorer.morphl2.io/"}],"hostedBy":"self","isTestnet":false,"layer":2,"rollupType":"optimistic"}'

  run "$RESOLVE_CHAIN" 2818

  [ "$status" -eq 0 ]
  [[ "$output" == *"instance_url=https://explorer.morph.network/"* ]]
  [[ "$output" == *"api_url=https://explorer-api.morph.network/api"* ]]
  [[ "$output" != *"morphl2.io"* ]]
}

@test "resolve-chain rejects unsafe and out-of-scope target IDs before requesting Chainscout" {
  run "$RESOLVE_CHAIN" 250

  [ "$status" -eq 1 ]
  [[ "$output" == *"chain_id=250 is marked unsafe"* ]]
  [ ! -e "$MOCK_CURL_LOG" ]

  run "$RESOLVE_CHAIN" 5330

  [ "$status" -eq 1 ]
  [[ "$output" == *"Conduit Explorer"* ]]
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

@test "chain-lookup resolves an alias to key=value lines" {
  run "$CHAIN_LOOKUP" bnb

  [ "$status" -eq 0 ]
  [[ "$output" == $'chainId=56\nchainName=BNB Chain\nslug=bsc\n'* ]]
  [[ "$output" == *$'\nrouteMesh='* ]]
}

@test "chain-lookup resolves a case-insensitive alias name and a chain ID to the same row" {
  run "$CHAIN_LOOKUP" 'arbitrum one'
  [ "$status" -eq 0 ]
  local by_name="$output"

  run "$CHAIN_LOOKUP" 42161
  [ "$status" -eq 0 ]
  [ "$output" = "$by_name" ]
  [[ "$output" == $'chainId=42161\nchainName=Arbitrum\n'* ]]
  [[ "$output" == *"explorerTxUrl="* ]]
}

@test "chain-lookup flattens a scalar-only object field" {
  run "$CHAIN_LOOKUP" superseed

  [ "$status" -eq 0 ]
  [[ "$output" == *$'\ndefunct.since='* ]]
}

@test "chain-lookup --json prints one compact row" {
  run "$CHAIN_LOOKUP" 42161 --json

  [ "$status" -eq 0 ]
  [ "${#lines[@]}" -eq 1 ]
  [ "$(printf '%s' "$output" | jq -r '.chainId')" = "42161" ]
}

@test "chain-lookup reports a missing chain with near matches" {
  run "$CHAIN_LOOKUP" nosuchchain

  [ "$status" -eq 1 ]
  [[ "$output" == *"'nosuchchain' is not an evm-atlas target chain"* ]]

  run "$CHAIN_LOOKUP" 'Arbitrum Mainnet'

  [ "$status" -eq 1 ]
  [[ "$output" == *"Arbitrum (42161)"* ]]
}

@test "chain-lookup rejects missing arguments" {
  run "$CHAIN_LOOKUP"

  [ "$status" -eq 2 ]
  [[ "$output" == *"Usage: chain-lookup.sh"* ]]
}
