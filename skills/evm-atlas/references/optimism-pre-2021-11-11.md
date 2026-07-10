# OP Mainnet Pre-Regenesis History

## Scope

Use this reference for OP Mainnet (`chain_id=10`) queries that target activity before the final regenesis on
`2021-11-11`.

Current OP Mainnet explorer and RPC routes are not authoritative for pre-final-regenesis transactions. Optimism's docs
state that transactions older than `2021-11-11` are not part of the current blockchain and do not appear on Etherscan.
The usual `evm-atlas` routes - Etherscan, Blockscout, and public RPC - can therefore fail even when the historical
activity happened.

## What Still Exists

For activity from `2021-06-23` through `2021-11-11`, Optimism points users to Dune:

- Use the OVM1.0 user address dashboard: `https://dune.com/optimismfnd/OVM1.0-User-Address-Transactions`
- For custom SQL, use Dune's `optimism_legacy_ovm1` schema.

For January-July 2021, Optimism says legacy L2Geth sequencer data directories were deleted during an August 2023
infrastructure cleanup. Those directories held transaction execution effects, especially emitted events and transaction
success or revert state. Final state, balances, and user assets were not lost, but intermediate balances and
event-derived transfer history may be unrecoverable from public indexed data.

The raw transaction inputs for that period were published to Ethereum's `CanonicalTransactionChain`. In principle, the
period can be reconstructed by downloading and re-executing the transaction chain, but Optimism documents this as labor
intensive, costly, and not guaranteed to fully recover the missing data.

## Agent Behavior

When a user asks for OP Mainnet pre-regenesis history:

1. State the limitation up front. Do not imply current OP Etherscan, Blockscout, or public RPC can recover canonical
   transaction receipts, logs, token transfers, NFT IDs, or tick/position identifiers from this era.
2. Search Dune/legacy OVM1 data when an address-level transaction record is enough.
3. Treat missing transaction hash, log, success state, token ID, Uniswap position ID, ticks, or exact transfer metadata
   as genuinely unknown unless found in a cited legacy source.
4. For accounting workflows, prefer explicit placeholder identifiers such as `unknown-op-pre-regenesis-position` plus a
   source note over invented NFT or transaction metadata.
5. Mention the `CanonicalTransactionChain` only as a deep-recovery path. Do not present it as a normal query fallback.

## Sources

- Optimism docs, "Accessing pre-regenesis history":
  `https://docs.optimism.io/node-operators/guides/management/regenesis-history`
- Optimism docs, "Block explorers": `https://docs.optimism.io/app-developers/tools/block-explorers`
