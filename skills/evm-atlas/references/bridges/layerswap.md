# Layerswap Bridging

## Overview

Use Layerswap as a read-only source for cross-chain swap quotes, transfer limits, supported networks/tokens, and swap
status lookups by transaction hash, swap ID, or wallet address.

Default to the API base URL:

```text
https://api.layerswap.io
```

Most Layerswap API calls work without an API key at lower rate limits. Add the key only when present:

```bash
-H "X-LS-APIKEY: $LAYERSWAP_API_KEY"
```

Never execute bridge steps from this skill. Do not sign messages, submit deposit transactions, call `POST /api/v2/swaps`
or any gasless/deposit-action endpoint to create a swap, or broadcast a returned deposit address as a transaction
target. Returned swap, quote, and deposit data are for inspection only.

Layerswap identifies networks by its own name strings (e.g. `ETHEREUM_MAINNET`, `ARBITRUM_MAINNET`), not by
`references/generated/target-mainnets.json`'s `chainName`. Call `GET /api/v2/networks` to resolve a Layerswap `name` to
its `chain_id`. Then match against `references/generated/target-mainnets.json`'s numeric `chainId`.

Layerswap returns `chain_id` as a JSON string (e.g. `"1"`, `"42161"`). Cast before comparing. Treat `/networks` as the
live authoritative source. Never hardcode a name-to-chain-ID table.

Layerswap settles some transfers through shared service infrastructure rather than a per-user deposit address. Etherscan
labels the Ethereum address `0x2Fc617E933a52713247CE25730f6695920B3befe` "Layerswap 1". This is a shared service
address, not a specific user's deposit address. Do not attribute a transfer to a specific user solely because it touches
this address. Confirm the actual swap and counterparties via `GET /api/v2/swaps/by_transaction_hash/{hash}` or
`GET /api/v2/swaps?address=`.

## Read-Only Router

Use this router after confirming the known origin and destination chains against
`references/generated/target-mainnets.json`.

1. **Known source tx hash:** call `GET /api/v2/swaps/by_transaction_hash/{transactionHash}`. This is the most useful
   lookup. It connects a known source-chain transaction to its Layerswap swap and destination outcome.
2. **Known swap ID:** call `GET /api/v2/swaps/{swapId}` when a UUID swap ID is known, e.g. from a Layerswap explorer
   link.
3. **Known sender/recipient address, no tx hash:** call `GET /api/v2/swaps?address=<address>`. Narrow with `statuses`,
   `networks`, or `page`.
4. **Quote / fee estimate:** call `GET /api/v2/quote` with `source_network`, `source_token`, `destination_network`,
   `destination_token`, and `amount` (all required).
5. **Min/max transfer limits:** call `GET /api/v2/limits` with the same four required network/token params.
6. **Available source routes into a destination:** call `GET /api/v2/sources` with `destination_network` and
   `destination_token`.
7. **Supported networks and tokens:** call `GET /api/v2/networks`. Add `network_types=evm` to scope to EVM chains.

Example tx-hash lookup:

```bash
curl -sS "https://api.layerswap.io/api/v2/swaps/by_transaction_hash/0xTX_HASH"
```

Example quote inspection:

```bash
curl -sS "https://api.layerswap.io/api/v2/quote?source_network=ETHEREUM_MAINNET&source_token=USDC&destination_network=ARBITRUM_MAINNET&destination_token=USDC&amount=100"
```

If `$LAYERSWAP_API_KEY` is set, add `-H "X-LS-APIKEY: $LAYERSWAP_API_KEY"`.

## Request Fields

| Field                                    | Use                                                                                                       |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `source_network` / `destination_network` | Layerswap network name string (e.g. `ETHEREUM_MAINNET`). Resolve via `GET /api/v2/networks`. Do not guess |
| `source_token` / `destination_token`     | Token symbol from that network's token catalog, or contract address                                       |
| `amount`                                 | Source amount in decimal units (e.g. `100` for 100 USDC) — not raw smallest-unit values                   |
| `source_address`                         | Sender address. Use only user-provided or placeholder addresses                                           |
| `slippage`                               | Optional slippage tolerance                                                                               |
| `refuel`                                 | Optional flag requesting a small native-gas top-up on the destination chain                               |
| `use_deposit_address`                    | Optional flag affecting deposit-address-based routing. Informational only here                            |

