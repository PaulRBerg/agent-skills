# Symbiosis Bridging

## Overview

After confirming the known origin and destination chains against `references/generated/target-mainnets.json`, use
Symbiosis Finance's public explorer API. Use it as a read-only source for cross-chain swap status.

Symbiosis is a liquidity-network aggregator. It does not move tokens directly between arbitrary chains. It routes every
cross-chain swap through synthesized-token liquidity pools ("Octopools") on its own host chain.

Symbiosis converts a source token to a synthetic representation. It moves that synthetic across the host chain. Then it
converts it to the requested destination token. A swap record therefore has three legs: `from` (origin chain), `join`
(host-chain leg), and `to` (destination chain).

Default to the explorer API base URL:

```text
https://api-v2.symbiosis.finance/explorer/v1
```

This public read API allows unauthenticated requests. The documentation does not specify an API key. The API requires no
key.

Never execute bridge steps from this skill. Do not sign messages or submit swap transactions. Returned transaction and
route data are for inspection only.

## Read-Only Router

Use this router after confirming the known origin and destination chains against
`references/generated/target-mainnets.json`.

1. **Known source tx hash and its origin chain ID:** call `GET /transactions/<originChainId>/<txHash>` for the single
   matching record.
2. **Known source tx hash, chain ID unknown:** call `GET /transactions?search=<txHash>`. The search endpoint matches the
   hash regardless of leg or chain.
3. **Scoped browsing:** call `GET /transactions?limit=<n>` to page recent swaps. In this form, results have no chain or
   address filter. Narrow further only with parameters you first confirmed via a `search` match.

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

When present, extract these fields from a single object (direct lookup) or `records[]` (search / list). Report the
extracted fields:

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

Amounts in `amounts[]` and `from_route[]`/`to_route[]` are raw integer units in the token's smallest denomination.
Convert with the accompanying `token.decimals`.

`from_client_id` shows the integrating frontend or aggregator that submitted the swap. Observed live values include
`"symbiosis-app"` and third-party aggregator names such as `"lifi"`. A Symbiosis-routed leg can appear while you
investigate a different aggregator's transaction. Check this field before assuming the immediate frontend is Symbiosis
itself.

## Status Values

The `state` field is an integer. Developer documentation does not publish its exact enum. Live sampling did not
establish a firm 1:1 mapping. Records with the same `state` value showed a mix of populated and null `success_at`. Some
`state=2` records still eventually recorded a `success_at`. Prefer these directly observable signals over the raw
`state` code:

| Signal                                    | Meaning                                                                                                         |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `to_tx_hash` present and `success_at` set | Destination leg completed                                                                                       |
| `to_tx_hash` null and `success_at` null   | Swap has not completed the destination leg yet (in progress or stuck)                                           |
| `state_stuck_reason` non-empty            | The swap hit an execution error (e.g. gas estimation failure, below-minimum deposit). Read the message directly |
| `retry_active` true                       | Symbiosis is actively retrying a failed step                                                                    |

Symbiosis's own user-facing documentation describes swap lifecycle states as In progress, Success, Success*,
Interrupted, and Reverted. It states that swaps stuck for too long automatically revert and return tokens to the sender.
This vocabulary remains unverified against the explorer API's integer `state` field. Treat it as background context, not
a field mapping to implement against.

## Failure Handling

- Empty `records` from `search`, or 404 from the direct chain/hash lookup: report that Symbiosis has no record for that
  hash. Continue normal explorer/RPC analysis.
- `state_stuck_reason` non-empty: report the raw reason string. Do not infer a specific remediation.
- `to_is_lost` / `join_is_lost` / `from_is_lost` true: report that Symbiosis itself flags that leg as unresolved. Treat
  this as a stronger signal than the raw `state` code.
- Non-target chains in `from_chain_id`/`to_chain_id`: report that the leg is outside this skill. For that leg, ask for a
  feature request instead of continuing analysis.
- `join_chain_id` values reflect Symbiosis's internal host chain, not necessarily a chain tracked in
  `references/generated/target-mainnets.json`. Do not treat it as a target chain requiring its own explorer
  verification.

## Sources

- https://docs.symbiosis.finance/developer-tools/symbiosis-api
- https://docs.symbiosis.finance/user-guide-webapp/symbiosis-explorer
- https://docs.symbiosis.finance/user-guide-webapp/where-are-my-tokens
- https://docs.symbiosis.finance/user-guide-webapp/stuck-transactions
- https://docs.symbiosis.finance/crosschain-liquidity-engine/symbiosis-octopools
- https://docs.symbiosis.finance/main-concepts/symbiosis-cross-chain-swaps
