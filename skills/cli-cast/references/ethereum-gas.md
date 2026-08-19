# Ethereum Slow Gas

Use this policy for Ethereum mainnet transactions only. Rabby's keyless gas-market endpoint supplies the same `slow`,
`normal`, and `fast` tiers used by the wallet, including separate EIP-1559 max-fee and priority-fee values. It is a
public runtime API, not a documented third-party SLA; stop on an outage or schema change instead of substituting another
source.

## Fetch the Quote

Resolve `scripts/rabby-slow-gas.sh` relative to this skill and run it immediately before simulation and review:

```sh
RABBY_SLOW_QUOTE=$("$CLI_CAST_SKILL_DIR/scripts/rabby-slow-gas.sh")
RABBY_SLOW_MAX_FEE_WEI=$(printf '%s\n' "$RABBY_SLOW_QUOTE" | jq -er '.max_fee_per_gas_wei')
RABBY_SLOW_PRIORITY_FEE_WEI=$(printf '%s\n' "$RABBY_SLOW_QUOTE" | jq -er '.max_priority_fee_per_gas_wei')
RABBY_SLOW_ESTIMATED_SECONDS=$(printf '%s\n' "$RABBY_SLOW_QUOTE" | jq -er '.estimated_seconds')
RABBY_SLOW_QUOTED_AT=$(printf '%s\n' "$RABBY_SLOW_QUOTE" | jq -er '.quoted_at')
```

Set `CLI_CAST_SKILL_DIR` to the absolute directory containing this `SKILL.md`. The helper calls
`https://api.rabby.io/v2/wallet/gas_market`, selects exactly one `slow` entry, and rejects malformed, non-integer,
incoherent, or incorrectly ordered tiers. Do not reuse a quote for another transaction or a restarted review.

Ask `$evm-atlas` for one read packet containing the resolved Ethereum chain ID, latest block number and hash, and that
block's base fee. Require chain ID `1`, then bind the returned values locally:

```sh
test "$EVM_ATLAS_CHAIN_ID" = '1'
RABBY_SLOW_BLOCK="$EVM_ATLAS_BLOCK_NUMBER"
RABBY_SLOW_BASE_FEE_WEI="$EVM_ATLAS_BASE_FEE_WEI"
test "$RABBY_SLOW_MAX_FEE_WEI" -ge "$RABBY_SLOW_BASE_FEE_WEI"
```

If validation fails, fetch once more. Stop if the second result fails; do not fall back to `cast gas-price`,
`eth_gasPrice`, Normal, or Fast.

## Bind the Fees

Pass both values to simulation where supported and to every transaction-building or `cast send` command:

```sh
cast send "$CONTRACT" 'transfer(address,uint256)' "$TO" "$AMOUNT" \
  --rpc-url "$RPC_URL" \
  --gas-price "$RABBY_SLOW_MAX_FEE_WEI" \
  --priority-gas-price "$RABBY_SLOW_PRIORITY_FEE_WEI" \
  SIGNER_FLAGS
```

For EIP-1559 transactions, Cast interprets `--gas-price` as the max fee per gas and `--priority-gas-price` as the max
priority fee per gas. Start browser (`--browser`), encrypted-keystore, hardware-wallet, and environment-backed
private-key commands with the same fee pair. Use one reviewed signer suffix such as `--browser`,
`--keystore "$KEYSTORE_FILE"`, `--ledger`, or `--private-key "$ETH_PRIVATE_KEY"`; the signer preference and approval
rules in `SKILL.md` still apply. For browser signing, these flags are starting values: the user may deliberately edit
the gas limit or fee caps in the wallet confirmation UI, and the approved final wallet values govern the transaction. Do
not use `--legacy` for this policy.

For `cast publish`, inspect the signed transaction first and verify it already contains the approved fee pair; fees
cannot be changed after signing. The Rabby quote covers execution gas only, so a blob transaction still needs an
independently reviewed blob-gas price.

Immediately before broadcast, have `$evm-atlas` fetch the latest block and base fee again. If it exceeds the reviewed
max fee, the transaction is not currently includable: obtain a new Slow quote, simulate again, and present a revised
review. For replacements or cancellations, stop when Slow does not satisfy the required fee bump rather than silently
increasing the tier.
