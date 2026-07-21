# Symbiosis Bridging

## Overview

Use Symbiosis Finance's public explorer API as a read-only source for cross-chain swap status after the known origin and
destination chains are confirmed against `references/generated/target-mainnets.json`. Symbiosis is a liquidity-network
aggregator: it does not move tokens directly between arbitrary chains but routes every cross-chain swap through
synthesized-token liquidity pools ("Octopools") on its own host chain, converting a source token to a synthetic
representation, moving that synthetic across the host chain, then converting to the requested destination token. A swap
record therefore has three legs: `from` (origin chain), `join` (host-chain leg), and `to` (destination chain).

Default to the explorer API base URL:

```text
https://api-v2.symbiosis.finance/explorer/v1
```

This is an unauthenticated public read API; no API key is documented or required.

Never execute bridge steps from this skill. Do not sign messages or submit swap transactions. Returned transaction and
route data are for inspection only.

## Read-Only Router

Use this router after the known origin and destination chains are confirmed against
`references/generated/target-mainnets.json`.

1. **Known source tx hash and its origin chain ID:** call `GET /transactions/<originChainId>/<txHash>` for the single
   matching record.
2. **Known source tx hash, chain ID unknown:** call `GET /transactions?search=<txHash>`; the search endpoint matches the
   hash regardless of leg or chain.
3. **Scoped browsing:** call `GET /transactions?limit=<n>` to page recent swaps; results are not filtered by chain or
   address in this form — narrow further only with parameters confirmed via a `search` match first.

Example direct chain/hash lookup:

```bash
curl -sS "https://api-v2.symbiosis.finance/explorer/v1/transactions/1/0xTX_HASH"
```

Example hash search:

```bash
curl -sS "https://api-v2.symbiosis.finance/explorer/v1/transactions?search=0xTX_HASH"
```

## Request Fields

| Field                                 | Use                                                            |
| ------------------------------------- | -------------------------------------------------------------- |
| `<originChainId>` / `<txHash>` (path) | Direct lookup: origin chain ID and the origin transaction hash |
| `search`                              | Free-text match against a transaction hash                     |
| `limit`                               | Page size for list-style queries                               |

## Report Fields

Extract and report these fields when present, either from a single object (direct lookup) or `records[]` (search /
list):

| Field                        | Symbiosis path examples                                                                  |
| ---------------------------- | ---------------------------------------------------------------------------------------- |
| Internal record ID           | `id`                                                                                     |
| Origin chain / tx            | `from_chain_id`, `from_tx_hash`                                                          |
| Host-chain (join) chain / tx | `join_chain_id`, `join_tx_hash`                                                          |
| Destination chain / tx       | `to_chain_id`, `to_tx_hash`                                                              |
| Sender / recipient           | `from_address`, `from_sender`, `to_address`, `to_sender`                                 |
| Tokens                       | `tokens[].symbol`, `tokens[].address`, `tokens[].decimals`                               |
| Route legs                   | `from_route[]`, `to_route[]` (each with `chain_id`, `amount`, `token`)                   |
| Amounts                      | `amounts[]` (parallel to `tokens[]`/route arrays)                                        |
| USD values                   | `from_amount_usd`, `to_amount_usd`                                                       |
| Timestamps                   | `created_at`, `mined_at`, `success_at`                                                   |
| Client/integrator            | `from_client_id` (e.g. `"symbiosis-app"`, or an aggregator name when routed through one) |
| Stuck / retry signal         | `state_stuck_reason`, `retry_active`                                                     |
| Lost-leg flags               | `from_is_lost`, `join_is_lost`, `to_is_lost`                                             |

Amounts in `amounts[]` and `from_route[]`/`to_route[]` are raw integer units in the token's smallest denomination;
convert with the accompanying `token.decimals`.

`from_client_id` shows the integrating frontend or aggregator that submitted the swap (observed live values include
`"symbiosis-app"` and third-party aggregator names such as `"lifi"`) — a Symbiosis-routed leg can appear while
investigating a different aggregator's transaction; check this field before assuming the immediate frontend is Symbiosis
itself.

## Status Values

The `state` field is an integer whose exact enum is not published in developer docs; live sampling was inconclusive
enough to assert a firm 1:1 mapping (records with the same `state` value showed a mix of populated and null
`success_at`, and some `state=2` records still eventually recorded a `success_at`). Prefer these directly observable
signals over the raw `state` code:

| Signal                                    | Meaning                                                                                                         |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `to_tx_hash` present and `success_at` set | Destination leg completed                                                                                       |
| `to_tx_hash` null and `success_at` null   | Swap has not completed the destination leg yet (in progress or stuck)                                           |
| `state_stuck_reason` non-empty            | The swap hit an execution error (e.g. gas estimation failure, below-minimum deposit); read the message directly |
| `retry_active` true                       | Symbiosis is actively retrying a failed step                                                                    |

Symbiosis's own user-facing documentation describes swap lifecycle states as In progress, Success, Success*,
Interrupted, and Reverted, and states that swaps stuck for too long are automatically reverted (tokens returned to the
sender) — this vocabulary is unverified against the explorer API's integer `state` field; treat it as background
context, not a field mapping to implement against.

## Failure Handling

- Empty `records` from `search`, or 404 from the direct chain/hash lookup: report that Symbiosis has no record for that
  hash and continue normal explorer/RPC analysis.
- `state_stuck_reason` non-empty: report the raw reason string; do not infer a specific remediation.
- `to_is_lost` / `join_is_lost` / `from_is_lost` true: report that Symbiosis itself flags that leg as unresolved; treat
  this as stronger signal than the raw `state` code.
- Non-target chains in `from_chain_id`/`to_chain_id`: report that the leg is outside this skill and ask for a feature
  request rather than continuing analysis on that leg.
- `join_chain_id` values reflect Symbiosis's internal host chain, not necessarily a chain tracked in
  `references/generated/target-mainnets.json`; do not treat it as a target chain requiring its own explorer
  verification.

## Sources

- https://docs.symbiosis.finance/developer-tools/symbiosis-api
- https://docs.symbiosis.finance/user-guide-webapp/symbiosis-explorer
- https://docs.symbiosis.finance/user-guide-webapp/where-are-my-tokens
- https://docs.symbiosis.finance/user-guide-webapp/stuck-transactions
- https://docs.symbiosis.finance/crosschain-liquidity-engine/symbiosis-octopools
- https://docs.symbiosis.finance/main-concepts/symbiosis-cross-chain-swaps
