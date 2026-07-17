#!/bin/bash
# Validate a saved Etherscan Transfer-topic OR-query response from stdin.
# A conformant vector contains at least one inbound and one outbound result for
# the target. This helper performs no network requests and reads no credentials.

set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: validate-etherscan-transfer-topics.sh <address> < response.json" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is required." >&2
  exit 1
fi

address=$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')
if ! printf '%s\n' "$address" | grep -Eq '^0x[0-9a-f]{40}$'; then
  echo "Error: address must contain exactly 20 hex bytes." >&2
  exit 2
fi

transfer_topic="0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
address_topic="0x000000000000000000000000${address#0x}"
response=$(cat)

if ! printf '%s' "$response" | jq -e \
  --arg address_topic "$address_topic" \
  --arg transfer_topic "$transfer_topic" '
    (.status == "1") and
    (.result | type == "array" and length > 0) and
    all(.result[];
      try (
        (.topics | type == "array" and length >= 3) and
        ((.topics[0] | ascii_downcase) == $transfer_topic) and
        (
          ((.topics[1] | ascii_downcase) == $address_topic) or
          ((.topics[2] | ascii_downcase) == $address_topic)
        )
      ) catch false
    ) and
    any(.result[]; try ((.topics[1] | ascii_downcase) == $address_topic) catch false) and
    any(.result[]; try ((.topics[2] | ascii_downcase) == $address_topic) catch false)
  ' >/dev/null; then
  echo "Error: response does not prove conformant inbound-and-outbound Transfer topic OR semantics." >&2
  exit 1
fi

printf '%s' "$response" | jq -r \
  --arg address_topic "$address_topic" '
    "transfer_topic_query=conformant",
    "outbound_results=\([.result[] | select((.topics[1] | ascii_downcase) == $address_topic)] | length)",
    "inbound_results=\([.result[] | select((.topics[2] | ascii_downcase) == $address_topic)] | length)"
  '
