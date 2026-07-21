# deBridge Bridging

## Overview

Use deBridge's DLN (deBridge Liquidity Network) as a read-only source for order-based cross-chain transfer status after
the known origin and destination chains are confirmed against `references/generated/target-mainnets.json`. DLN is an
order/intent protocol, not a lock-and-mint bridge: a maker places an order on the source chain (`DlnSource`), a taker
fills it on the destination chain (`DlnDestination`), and the source-side collateral is later unlocked to the taker.

Two hosts serve the same order data; either works, use whichever responds:

```text
https://dln.debridge.finance/v1.0    # simple, minimal-shape responses
https://dln-api.debridge.finance     # equivalent to stats-api.dln.trade; verbose typed-value response shape
```

Both are unauthenticated for reads; no API key is required or documented for these endpoints.

Never execute bridge steps from this skill. Do not sign messages, submit order-creation transactions, or broadcast any
returned calldata. Returned order and status data are for inspection only.

deBridge's internal chain IDs equal the real EVM chain ID for EVM chains (observed live: `1` for Ethereum, `8453` for
Base, `56` for BNB Chain) but diverge for non-EVM chains — Solana is internal ID `7565164`, not a real EVM chain ID. Do
not assume a deBridge chain ID is always the target EVM chain ID; cross-check non-EVM legs before reporting them.

## Read-Only Router

Use this router after the known origin and destination chains are confirmed against
`references/generated/target-mainnets.json`.

1. **Known source tx hash, order ID unknown:** call `GET /v1.0/dln/tx/<tx-hash>/order-ids` on `dln.debridge.finance` to
   resolve the order ID(s) created in that transaction. Empty `orderIds` means the hash did not create a DLN order (or
   is not yet indexed).
2. **Known order ID:** call `GET /v1.0/dln/order/<order-id>` on `dln.debridge.finance` for a compact struct with
   `status`, or `GET /v1.0/dln/order/<order-id>/status` for just the status string. Use `GET /api/Orders/<order-id>` on
   `dln-api.debridge.finance` for the full typed record including fulfillment and unlock event metadata.
3. **Need the full order lifecycle in one call:** prefer `dln-api.debridge.finance` — its response includes
   `createdSrcEventMetadata`, `fulfilledDstEventMetadata`, `sentUnlockDstEventInfo`, and `claimedUnlockSrcEventInfo`,
   each with a `transactionHash` when that stage has occurred.
4. **Known maker/taker address, no tx hash:** call `POST /api/Orders/filteredList` on `dln-api.debridge.finance` with a
   JSON body scoping by `giveChainIds`, `takeChainIds`, `orderStates`, `skip`, and `take`.

Example tx-hash to order-ID lookup:

```bash
curl -sS "https://dln.debridge.finance/v1.0/dln/tx/0xTX_HASH/order-ids"
```

Example compact order status:

```bash
curl -sS "https://dln.debridge.finance/v1.0/dln/order/0xORDER_ID/status"
```

Example full typed order record:

```bash
curl -sS "https://dln-api.debridge.finance/api/Orders/0xORDER_ID"
```

Example filtered list (Bash 3.2-compatible heredoc for the JSON body):

```bash
curl -sS -X POST "https://dln-api.debridge.finance/api/Orders/filteredList" \
  -H 'Content-Type: application/json' \
  -d '{"giveChainIds":[1],"takeChainIds":[8453],"orderStates":["Fulfilled"],"skip":0,"take":10}'
```

## Request Fields

| Field                    | Use                                                                       |
| ------------------------ | ------------------------------------------------------------------------- |
| `<tx-hash>` (path)       | Source transaction hash for the order-ids lookup                          |
| `<order-id>` (path)      | DLN order ID (0x-prefixed 32-byte hash), from order-ids or a prior lookup |
| `giveChainIds`           | Filter `filteredList` by source (give) chain deBridge ID                  |
| `takeChainIds`           | Filter `filteredList` by destination (take) chain deBridge ID             |
| `orderStates`            | Filter by order state (see Status Values)                                 |
| `maker` / `referralCode` | Filter `filteredList` by maker address or referral code                   |
| `skip` / `take`          | Pagination for `filteredList`; `take` must be greater than zero           |

## Report Fields

Extract and report these fields when present. Field paths below are from `dln-api.debridge.finance` /
`stats-api.dln.trade`'s typed-value shape, where each value is wrapped as `{bigIntegerValue, stringValue, ...}` — prefer
`stringValue` (or `bigIntegerValue` for chain IDs) for reporting.

