# Hop Bridging

## Overview

Use Hop as a read-only enrichment source for legacy Hop Protocol v1 transfers: matching a source-chain send to its
bonded destination-chain withdrawal. Hop's bridge volume is largely historical — treat this reference as relevant mainly
for enriching older transfers, not for current active-route discovery.

Default to the public API base URL:

```text
https://api.hop.exchange
```

No API key is documented or required for these read endpoints.

Never execute bridge steps from this skill. Do not sign messages, submit `sendToL2`/`send`/`swapAndSend` transactions,
or call any bonder-only method (`bondWithdrawal`, `bondWithdrawalAndDistribute`, etc.). Returned quote and transfer data
are for inspection only.

Hop v1 uses a bonded-liquidity model, not a lock-and-mint bridge: a user sends into a source-chain bridge/AMM-wrapper
contract, and a bonder immediately fronts the equivalent output on the destination chain from its own inventory, settled
later via the canonical L1<->L2 messaging path once the source-chain root bundle finalizes. The report-relevant
destination event is a bonder-funded withdrawal, not a mint.

## Read-Only Router

Use this router after the known origin and destination chains are confirmed against
`references/generated/target-mainnets.json`.

1. **Known source tx hash or transfer ID:** call `GET /v1/transfer-status?transactionHash=<hash>` (or `?transferId=<id>`
   if a transfer ID rather than a tx hash is known — the two params are mutually alternative, and at least one is
   required).
2. **Available routes:** call `GET /v1/available-routes` to check which token/source-chain/destination-chain
   combinations Hop supports; optionally filter with `network` (default `mainnet`).
3. **Bonder fee quote:** call `GET /v1/quote` with `amount`, `token`, `fromChain`, `toChain`, and `slippage` for a
   current bonder-fee estimate; not useful for historical transfer enrichment.

Example status lookup:

```bash
curl -sS "https://api.hop.exchange/v1/transfer-status?transactionHash=0xTX_HASH"
```

Example available-routes:

```bash
curl -sS "https://api.hop.exchange/v1/available-routes"
```

## Request Fields

| Field                   | Use                                                                      |
| ----------------------- | ------------------------------------------------------------------------ |
| `transactionHash`       | Source-chain send transaction hash; alternative to `transferId`          |
| `transferId`            | Hop-assigned transfer ID; alternative to `transactionHash`               |
| `network`               | Optional; `mainnet` (default) vs a testnet name                          |
| `amount`                | Quote-only: source amount in the token's smallest units                  |
| `token`                 | Quote-only: token symbol                                                 |
| `fromChain` / `toChain` | Quote-only: source/destination chain slugs (e.g. `ethereum`, `arbitrum`) |
| `slippage`              | Quote-only: slippage tolerance                                           |

Do not invent wallet addresses. Use only user-provided or known on-chain addresses.

## Report Fields

`GET /v1/transfer-status` documents these fields for a resolved transfer: source and destination chain, sent/received
amounts, bonder address, and bonded status. This reference cannot confirm the exact JSON path names live — the API's
backing indexer (`explorer-api.hop.exchange`) was returning `503 Service Temporarily Unavailable` on every call during
verification (see Failure Handling), so no live successful `transfer-status` response body was observed. Treat the
following as unverified-from-docs and confirm the actual response shape once the indexer is reachable again:

| Field (unverified path)   | Expected content                                      |
| ------------------------- | ----------------------------------------------------- |
| Source/destination chain  | Chain slugs or IDs for the sent leg                   |
| Sent amount               | Source-chain amount, likely raw smallest-unit integer |
| Bonder address            | Address that fronted the destination withdrawal       |
| Bonded / recipient status | Whether the bonder has withdrawn on the destination   |

`GET /v1/available-routes` is verified live and returns an array of
`{token, sourceChainSlug, sourceChainId, destinationChainSlug, destinationChainId}` rows.

## Bonder Model and AMM Wrapper

Hop v1 routes L1<->L2 and L2<->L2 sends through a per-chain `L2_AmmWrapper` (and a plain `L2_Bridge`) contract per
token. The Arbitrum One `L2_AmmWrapper` is at `0x33ceb27b39d2Bb7D2e61F7564d3Df29344020417` (verified on-chain, labeled
`L2_AmmWrapper` on block explorers). The AMM wrapper swaps the canonical bridge token (`h<TOKEN>`) against the native
token via Hop's AMM on send/receive, so a single user-facing transfer can appear on-chain as a wrapper swap plus a
bridge send rather than a single bridge call. Resolve other chains' bridge/wrapper addresses from
`references/generated/target-mainnets.json` or the chain's block explorer rather than hardcoding a full table here.

A destination-chain withdrawal is bonded (fronted immediately by a bonder, pending later settlement) unless the bonder
has insufficient available liquidity, in which case the recipient must wait for the slower canonical-message-based
withdrawal instead. Both paths are legitimate terminal successes; only the timing and the presence of a bonder address
differ.

## Status Values

`GET /v1/transfer-status` is documented to report a resolved/bonded boolean-style state rather than a fixed status enum;
this could not be live-confirmed (see Report Fields). Treat any status text returned as informational and verify the
destination withdrawal directly against explorer or RPC data.

## Failure Handling

- `explorer-api.hop.exchange` outage (verified 2026-07-21): repeated `GET /v1/transfers` and `GET /v1/transfer-status`
  calls returned `503 Service Temporarily Unavailable` across multiple retries over several minutes. `api.hop.exchange`
  itself responds and validates request params (missing `transactionHash`/`transferId` returns
  `{"error":"transferId or transactionHash is required"}`), but any request that reaches the backing indexer fails with
  `{"error":"fetchJsonOrThrow error: ... is not valid JSON ..."}` regardless of whether the hash/ID is real or bogus.
  When this recurs, report the outage and fall back to decoding the source-chain send event and destination-chain
  bond/withdrawal event directly from explorer or RPC logs.
- Missing both `transactionHash` and `transferId`: the API returns
  `{"error":"transferId or transactionHash is required"}`; supply one.
- No route found in `/available-routes`: report that Hop does not support the pair; do not try alternative Hop
  chains/tokens unless the user asks.
- Non-target chains in Hop results: report that the leg is outside this skill and ask for a feature request rather than
  continuing analysis on that leg.
- Given Hop's largely historical volume, absence of a live indexer response is not itself evidence a transfer failed —
  confirm terminal state via explorer/RPC before reporting failure.

## Sources

- https://docs.hop.exchange/developer-docs/api/api
- https://github.com/hop-protocol/hop
- https://github.com/hop-protocol/explorer (archived; superseded by the hop-protocol/hop monorepo)
- https://explorer.hop.exchange/
