#!/bin/bash
set -euo pipefail

command -v curl >/dev/null 2>&1 || {
  echo 'rabby-slow-gas: curl is required' >&2
  exit 1
}
command -v jq >/dev/null 2>&1 || {
  echo 'rabby-slow-gas: jq is required' >&2
  exit 1
}

endpoint='https://api.rabby.io/v2/wallet/gas_market'
response=$(
  curl --fail --silent --show-error \
    --connect-timeout 5 \
    --max-time 10 \
    --retry 2 \
    --retry-delay 1 \
    --request POST \
    --header 'content-type: application/json' \
    --data '{"chain_id":"eth"}' \
    "$endpoint"
)
quoted_at_unix=$(date +%s)
quoted_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

printf '%s\n' "$response" | jq -ce \
  --arg endpoint "$endpoint" \
  --arg quoted_at "$quoted_at" \
  --argjson quoted_at_unix "$quoted_at_unix" '
  def one_tier($name):
    [.[] | select(.level == $name)] as $matches
    | if ($matches | length) == 1 then $matches[0]
      else error("expected exactly one \($name) tier")
      end;
  def valid_tier:
    (.price | type) == "number"
    and (.price | floor) == .price
    and .price > 0
    and (.priority_price | type) == "number"
    and (.priority_price | floor) == .priority_price
    and .priority_price >= 0
    and .priority_price <= .price
    and (.estimated_seconds | type) == "number"
    and (.estimated_seconds | floor) == .estimated_seconds
    and .estimated_seconds >= 0;

  if type != "array" then error("expected a tier array") else . end
  | . as $tiers
  | ($tiers | one_tier("slow")) as $slow
  | ($tiers | one_tier("normal")) as $normal
  | ($tiers | one_tier("fast")) as $fast
  | if (($slow | valid_tier) and ($normal | valid_tier) and ($fast | valid_tier))
      and $slow.price <= $normal.price
      and $normal.price <= $fast.price
      and $slow.priority_price <= $normal.priority_price
      and $normal.priority_price <= $fast.priority_price
    then {
      endpoint: $endpoint,
      chain_id: 1,
      chain_server_id: "eth",
      tier: "slow",
      max_fee_per_gas_wei: ($slow.price | tostring),
      max_priority_fee_per_gas_wei: ($slow.priority_price | tostring),
      estimated_seconds: $slow.estimated_seconds,
      quoted_at_unix: $quoted_at_unix,
      quoted_at: $quoted_at
    }
    else error("invalid or incorrectly ordered gas tiers")
    end
  '