| Field                          | Path examples                                                                                     |
| ------------------------------ | ------------------------------------------------------------------------------------------------- |
| Order ID                       | `orderId.stringValue`                                                                             |
| Maker (source)                 | `makerSrc.stringValue`                                                                            |
| Taker (destination)            | `taker.stringValue`                                                                               |
| Give (source) chain/token      | `giveOfferWithMetadata.chainId.bigIntegerValue`, `giveOfferWithMetadata.tokenAddress.stringValue` |
| Give amount                    | `giveOfferWithMetadata.amount.stringValue`, `.metadata.decimals`                                  |
| Take (destination) chain/token | `takeOfferWithMetadata.chainId.bigIntegerValue`, `takeOfferWithMetadata.tokenAddress.stringValue` |
| Take amount                    | `takeOfferWithMetadata.amount.stringValue`, `.actualFulfillAmount.stringValue`                    |
| Order state                    | `state`                                                                                           |
| Order creation tx              | `createdSrcEventMetadata.transactionHash.stringValue`                                             |
| Fulfillment tx                 | `fulfilledDstEventMetadata.transactionHash.stringValue`                                           |
| Unlock-sent tx (dest)          | `sentUnlockDstEventInfo.transactionMetadata.transactionHash.stringValue`                          |
| Unlock-claimed tx (source)     | `claimedUnlockSrcEventInfo.transactionMetadata.transactionHash.stringValue`                       |
| Fees                           | `percentFee.stringValue`, `finalPercentFee.stringValue`, `fixFee.stringValue`                     |

The `dln.debridge.finance/v1.0` host returns the same identifiers as plain JSON strings/numbers instead of the typed
wrapper (e.g. `orderIds: ["0x..."]`); use it when only the order ID is needed.

Give/take amounts are raw integer units in the token's smallest denomination; convert with the accompanying
`metadata.decimals`.

## Status Values

Interpret the `state` field as (per the official docs' terminal-state guidance):

| State                                                       | Meaning                                                  |
| ----------------------------------------------------------- | -------------------------------------------------------- |
| `Created`                                                   | Order placed on the source chain; not yet fulfilled      |
| `Fulfilled`                                                 | Taker delivered the take amount on the destination chain |
| `SentUnlock`                                                | Unlock message sent from destination back to source      |
| `ClaimedUnlock`                                             | Source-side collateral unlocked to the taker             |
| `OrderCancelled` / `SentOrderCancel` / `ClaimedOrderCancel` | Order cancelled and collateral returned to the maker     |

`Fulfilled`, `SentUnlock`, and `ClaimedUnlock` are all valid terminal-success states — the recipient already received
funds at `Fulfilled`; the later states only reflect internal solver settlement, not user-facing outcome. Live samples
observed `Fulfilled` orders with populated `fulfilledDstEventMetadata`, confirming destination delivery.

## Failure Handling

- Empty `orderIds` from the tx-hash lookup: report that deBridge has no order for that transaction (or it is not a DLN
  transaction) and continue normal explorer/RPC analysis.
- `take` must be greater than zero on `filteredList`: always pass an explicit positive `take`.
- Non-EVM chain IDs in results (e.g. `7565164` for Solana): report that the leg is outside this skill's EVM chain ID
  space and describe it in deBridge's own terms rather than misreporting it as an EVM chain ID.
- Non-target chains in deBridge results: report that the leg is outside this skill and ask for a feature request rather
  than continuing analysis on that leg.
- Order state stuck at `Created` past a normal fill window: report that fulfillment has not occurred yet; do not infer
  cancellation without an `OrderCancelled`/`SentOrderCancel`/`ClaimedOrderCancel` state.

## Sources

- https://docs.debridge.com/dln-the-debridge-liquidity-network-protocol/integration-guidelines/interacting-with-the-api/tracking-a-status-of-the-order
- https://docs.debridge.com/dln-details/integration-guidelines/order-creation/order-tracking-api/tracking-orders
- https://docs.debridge.com/dln-the-debridge-liquidity-network-protocol/deployed-contracts
- https://etherscan.io/address/0xef4fb24ad0916217251f553c0596f8edc630eb66
- https://etherscan.io/address/0xe7351fd770a37282b91d153ee690b63579d6dd7f
