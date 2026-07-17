#!/bin/bash
# Validate a saved Blockscout v2 address-counters response from stdin.
# This helper performs no network requests and reads no credentials.

set -eu

if [ "$#" -ne 0 ]; then
  echo "Usage: validate-blockscout-address-counters.sh < response.json" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is required." >&2
  exit 1
fi

response=$(cat)

if ! printf '%s' "$response" | jq -e '
  def count_string:
    type == "string" and test("^(0|[1-9][0-9]*)$");
  type == "object" and
  (.transactions_count | count_string) and
  (.token_transfers_count | count_string) and
  (.gas_usage_count | count_string) and
  (.validations_count | count_string)
' >/dev/null; then
  echo "Error: response is not a conformant Blockscout address-counters object." >&2
  exit 1
fi

printf '%s' "$response" | jq -r '
  "transactions_count=\(.transactions_count)",
  "token_transfers_count=\(.token_transfers_count)",
  "gas_usage_count=\(.gas_usage_count)",
  "validations_count=\(.validations_count)"
'
