#!/bin/bash
# chain-lookup.sh - Resolve one evm-atlas target chain by name, alias, slug, or chain ID.
#
# Usage: chain-lookup.sh <name|alias|slug|chain_id> [--json]
#
# Matching is case-insensitive against chainName, slug, numeric chainId, and every alias in
# chain-aliases.json. Only rows in target-mainnets.json can match.
#
# Default output: key=value lines on stdout, in this order:
#   chainId, chainName, slug, category, nativeCurrencySymbol, primaryPublicRpc,
#   accountActivityModel, explorerUrl, explorerTxUrl, explorerAddressUrl, routeMesh,
#   then any optional keys in sorted order (for example defunct, explorerApiUrl).
# An object value with only scalar members prints as <key>.<member>=<value> lines
# (for example defunct.since=2026-06-25). A deeper object or an array prints as <key>=<compact JSON>.
# --json prints the full target row as one compact JSON line.
#
# Exit codes: 0 found, 1 not found or ambiguous (one stderr line with up to five near matches),
# 2 usage error or missing jq.
#
# Data sources (relative to this script, maintained by the evm-atlas generator):
#   ../references/generated/target-mainnets.json
#     {"chains":[{"chainId":42161,"chainName":"Arbitrum","slug":"arbitrum","category":"...",
#       "accountActivityModel":"...","nativeCurrencySymbol":"ETH","primaryPublicRpc":"https://...",
#       "explorerUrl":"https://...","explorerTxUrl":"https://.../tx/{tx_hash}",
#       "explorerAddressUrl":"https://.../address/{address}","routeMesh":true,
#       optional "defunct":{"since":"YYYY-MM-DD","finalStateBlock":N}, optional "explorerApiUrl":"https://..."}]}
#   ../references/generated/chain-aliases.json
#     {"aliases":[{"alias":"bsc","chainName":"BNB Chain","chainId":56}]}

set -eu
set -o pipefail

usage() {
  echo "Usage: chain-lookup.sh <name|alias|slug|chain_id> [--json]" >&2
  exit 2
}

query=""
json_output=false
for arg in "$@"; do
  case "$arg" in
    --json) json_output=true ;;
    -h | --help) usage ;;
    *)
      [ -z "$query" ] || usage
      query="$arg"
      ;;
  esac
done
[ -n "$query" ] || usage

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is required" >&2
  exit 2
fi

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
data_dir="$script_dir/../references/generated"
targets="$data_dir/target-mainnets.json"
aliases="$data_dir/chain-aliases.json"

matches=$(jq -c --arg q "$query" --slurpfile aliases "$aliases" '
  def norm: ascii_downcase | gsub("^\\s+|\\s+$"; "");
  ($q | norm) as $n
  | [$aliases[0].aliases[] | select((.alias | norm) == $n) | .chainId] as $alias_ids
  | [.chains[] | select(
      (.chainName | norm) == $n
      or (.slug | norm) == $n
      or (.chainId | tostring) == $n
      or (.chainId as $id | any($alias_ids[]; . == $id))
    )]
' "$targets")

count=$(printf '%s' "$matches" | jq 'length')

if [ "$count" -eq 0 ]; then
  near=$(jq -r --arg q "$query" --slurpfile aliases "$aliases" '
    def norm: ascii_downcase | gsub("^\\s+|\\s+$"; "");
    ($q | norm) as $n
    | [(.chains[] | {name: .chainName, id: .chainId}),
       ($aliases[0].aliases[] | {name: .alias, id: .chainId})]
    | map(select((.name | norm) as $m
        | ($m | contains($n)) or (($m | length) >= 3 and ($n | contains($m)))))
    | unique_by(.id) | .[:5] | map("\(.name) (\(.id))") | join(", ")
  ' "$targets")
  echo "Error: '$query' is not an evm-atlas target chain. Near matches: ${near:-none}" >&2
  exit 1
fi

if [ "$count" -gt 1 ]; then
  names=$(printf '%s' "$matches" | jq -r '.[:5] | map("\(.chainName) (\(.chainId))") | join(", ")')
  echo "Error: '$query' matches more than one target chain: $names" >&2
  exit 1
fi

if [ "$json_output" = true ]; then
  printf '%s' "$matches" | jq -c '.[0]'
  exit 0
fi

printf '%s' "$matches" | jq -r '
  .[0] as $r
  | ["chainId", "chainName", "slug", "category", "nativeCurrencySymbol", "primaryPublicRpc",
     "accountActivityModel", "explorerUrl", "explorerTxUrl", "explorerAddressUrl", "routeMesh"] as $order
  | ($order + (($r | keys) - $order))[] as $k
  | select($r | has($k))
  | $r[$k] as $v
  | if ($v | type) == "object" and all($v[]; type != "object" and type != "array") then
      $v | to_entries[] | "\($k).\(.key)=\(.value)"
    elif ($v | type) == "object" or ($v | type) == "array" then
      "\($k)=\($v | tojson)"
    else
      "\($k)=\($v)"
    end
'
