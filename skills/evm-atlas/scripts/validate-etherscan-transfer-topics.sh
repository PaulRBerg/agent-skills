#!/bin/bash
# Compatibility adapter for sweep-core.py validate.

set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: validate-etherscan-transfer-topics.sh <address> < response.json" >&2
  exit 2
fi

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
set +e
result=$(python3 "$script_dir/sweep-core.py" validate etherscan-transfer-topics --address "$1")
rc=$?
set -e
if [ "$rc" -ne 0 ]; then
  echo "Error: response does not prove distinct inbound-only and outbound-only Transfer topic OR semantics." >&2
  exit "$rc"
fi
printf '%s\n' "$result" | jq -r '
  "transfer_topic_query=conformant",
  "outbound_only_results=\(.outboundOnlyResults)",
  "inbound_only_results=\(.inboundOnlyResults)"
'
