# Provider Routing

Read this reference only after resolving a chain in `target-mainnets.json`.

## Account and Transaction Data

1. If the chain ID is in `etherscan-chains.md`, use `etherscan-api.md` with `$ETHERSCAN_API_KEY`.
2. For Base (`8453`), Optimism (`10`), Avalanche (`43114`), and BNB Chain (`56`), run
   `scripts/etherscan-detect-plan.sh`. On a free plan, route to `blockscout-api.md` instead.
3. For target chains outside Etherscan coverage, use `blockscout-api.md` when listed.
4. If neither provider covers the target, use its listed public RPC for read-only JSON-RPC calls.

Do not infer API support from an Etherscan-shaped explorer URL.

## RouteMesh and Public RPC

Use RouteMesh only when the target row has `routeMesh: true` and `ROUTEMESH_API_KEY` is available:

```text
https://lb.routeme.sh/rpc/CHAIN_ID/ROUTEMESH_API_KEY
```

Verify current support through `https://api.routeme.sh/chains`. Otherwise verify the target's `primaryPublicRpc` with
`eth_chainId`, then try `target-fallback-rpcs.json` in order. Public RPCs are best-effort and may be rate limited.

## Explorer Links

Use the target row's `explorerUrl` plus `explorer-paths.json`. Verify nonstandard explorers in their UI; Ronin does not
reliably follow Etherscan paths and its chain ID collides with a non-target Chainscout entry.

## Exceptional History

For OP Mainnet data before `2021-11-11`, read `optimism-pre-2021-11-11.md` before interpreting provider or RPC results.
