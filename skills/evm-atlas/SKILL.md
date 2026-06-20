---
argument-hint: <chain-name-or-id>
disable-model-invocation: false
name: evm-atlas
user-invocable: true
description: 'Use for targeted EVM chain, account, transaction, RPC, explorer, and bridge data: chain name/ID, native symbol, RouteMesh, balances, token/NFT holdings/transfers, tx history, funding origin via Etherscan/Blockscout/Chainscout; Bungee, LayerZero, LI.FI, Socket enrichment.'
---

# EVM Atlas

Targeted EVM chain atlas (chain name, chain ID, public RPCs, native currency symbol, default block explorer URL, RouteMesh support) **and** a router for reading on-chain data: resolve the chain, then dispatch balance, token, transfer, transaction, and first-funding queries to Etherscan (preferred) or Blockscout (fallback). For bridge transactions and cross-chain swaps, enrich explorer/RPC verification with Bungee, LayerZero, or LI.FI data only when the relevant chains are target chains.

Use this skill to resolve chain metadata before reading from an RPC, sending transactions, calling contracts, constructing chain-specific RPC URLs, or building explorer links to addresses, transactions, or blocks — and to query on-chain account data once the chain is resolved (see [Querying On-Chain Data (Routing)](#querying-on-chain-data-routing)). Also use it when the user mentions bridging, bridge tx, cross-chain swap, Bungee, LayerZero, LI.FI, Socket, Stargate, OFT, CCTP, or Aori, or when a transaction is inferred to be bridge-related.

Match chains by displayed name or numeric chain ID. `./references/target-mainnets.json` is the skill's authoritative scope. If the requested chain is not listed there, do not use Etherscan, Blockscout, Bungee, LayerZero, LI.FI, RouteMesh, Chainlist, web search, or public RPCs to work around the scope. Ask the user to file a feature request in <https://github.com/PaulRBerg/agent-skills>.

Also normalize common code aliases from `./references/chain-aliases.json` to their target mainnet rows.

## Address-Wide Sweeps

When the user provides an EVM address and asks whether it was ever active on any chain, which chains it used, or whether it holds balances now across chains, read `./references/address-sweeps.md` before the single-chain router below. The sweep scope is still only `./references/target-mainnets.json`, not every provider-supported EVM chain.

If the prompt names a specific target chain, use [Querying On-Chain Data (Routing)](#querying-on-chain-data-routing) instead.

## Querying On-Chain Data (Routing)

To read account data — native balance, token holdings, ERC-20/721/1155 transfers, transaction history, or first-funding — resolve the chain, then dispatch to the right explorer API. Do not default to Ethereum; infer the chain from the prompt (explicit chain mention, chain-specific tokens like POL→137 / ARB→42161, target-chain aliases). If ambiguous, ask. If the resolved chain is not a target chain, ask for a feature request instead of querying.

1. **Resolve the chain** — map the name or alias to a chain ID via `./references/target-mainnets.json` and `./references/chain-aliases.json`.
2. **Etherscan (preferred)** — if the chain ID is listed in `./references/etherscan-chains.md`, follow `./references/etherscan-api.md` (unified API V2, needs `$ETHERSCAN_API_KEY`).
3. **Blockscout (fallback)** — for target chains not served by Etherscan, or paid Etherscan chains on a free Etherscan plan, follow `./references/blockscout-api.md`.
4. **Neither** — if the target chain is in neither registry, query the target chain's public RPC directly over JSON-RPC (`cast` from the `cli-cast` skill, or `curl`). If the chain is not a target chain, ask for a feature request instead of querying.

**Paid-chain auto-fallback.** Base (`8453`), Optimism (`10`), Avalanche (`43114`), and BNB Chain (`56`) are Etherscan-listed target chains, but their data endpoints require a paid Etherscan plan. If the target is one of these **and** `./scripts/etherscan-detect-plan.sh` reports `paid_chains=false` (free tier), route to Blockscout instead. Etherscan stays the default for every other Etherscan-listed target chain.

## Bridge Data Enrichment

When the user mentions bridging, bridge tx, cross-chain swap, or a bridge provider, or when a transaction looks bridge-related from logs, counterparties, calldata, or token movement, first confirm the known origin and destination chains are target chains. If either known chain is outside `./references/target-mainnets.json`, ask for a feature request instead of continuing.

Then route to the bridge reference that matches the prompt or evidence:

- **Bungee / Socket / Bungee Auto:** read `./references/bungee-api.md`.
- **LayerZero / Stargate / OFT / CCTP / Aori / Value Transfer API / LayerZeroScan:** read `./references/layerzero-bridging.md`.
- **LI.FI / LiFi / li.quest / LI.FI Explorer:** read `./references/lifi-bridging.md`.

Use bridge APIs as read-only enrichment sources alongside Etherscan, Blockscout, explorers, and RPC receipts. Do not treat bridge APIs as authoritative for on-chain execution by themselves; verify submitted transactions and terminal outcomes with explorer/RPC data whenever possible. Returned route steps, calldata, `userSteps`, `transactionRequest`, or signatures are for inspection only in this skill. Do not sign messages, submit signatures, execute user steps, or broadcast bridge transactions.

## RouteMesh

Use RouteMesh only when the `routeMesh` field in `./references/target-mainnets.json` is `true` and the `ROUTEMESH_API_KEY` environment variable is available.

To verify current RouteMesh support, call `GET https://lb.routeme.sh/chains`; use `https://lb2.routeme.sh/chains` as the backup endpoint. Do not use `https://rpc.routeme.sh/chains`; the hostname may not resolve even though it appears in RouteMesh's OpenAPI spec.

Construct the RouteMesh RPC URL as:

```text
https://lb.routeme.sh/rpc/CHAIN_ID/ROUTEMESH_API_KEY
```

Replace `CHAIN_ID` with the numeric chain ID and `ROUTEMESH_API_KEY` with the value of the `ROUTEMESH_API_KEY` environment variable. If `routeMesh` is `false` or `ROUTEMESH_API_KEY` is not available, use the chain's primary public RPC first, then the listed fallback RPCs in order.

## Public RPCs

Public RPCs are best-effort. Before relying on one for data fetches or contract calls, verify it with `eth_chainId`. If the primary endpoint fails, try the fallback endpoints for that chain from `./references/target-fallback-rpcs.json` in order. If a chain has no fallback row, only the primary public RPC is listed.

Only use RPCs for target chains. For any chain not in `./references/target-mainnets.json`, ask the user to file a feature request in <https://github.com/PaulRBerg/agent-skills>.

## Explorer URLs

The `explorerUrl` field in `./references/target-mainnets.json` is the base URL of the chain's canonical block explorer. Append the path patterns from `./references/explorer-paths.json` to build links.

Etherscan and Etherscan-stack explorers (Arbiscan, Basescan, BscScan, Polygonscan, Optimism Etherscan, Lineascan, Snowscan, Blastscan, Berascan, Uniscan, Gnosisscan, abscan.org) all follow this scheme. Most Blockscout-based and chain-native target explorers accept the same segments, but conventions can drift — verify against the explorer UI when in doubt.

Explorer URL presence, Etherscan-style paths, or an Etherscan-stack explorer name do not imply Etherscan API V2 support. When API coverage matters, use `./references/etherscan-chains.md`. Etherscan's live `https://api.etherscan.io/v2/chainlist` endpoint may contain additional provider-supported chains, but those are outside this skill unless they appear in `./references/target-mainnets.json`. Treat this JSON as chain metadata for RPC and explorer-link construction only.

## Caveats

**OP Mainnet pre-regenesis history is not available through current explorer/RPC routes.** For OP Mainnet (`10`) queries before the final regenesis on `2021-11-11`, read `./references/optimism-pre-2021-11-11.md` before using Etherscan, Blockscout, or public RPC results.

**Ronin (`app.roninchain.com`) does not follow the Etherscan path scheme.** Verify against the explorer UI before constructing a Ronin link.

**Ronin (`2020`) collides with a non-target Chainscout registry entry.** Do not use Chainscout metadata for Ronin; use Ronin's listed explorer/RPC or another target-aware source.

## Target Mainnets

Target mainnet metadata lives in `./references/target-mainnets.json`. The `slug` field is the Sablier SDK chain slug (`sablier` package `~/sablier/sdk`, `src/evm/chains/specs.ts`, resolved as `meta.slug ?? key`). Arbitrum Nova, Celo, Fantom, IoTeX, and Zora are not defined in the SDK; their slugs follow the same convention (lowercase, hyphenated name).

## Target Fallback RPCs

Target fallback RPC metadata lives in `./references/target-fallback-rpcs.json`. Use each chain's `fallbackPublicRpcs` values in order after the `primaryPublicRpc` from `./references/target-mainnets.json` fails verification.