Do not invent wallet addresses. Use placeholders for hypothetical quotes and user-provided addresses for real lookups.

## Report Fields

When present, extract these fields. Report the extracted fields:

| Field                        | Layerswap path examples                                                                                      |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Swap ID                      | `data.swap.id`                                                                                               |
| Source chain/token           | `data.quote.source_network.name`, `data.quote.source_token.symbol`                                           |
| Destination chain/token      | `data.quote.destination_network.name`, `data.quote.destination_token.symbol`                                 |
| Amounts                      | `data.quote.requested_amount`, `data.quote.receive_amount`, `data.quote.min_receive_amount`                  |
| Fees                         | `data.quote.total_fee`, `data.quote.total_fee_in_usd`, `data.quote.blockchain_fee`, `data.quote.service_fee` |
| Status                       | `data.swap.status`                                                                                           |
| Source/destination tx hashes | `data.swap.transactions[].transaction_hash`, `data.deposit_actions[]`                                        |
| Explorer links               | Build from `transaction_explorer_template` / `account_explorer_template` returned by `GET /api/v2/networks`  |

Unlike other bridge references in this skill, Layerswap amounts are already decimal, human-readable values scaled by the
token's decimals. Do not divide by `10^decimals` again. `chain_id` fields are JSON strings, not numbers. Cast before
comparing to `references/generated/target-mainnets.json`.

## Status Values

Interpret the `swap.status` field as:

| Status                  | Meaning                                                                 |
| ----------------------- | ----------------------------------------------------------------------- |
| `user_transfer_pending` | Waiting for the user's deposit transaction on the source network        |
| `cancelled`             | User cancelled before depositing                                        |
| `expired`               | No deposit arrived within the 4-day deposit window                      |
| `ls_transfer_pending`   | Layerswap matched the deposit and is executing the cross-chain transfer |
| `completed`             | Layerswap initiated the outgoing transaction to the destination address |
| `failed`                | Layerswap could not initiate the outgoing transaction                   |

Layerswap's refunds documentation also describes `pending_refund` and `refunded` states for retry-exhausted swaps with a
`refund_address`. Layerswap always sends refunds on the source chain in the source token, minus gas. These states do not
appear in the core `swap.status` enum above. Treat them as real. Verify them via `data.swap.transactions[]` entries with
`type: "refund"` instead of assuming the exact status string.

Separately, the `GET /api/v2/swaps` list endpoint's `statuses` filter uses a third, differently-cased vocabulary:
`PendingDeposit`, `Completed`, `Failed`, `Expired`, `PendingWithdrawal`, `PendingRefund`, `Refunded`. Treat it as a
distinct filter vocabulary, not a direct casing-only mapping of the values above.

For `completed` or a refund outcome, still verify the terminal destination or refund transfer with explorer or RPC data
when the relevant chain is a target chain.

## Failure Handling

- Missing `$LAYERSWAP_API_KEY`: use unauthenticated requests. For those requests, respect lower public rate limits.
- Rate limit `429` or 5xx: back off. Mention the limit. Continue explorer/RPC analysis.
- Empty `/quote`, `/limits`, or `/sources` result: report the API's `error` field when present. Try only user-approved
  alternative tokens, chains, or amounts.
- Unknown swap ID or tx hash (empty `data`): report that Layerswap has no record. Continue normal explorer/RPC analysis.
- Non-target chains in Layerswap results: report that the leg is outside this skill. For that leg, ask for a feature
  request instead of continuing analysis.
- A transfer touching `0x2Fc617E933a52713247CE25730f6695920B3befe` alone does not prove attribution to a specific user.
  Confirm via the swap-lookup endpoints.

## Sources

- https://learn.layerswap.io/api/overview
- https://learn.layerswap.io/api/api-integration/quickstart
- https://learn.layerswap.io/api/api-integration/swap-lifecycle
- https://learn.layerswap.io/api/data/object-types
- https://docs.layerswap.io/api-reference/swaps/get-quote
- https://docs.layerswap.io/api-reference/swaps/get-swap-route-limits
- https://docs.layerswap.io/api-reference/swaps/get-sources
- https://docs.layerswap.io/api-reference/swaps/get-swap-details
- https://docs.layerswap.io/api-reference/swaps/get-swap-by-transaction-hash
- https://docs.layerswap.io/api-reference/swaps/get-all-swaps
- https://docs.layerswap.io/api-reference/swaps/get-networks
