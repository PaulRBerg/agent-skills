# Across Bridging

## Overview

Use Across as a read-only source for Across Protocol intent-based deposit/fill status. Use it to match an origin deposit
transaction to its destination fill transaction. Use it to list recent deposits for a route.

Default to the public API base URL:

```text
https://app.across.to/api
```

These read endpoints require no API key. The documentation does not specify an API key for them.

Never execute bridge steps from this skill. Do not sign messages, submit deposit transactions, or call any Across
`SpokePool` write method (`depositV3`, `fillV3Relay`, `speedUpV3Deposit`, etc.). Returned deposit and fill data are for
inspection only.

Across is an intents/relayer bridge, not a lock-and-mint bridge. A depositor locks funds into an origin-chain
`SpokePool`. An off-chain relayer immediately fronts the equivalent output on the destination-chain `SpokePool` from its
own inventory. Across later reimburses the relayer through its UMA-secured bundle settlement. There is no destination
mint event to find. For reports, the relevant destination event is a relayer-funded fill, not a mint.

## Read-Only Router

Use this router after confirming the known origin and destination chains against
`references/generated/target-mainnets.json`.

1. **Known origin deposit tx hash:** call `GET /deposit/status?depositTxHash=<hash>&originChainId=<id>`. This is the
   most useful lookup. It connects a known origin-chain deposit transaction to its relayer fill.
2. **Known origin chain + deposit ID:** call `GET /deposit/status?originChainId=<id>&depositId=<id>` when the numeric
   `SpokePool` deposit ID is known instead of a tx hash (e.g. decoded from a `V3FundsDeposited` log).
3. **Browse recent/matching deposits:** call `GET /deposits` with optional filters (`originChainId`,
   `destinationChainId`, `depositor`, `recipient`, `status`, `limit`) to find a deposit when only a wallet or route is
   known, not an exact tx hash or deposit ID.
4. **Supported routes:** call `GET /available-routes` to check which origin/destination chain and token pairs Across
   currently supports. Optionally filter with `originChainId`, `destinationChainId`, `originToken`, `destinationToken`.

Example status lookup by deposit tx hash:

```bash
curl -sS "https://app.across.to/api/deposit/status?depositTxHash=0xTX_HASH&originChainId=1"
```

Example status lookup by origin chain + deposit ID:

```bash
curl -sS "https://app.across.to/api/deposit/status?originChainId=1&depositId=4181942"
```

Example deposit browse:

```bash
curl -sS "https://app.across.to/api/deposits?originChainId=1&destinationChainId=8453&limit=5"
```

`GET /deposit/status` requires either `depositTxHash` or (`originChainId` + `depositId`). A call without identifying
parameters returns a 400 `IncorrectQueryParamsException`. An unmatched deposit returns a 404 `DepositNotFoundException`.

## Request Fields

| Field                     | Use                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------ |
| `originChainId`           | Origin chain ID. Required alongside `depositId`, optional (but recommended) with `depositTxHash` |
| `depositId`               | Numeric `SpokePool` deposit ID. Use with `originChainId` instead of a tx hash                    |
| `depositTxHash`           | Origin-chain deposit transaction hash                                                            |
| `destinationChainId`      | Destination chain ID. Filter for `/deposits` and `/available-routes`                             |
| `depositor` / `recipient` | Sender/receiver address filter for `/deposits`. Use only known addresses                         |
| `status`                  | Filter for `/deposits`. Use one of the values in Status Values below                             |
| `limit`                   | Row cap for `/deposits`                                                                          |

Do not invent wallet addresses. Use only user-provided or known on-chain addresses.

## Report Fields

When present, extract these fields from `/deposit/status` or a `/deposits` row. Both share the same schema. Report the
extracted fields:

| Field                    | Across path examples                                |
| ------------------------ | --------------------------------------------------- |
| Origin chain/deposit ID  | `originChainId`, `depositId`                        |
| Origin deposit tx hash   | `depositTxHash` (alias `depositTxnRef`)             |
| Depositor / recipient    | `depositor`, `recipient`                            |
| Input token/amount       | `inputToken`, `inputAmount`                         |
| Output token/amount      | `outputToken`, `outputAmount`                       |
| Destination chain        | `destinationChainId`                                |
| Status                   | `status`                                            |
| Relayer                  | `relayer`                                           |
| Destination fill tx hash | `fillTx` (alias `fillTxnRef`)                       |
| Fill block/timestamp     | `fillBlockNumber`, `fillBlockTimestamp`             |
| Refund tx hash           | `depositRefundTxHash` (alias `depositRefundTxnRef`) |
| Bridge fee (USD)         | `bridgeFeeUsd`                                      |
| Deposit block/timestamp  | `depositBlockNumber`, `depositBlockTimestamp`       |

`inputAmount` and `outputAmount` are raw integer units in the respective token's smallest denomination. Convert with
token decimals when present. USD fee fields (`bridgeFeeUsd`, `fillGasFeeUsd`) are already decimal.

## SpokePool Contracts and Events

Across routes deposits and fills through per-chain `SpokePool` contracts rather than a single hub contract. The Ethereum
mainnet `SpokePool` is at `0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5`. It is an EIP-1967 proxy. Block explorers label
its current implementation `Ethereum_SpokePool`. Resolve `SpokePool` addresses on other target chains from
`references/generated/target-mainnets.json` or the chain's block explorer rather than hardcoding a full table here.

When decoding logs directly instead of using the API, the relevant `SpokePool` events are:

| Chain side  | Current event      | Deprecated alias |
| ----------- | ------------------ | ---------------- |
| Origin      | `V3FundsDeposited` | `FundsDeposited` |
| Destination | `FilledV3Relay`    | `FilledRelay`    |

The deployed `SpokePool` ABI includes both current and deprecated event names. A specific deposit emits only one of the
pair, depending on the protocol version active at that block. Match an origin deposit to its destination fill by the
shared `depositId` (and `originChainId`), not by matching transaction hashes across chains.

## Status Values

Interpret the `status` field as:

| Status              | Meaning                                                 |
| ------------------- | ------------------------------------------------------- |
| `unfilled`          | Deposited, no relayer fill yet                          |
| `filled`            | Terminal success — relayer fronted the output           |
| `slowFillRequested` | Fast fill window missed. A slow fill has been requested |
| `slowFilled`        | Terminal success via the slower pool-funded fill path   |
| `expired`           | Fill deadline passed with no fill                       |
| `refunded`          | Terminal failure/refund — depositor refunded on origin  |

For `filled` or `slowFilled`, still verify the destination fill transaction directly with explorer or RPC data when the
destination chain is a target chain.

## Failure Handling

- 400 `IncorrectQueryParamsException` from `/deposit/status`: supply either `depositTxHash` or both `originChainId` and
  `depositId`.
- 404 `DepositNotFoundException`: report that Across has no record for those parameters. Continue normal explorer/RPC
  analysis. Do not assume the deposit never happened. `/deposit/status` only indexes deposits made through Across's own
  `SpokePool` flow.
- `/deposits` returning an empty array: broaden or drop filters, or fall back to a direct `depositTxHash` lookup.
- `status: "expired"` or `"refunded"`: report as a failed/refunded transfer. When the origin chain is a target chain,
  verify the refund transaction there.
- Rate limiting or 5xx responses: back off. Continue explorer/RPC analysis.

## Sources

- https://docs.across.to/reference/api-reference
- https://docs.across.to/introduction/what-is-across
- https://docs.across.to/concepts/intents
- https://github.com/across-protocol/contracts
