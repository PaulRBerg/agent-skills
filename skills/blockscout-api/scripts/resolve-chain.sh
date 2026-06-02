#!/bin/bash
# resolve-chain.sh — Resolve a chain_id to its Blockscout instance via Chainscout.
#
# Usage: resolve-chain.sh <chain_id>
#
# Outputs key=value lines on stdout:
#   chain_id=<int>
#   name=<string>
#   native_currency=<symbol>
#   instance_url=<url, ends in />
#   hosted_by=<blockscout|other>
#   is_testnet=<true|false>
#   layer=<int|>
#   rollup_type=<string|>
#
# No API key required. Source: https://chains.blockscout.com/

set -eu

if [ $# -lt 1 ] || [ -z "${1:-}" ]; then
  echo "Usage: resolve-chain.sh <chain_id>" >&2
  exit 1
fi

chain_id="$1"
resp=$(curl -fsS "https://chains.blockscout.com/api/chains/$chain_id" 2>/dev/null) || {
  echo "Error: Chainscout request failed for chain_id=$chain_id" >&2
  exit 1
}

sval() { printf '%s' "$resp" | grep -o "\"$1\":\"[^\"]*\"" | head -1 | sed 's/.*:"\(.*\)"/\1/'; }
nval() { printf '%s' "$resp" | grep -o "\"$1\":[0-9]*" | head -1 | grep -o '[0-9]*'; }
bval() { printf '%s' "$resp" | grep -oE "\"$1\":(true|false)" | head -1 | sed 's/.*://'; }

name=$(sval "name")
if [ -z "$name" ]; then
  echo "Error: chain_id=$chain_id not found in Chainscout" >&2
  exit 1
fi

native=$(sval "native_currency")
# "url" only appears inside explorers[]; the first is the primary explorer.
instance=$(printf '%s' "$resp" | grep -o '"url":"[^"]*"' | head -1 | sed 's/.*:"\(.*\)"/\1/')
hosted=$(sval "hostedBy")
testnet=$(bval "isTestnet")
layer=$(nval "layer")
rollup=$(sval "rollupType")

cat <<EOF
chain_id=$chain_id
name=$name
native_currency=$native
instance_url=$instance
hosted_by=$hosted
is_testnet=$testnet
layer=$layer
rollup_type=$rollup
EOF
