#!/bin/bash
# Compatibility adapter for sweep-core.py validate.

set -eu

if [ "$#" -ne 0 ]; then
  echo "Usage: validate-blockscout-address-counters.sh < response.json" >&2
  exit 2
fi

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
set +e
result=$(python3 "$script_dir/sweep-core.py" validate blockscout-address-counters)
rc=$?
set -e
if [ "$rc" -ne 0 ]; then
  echo "Error: response is not a conformant Blockscout address-counters object." >&2
  exit "$rc"
fi
printf '%s\n' "$result" | jq -r '
  "transactions_count=\(.counts.transactions_count)",
  "token_transfers_count=\(.counts.token_transfers_count)",
  "gas_usage_count=\(.counts.gas_usage_count)",
  "validations_count=\(.counts.validations_count)"
'
