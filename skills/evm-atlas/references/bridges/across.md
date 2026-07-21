# Across Bridging

## Overview

Use Across as a read-only source for Across Protocol intent-based deposit/fill status, matching an origin deposit
transaction to its destination fill transaction, and enumerating recent deposits for a route.

Default to the public API base URL:

```text
https://app.across.to/api
```

No API key is required or documented for these read endpoints.

Never execute bridge steps from this skill. Do not sign messages, submit deposit transactions, or call any Across
`SpokePool` write method (`depositV3`, `fillV3Relay`, `speedUpV3Deposit`, etc.). Returned deposit and fill data are for
inspection only.

Across is an intents/relayer bridge, not a lock-and-mint bridge: a depositor locks funds into an origin-chain
`SpokePool`, and an off-chain relayer immediately fronts the equivalent output on the destination-chain `SpokePool` from
its own inventory, later reimbursed through Across's UMA-secured bundle settlement. There is no destination mint event
to look for; the report-relevant destination event is a relayer-funded fill, not a mint.

## Read-Only Router

Use this router after the known origin and destination chains are confirmed against
`references/generated/target-mainnets.json`.

1. **Known origin deposit tx hash:** call `GET /deposit/status?depositTxHash=<hash>&originChainId=<id>`. This is the
   most useful lookup — connects a known origin-chain deposit transaction to its relayer fill.
2. **Known origin chain + deposit ID:** call `GET /deposit/status?originChainId=<id>&depositId=<id>` when the numeric
   `SpokePool` deposit ID is known instead of a tx hash (e.g. decoded from a `V3FundsDeposited` log).
3. **Browse recent/matching deposits:** call `GET /deposits` with optional filters (`originChainId`,
   `destinationChainId`, `depositor`, `recipient`, `status`, `limit`) to find a deposit when only a wallet or route is
   known, not an exact tx hash or deposit ID.
4. **Supported routes:** call `GET /available-routes` to check which origin/destination chain and token pairs Across
   currently supports; optionally filter with `originChainId`, `destinationChainId`, `originToken`, `destinationToken`.

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

`GET /deposit/status` requires either `depositTxHash` or (`originChainId` + `depositId`); calling it with no identifying
params returns a 400 `IncorrectQueryParamsException`, and an unmatched deposit returns a 404 `DepositNotFoundException`.

## Request Fields

| Field                     | Use                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------ |
| `originChainId`           | Origin chain ID; required alongside `depositId`, optional (but recommended) with `depositTxHash` |
| `depositId`               | Numeric `SpokePool` deposit ID; use with `originChainId` instead of a tx hash                    |
| `depositTxHash`           | Origin-chain deposit transaction hash                                                            |
| `destinationChainId`      | Destination chain ID; filter for `/deposits` and `/available-routes`                             |
| `depositor` / `recipient` | Sender/receiver address filter for `/deposits`; use only known addresses                         |
| `status`                  | Filter for `/deposits`; one of the values in Status Values below                                 |
| `limit`                   | Row cap for `/deposits`                                                                          |

Do not invent wallet addresses. Use only user-provided or known on-chain addresses.

## Report Fields

Extract and report these fields when present (from `/deposit/status` or a `/deposits` row — both share the same schema):

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
mainnet `SpokePool` is at `0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5` (an EIP-1967 proxy; current implementation
labeled `Ethereum_SpokePool` on block explorers). Resolve `SpokePool` addresses on other target chains from
`references/generated/target-mainnets.json` or the chain's block explorer rather than hardcoding a full table here.

When decoding logs directly instead of using the API, the relevant `SpokePool` events are:

| Chain side  | Current event      | Deprecated alias |
| ----------- | ------------------ | ---------------- |
| Origin      | `V3FundsDeposited` | `FundsDeposited` |
| Destination | `FilledV3Relay`    | `FilledRelay`    |

Both the current and deprecated event names are present in the deployed `SpokePool` ABI; a specific deposit emits only
one of the pair depending on the protocol version active at that block. Match an origin deposit to its destination fill
by the shared `depositId` (and `originChainId`), not by matching transaction hashes across chains.

## Status Values

Interpret the `status` field as:

| Status              | Meaning                                                 |
| ------------------- | ------------------------------------------------------- |
| `unfilled`          | Deposited, no relayer fill yet                          |
| `filled`            | Terminal success — relayer fronted the output           |
| `slowFillRequested` | Fast fill window missed; a slow fill has been requested |
| `slowFilled`        | Terminal success via the slower pool-funded fill path   |
| `expired`           | Fill deadline passed with no fill                       |
| `refunded`          | Terminal failure/refund — depositor refunded on origin  |

For `filled` or `slowFilled`, still verify the destination fill transaction directly with explorer or RPC data when the
destination chain is a target chain.

## Failure Handling

- 400 `IncorrectQueryParamsException` from `/deposit/status`: supply either `depositTxHash` or both `originChainId` and
  `depositId`.
- 404 `DepositNotFoundException`: report that Across has no record for those params and continue normal explorer/RPC
  analysis; do not assume the deposit never happened, since `/deposit/status` only indexes deposits made through
  Across's own `SpokePool` flow.
- `/deposits` returning an empty array: broaden or drop filters, or fall back to a direct `depositTxHash` lookup.
- `status: "expired"` or `"refunded"`: report as a failed/refunded transfer; verify the refund transaction on the origin
  chain when a target chain.
- Rate limiting or 5xx responses: back off and continue explorer/RPC analysis.

## Sources

- https://docs.across.to/reference/api-reference
- https://docs.across.to/introduction/what-is-across
- https://docs.across.to/concepts/intents
- https://github.com/across-protocol/contracts
