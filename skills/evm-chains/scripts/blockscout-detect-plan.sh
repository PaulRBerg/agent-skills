#!/bin/bash
# blockscout-detect-plan.sh — Detect Blockscout PRO plan + credits from $BLOCKSCOUT_API_KEY.
#
# Reads the rate-limit/credit response headers the PRO host returns on every
# call. x-ratelimit-limit maps directly to the plan tier:
#   5 -> free, 15 -> standard, 30 -> pro
#
# Outputs key=value lines on stdout:
#   plan=<free|standard|pro|unknown>
#   rate_limit_rps=<int>
#   rate_limit_remaining=<int>
#   rate_limit_reset=<int seconds>
#   credits_remaining=<int>
#
# Costs ~20 credits (one address call). Cache the result for the session.

set -eu

if [ -z "${BLOCKSCOUT_API_KEY:-}" ]; then
  echo "Error: BLOCKSCOUT_API_KEY is not set" >&2
  echo "Get a free key at: https://dev.blockscout.com/" >&2
  exit 1
fi

# Any valid call returns the headers we need; use a known mainnet address.
addr="0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"
url="https://api.blockscout.com/1/api/v2/addresses/$addr"

headers=$(curl -fsS -D - -o /dev/null \
  -H "authorization: Bearer $BLOCKSCOUT_API_KEY" "$url" 2>/dev/null) || {
  echo "Error: request failed — invalid key or network issue (PRO host returns 401 for a bad key)." >&2
  exit 1
}

# Case-insensitive header lookup; strip trailing CR.
hval() {
  printf '%s' "$headers" | grep -i "^$1:" | head -1 \
    | sed 's/^[^:]*:[[:space:]]*//' | tr -d '\r'
}

rps=$(hval "x-ratelimit-limit")
remaining=$(hval "x-ratelimit-remaining")
reset=$(hval "x-ratelimit-reset")
credits=$(hval "x-credits-remaining")

case "$rps" in
  5)  plan="free" ;;
  15) plan="standard" ;;
  30) plan="pro" ;;
  *)  plan="unknown" ;;
esac

cat <<EOF
plan=$plan
rate_limit_rps=$rps
rate_limit_remaining=$remaining
rate_limit_reset=$reset
credits_remaining=$credits
EOF
