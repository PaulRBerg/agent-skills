# Relay Bridging

## Overview

After confirming the known origin and destination chains against `references/generated/target-mainnets.json`, use Relay
(relay.link) as a read-only source. Use it for cross-chain request status, solver-fill details, and route metadata.

Default to the API base URL:

```text
https://api.relay.link
```

Relay's public API works without a key at standard rate limits. The documentation does not specify an API-key header for
read endpoints.

Never execute bridge steps from this skill. Do not sign messages, submit deposit transactions, or broadcast a returned
`depositAddress` or `inTxs[].data` as a transaction target. Returned request, deposit, and fill data are for inspection
only.

Relay uses a solver-fill model, not a lock-and-mint bridge. The user sends funds to a per-request deposit address or an
existing shared receiver contract on the origin chain. A Relay solver fills the equivalent amount on the destination
chain. The deposit later reimburses the solver. Two addresses recur across requests and are not user-specific:

- `0xf70da97812CB96acDF810712Aa562db8dfA3dbEF` — Etherscan-labeled "Relay: Solver", Relay's primary solver wallet that
  executes destination-side fills across many chains. It appears as `protocol.solver.address` in request records.
- `0xa5F565650890fBA1824Ee0F21EbBbF660a179934` — Etherscan-labeled "Reservoir: Relay Receiver", a shared RelayReceiver
  contract deployed at the same address on multiple chains that forwards deposits and can execute calls on a solver's
  behalf. Do not attribute a transfer to a specific user solely because it touches this address.

Per-request deposit addresses (`protocol.deposit.origin.depository`) are distinct from the shared receiver above and are
typically unique per request. Do not assume they are stable across requests.

## Read-Only Router

Use this router after confirming the known origin and destination chains against
`references/generated/target-mainnets.json`.

1. **Known source or destination tx hash:** call `GET /requests/v2?hash=<tx-hash>`. Relay indexes this against both
   `inTxs[].hash` (origin deposit) and `outTxs[].hash` (destination fill).
2. **Known sender/recipient address, no tx hash:** call `GET /requests/v2?user=<address>&limit=20`. Narrow with
   `chainId` when only one leg's chain is known.
3. **Known request or order ID:** call `GET /requests/v2?id=<request-id>` or `GET /requests/v2?orderId=<order-id>` when
   known, e.g. from a Relay explorer link. `id` is a Relay-internal request identifier distinct from both the deposit tx
   hash and `protocol.orderId`. Do not assume it equals either.
4. **Scoped browsing:** call `GET /requests/v2?chainId=<chainId>&limit=<n>` (or `originChainId=`/ `destinationChainId=`)
   to list recent requests on one chain when no hash or address is known yet. Add `status=<value>` to filter by
   lifecycle stage. Add `sortBy=createdAt&sortDirection=desc` for recency ordering.

Example hash lookup:

```bash
curl -sS "https://api.relay.link/requests/v2?hash=0xTX_HASH"
```

Example user-scoped lookup:

```bash
curl -sS "https://api.relay.link/requests/v2?user=0xADDRESS&limit=20"
```

## Request Fields

| Field                                              | Use                                                              |
| -------------------------------------------------- | ---------------------------------------------------------------- |
| `hash`                                             | Origin or destination transaction hash to look up a request      |
| `id`                                               | Relay-internal request ID (from a prior lookup or explorer link) |
| `orderId`                                          | `protocol.orderId` value from a prior lookup                     |
| `user`                                             | Depositor/sender address. Scopes results to that user's requests |
| `depositAddress`                                   | Per-request deposit address. Scopes results to that address      |
| `chainId` / `originChainId` / `destinationChainId` | Restrict results to requests touching a given chain              |
| `status`                                           | Filter by lifecycle status (see Status Values)                   |
| `startTimestamp` / `endTimestamp`                  | Bound results by Unix timestamp                                  |
| `limit`                                            | Page size, default 20, max 50                                    |
| `apiKey` (query) / `x-api-key` (header)            | Optional API key. Not required for standard-rate reads           |

## Report Fields

When present, extract these fields from the top-level `requests[]` array. Report the extracted fields:

| Field                  | Relay path examples                                                                      |
| ---------------------- | ---------------------------------------------------------------------------------------- |
| Request ID             | `id`                                                                                     |
| Status                 | `status`                                                                                 |
| Depositor / recipient  | `user`, `recipient`, `protocol.deposit.origin.depositor`                                 |
| Origin chain / tx      | `data.inTxs[].chainId`, `data.inTxs[].hash`, `protocol.deposit.origin.chainId`           |
| Destination chain / tx | `data.outTxs[].chainId`, `data.outTxs[].hash`, `protocol.settlement.destination.fills[]` |
| Deposit address        | `protocol.deposit.origin.depository`                                                     |
| Solver                 | `protocol.solver.address`, `protocol.solver.chainId`                                     |
| Order ID               | `protocol.orderId`                                                                       |
| Currency in / out      | `data.metadata.currencyIn`, `data.metadata.currencyOut`                                  |
| Amounts                | `data.metadata.currencyIn.amount`, `data.metadata.currencyOut.amount`                    |
| USD values             | `data.metadata.currencyIn.amountUsd`, `data.metadata.currencyOut.amountUsd`              |
| Fees                   | `data.fees`, `data.feesUsd`, `data.appFees[]`                                            |
| Timestamps             | `createdAt`, `updatedAt`, `data.inTxs[].timestamp`, `data.outTxs[].timestamp`            |
| Refund                 | `data.failReason`, `data.refundFailReason`                                               |

Amounts under `data.metadata.currencyIn`/`currencyOut` and `data.fees`/`data.feesUsd` are raw integer units in the
token's smallest denomination. Convert with the accompanying `currency.decimals` (or `amountFormatted`, which is already
decimal, when present). Do not divide `amountFormatted` again.

## Status Values

Interpret the top-level `status` field as (per the official API reference):

| Status       | Meaning                                                |
| ------------ | ------------------------------------------------------ |
| `depositing` | Deposit transaction submitted but not yet confirmed    |
| `pending`    | Deposit confirmed. Awaiting solver fill                |
| `success`    | Destination fill completed                             |
| `failure`    | Request failed. Check `data.failReason`                |
| `refund`     | Sender received the deposit refund on the origin chain |

Live samples observed only `pending` and `success` (see Sources). The documentation includes `depositing`, `failure`,
and `refund`, but this pass did not observe them directly.

For `success`, still verify the destination fill transaction with explorer or RPC data when the destination chain is a
target chain. `data.outTxs[].stateChanges[]` lists the observed balance deltas as an additional cross-check.

## Failure Handling

- Empty `requests` array: report that Relay has no record for the hash/user/ID. Continue normal explorer/RPC analysis.
- Rate limit `429` or 5xx: back off. Continue explorer/RPC analysis.
- `status: "failure"`: report `data.failReason` when present. Do not attempt remediation transactions.
- Non-target chains in Relay results: report that the leg is outside this skill. For that leg, ask for a feature request
  instead of continuing analysis.
- A transfer touching `0xa5F565650890fBA1824Ee0F21EbBbF660a179934` or `0xf70da97812CB96acDF810712Aa562db8dfA3dbEF` alone
  does not prove attribution to a specific user. Confirm via a request lookup keyed on the actual hash or user address.

## Sources

- https://docs.relay.link/what-is-relay
- https://docs.relay.link/references/api/get-requests
- https://etherscan.io/address/0xf70da97812cb96acdf810712aa562db8dfa3dbef
- https://etherscan.io/address/0xa5f565650890fba1824ee0f21ebbbf660a179934
