#!/bin/bash
# Run the evm-atlas provider-response validators against synthetic fixtures.
# This test is offline: it performs no network requests and reads no credentials.

set -eu

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
fixture_dir="$script_dir/../fixtures"
example_address="0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"

bash "$script_dir/validate-blockscout-address-counters.sh" \
  < "$fixture_dir/blockscout-address-counters.json" >/dev/null
bash "$script_dir/validate-etherscan-transfer-topics.sh" "$example_address" \
  < "$fixture_dir/etherscan-transfer-topic-response.json" >/dev/null
if bash "$script_dir/validate-etherscan-transfer-topics.sh" "$example_address" \
  < "$fixture_dir/etherscan-transfer-topic-self-transfer.json" >/dev/null 2>&1; then
  echo "Error: a self-transfer incorrectly proved distinct inbound-only and outbound-only OR semantics." >&2
  exit 1
fi

echo "evm-atlas conformance fixtures are valid"
