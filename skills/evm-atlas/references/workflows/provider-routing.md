# Provider Routing

Read this reference only after resolving a chain in `references/generated/target-mainnets.json`.

## Account and Transaction Data

Choose one authoritative history provider per chain and sweep. A provider is authoritative for that result, not a
globally canonical source. Keep a second provider only as a fallback:

1. Use Blockscout on covered overlaps, especially when Etherscan cannot serve the chain on the detected plan, its
   pagination/rate/PRO limits make the requested sweep less complete, or Blockscout's native holdings/counters avoid
   those limits.
2. Otherwise use Etherscan V2 when the chain is in `references/generated/etherscan-chains.md`, the detected plan can
   query it, and the needed actions accept the fixed cutoff.
3. Use the other indexed provider as fallback when the authoritative provider is unavailable, malformed, behind the
   cutoff, rate/plan limited, or missing a required action. A valid empty response is a completed negative, not a
   fallback trigger. Move the affected result to the fallback; do not silently splice two negative responses into one
   complete result.
4. If neither indexed provider covers the target, use its listed public RPC only for facts that JSON-RPC can prove.
   Missing indexed history remains unknown, never empty.

On overlaps, Blockscout is not automatically secondary. In particular, prefer it for Base (`8453`), Optimism (`10`),
Avalanche (`43114`), and BNB Chain (`56`) when `scripts/etherscan-detect-plan.sh` reports `paid_chains=false`, and when
its unmetered per-instance or full-holdings routes are materially more complete than the available Etherscan plan. Do
not infer API support from an Etherscan-shaped explorer URL.

For raw Etherscan V2 endpoint parameters, plan gating, and error handling, see `references/explorers/etherscan-api.md`.
For raw Blockscout endpoint parameters, plan gating, and error handling, see `references/explorers/blockscout-api.md`.

## Checkpoints and State

Fix one required ISO-8601 UTC cutoff for the whole sweep. Resolve it once per chain to an exact finalized or otherwise
independently verified block at or before that time. Record the requested cutoff, resolution kind (`finalized` or
`verified`), block number, hash, timestamp, and observation time. For a timestamp lookup, prove that the returned `B` is
the greatest block at or before the cutoff by checking `B.timestamp <= requestedAt` and either
`B+1.timestamp > requestedAt` or that `B` is independently the current finalized head. Reuse that exact checkpoint in
every request. Do not mix `latest`, different provider heads, or a newly resolved block into the same result. If no
route can establish the checkpoint, mark the result unknown rather than inventing one.

Batch `eth_getTransactionCount` and `eth_getBalance` with the EIP-1898 `{ blockHash, requireCanonical: true }` selector
before indexed history. If a provider rejects that selector, a numeric fallback requires matching block-number/hash
headers from the same endpoint immediately before and after the batch; otherwise try the next RPC or report unknown. The
target row's `accountActivityModel` controls whether zero nonce plus zero balance may satisfy a profile's native-history
shortcut:

- Allow the shortcut only for exact `ethereum-eoa`.
- Default-deny it for `native-account-abstraction`, `cross-vm`, `unknown`, a missing field, or an unrecognized value.
- Under the prb-finance bootstrap profile, the exact `ethereum-eoa` zero-state invariant may omit both `txlist` and
  `txlistinternal` wholesale. That profile counts a successful outgoing normal row or a successful positive-value
  normal/internal row touching the address; zero-value inbound normal/internal noise is outside it. The invariant never
  covers token/NFT transfers. Apply the profile rules in `references/workflows/address-sweeps.md` before calling an
  address inactive; a general policy that counts zero-value calls must still query those channels.

For `cross-vm`, scope all state, history, and negative claims to the chain's EVM execution environment. EVM evidence
does not cover the native non-EVM account environment and cannot prove whole-chain inactivity.

An indexer result is cutoff-complete only when the provider is synced through the checkpoint and the query is bounded to
it. Filter or paginate past post-cutoff rows; an unbounded newest-first empty/non-empty page is not equivalent to a
checkpointed result.

Quorum is optional and must be explicit. When requested, enforce it strictly across independent indexed providers that
cover the same checkpoint and channel set. PRO and per-instance Blockscout surfaces backed by the same index are one
provider. Descending one-row probes establish existence only. For a positive quorum, every provider must query every
required channel ascending from genesis or fully paginate its bounded result, apply the same profile predicates, and
return the same earliest qualifying transaction hash, block, action/channel, and timestamp. A negative quorum requires
valid empty coverage from every provider. Errors and unsupported channels are not votes; never weaken the requested
quorum, and report disagreement as unknown.

## RouteMesh and Public RPC

Use RouteMesh only when the target row has `routeMesh: true` and `ROUTEMESH_API_KEY` is available:

```text
https://lb.routeme.sh/rpc/CHAIN_ID/ROUTEMESH_API_KEY
```

Verify current support through `https://api.routeme.sh/chains`. Otherwise verify the target's `primaryPublicRpc` with
`eth_chainId`, then try `references/generated/target-fallback-rpcs.json` in order. Public RPCs are best-effort and may
be rate limited.

## Explorer Links

Use the target row's `explorerUrl` plus `references/explorers/explorer-paths.json`. Verify nonstandard explorers in
their UI; Ronin does not reliably follow Etherscan paths and its chain ID collides with a non-target Chainscout entry.
Ronin's explorer (`app.roninchain.com`) also blocks scripted access, so open it with `chrome-devtools`/Chromium rather
than `curl` or `WebFetch`, the same way `references/workflows/blockscan-balances.md` requires Chromium for Blockscan.

## Exceptional History

For OP Mainnet data before `2021-11-11`, read `references/explorers/optimism-pre-regenesis.md` before interpreting
provider or RPC results.
