# Bungee API

## Overview

Use Bungee as a read-only enrichment source for bridge transactions, cross-chain swaps, and Socket/Bungee route status after the known origin and destination chains are confirmed against `./references/target-mainnets.json`. Bungee can connect the source transaction to destination execution, route metadata, and refund details, but it does not replace explorer, RPC, or receipt verification.

## Router Caveat

Bungee/Socket is a routing layer, not a single bridge protocol. Socket Swap V3 exposes bridge providers including `cctp-v2` and `cctp-v2-slow`, so a Bungee/Socket route may use Circle CCTP under the hood. When logs, provider metadata, or destination mint events indicate CCTP, read `./references/bridge-circle.md` to interpret Circle CCTP contracts, fee recipients, and `MintAndWithdraw` logs.

Default to the public sandbox API:

```text
https://public-backend.bungee.exchange
```

Allow an override via `$BUNGEE_API_BASE_URL`. Only send paid or dedicated headers when the corresponding variables are present:

```bash
-H "x-api-key: $BUNGEE_API_KEY"
-H "affiliate: $BUNGEE_AFFILIATE_ID"
```

The public sandbox is shared and rate-limited. On rate-limit responses or 5xx errors, report that limitation, include the `server-req-id` response header when available, and continue normal explorer/RPC analysis.

## Lookup Router

Use this router when the user mentions bridging, bridge tx, cross-chain swap, Bungee, Socket, or the transaction looks bridge-related from logs, counterparties, calldata, or token movement. If Bungee returns an origin, destination, or refund chain outside the target list, report that the non-target leg is outside this skill and ask for a feature request rather than continuing analysis on that leg.

1. **Known source tx hash:** query `GET /api/v1/bungee/status?txHash=<hash>`.
2. **Known Bungee Auto request hash:** query `GET /api/v1/bungee/status?requestHash=<hash>`.
3. **Ambiguous hash:** try `txHash` first, then `requestHash`; only then say Bungee had no matching record.
4. **Known sender, no tx hash:** query `GET /api/v1/bungee-auto/history?sender=<addr>&pageNumber=1&pageSize=10`; paginate only when the first page does not cover the relevant time window or result count.

Example:

```bash
base="${BUNGEE_API_BASE_URL:-https://public-backend.bungee.exchange}"
headers_file="$(mktemp)"
trap 'rm -f "$headers_file"' EXIT
curl -sS -D "$headers_file" "$base/api/v1/bungee/status?txHash=0xTX_HASH"
```

If `$BUNGEE_API_KEY` or `$BUNGEE_AFFILIATE_ID` is set, add those headers to the request. Do not require them for public sandbox reads.

## Report Fields

Extract and report these fields when present:

Status lookups return rows under `result[]`. History lookups may return rows under `result.data[]` with `pageNumber` and `pageSize` metadata.

| Field                 | Bungee path examples                                                                                       |
| --------------------- | ---------------------------------------------------------------------------------------------------------- |
| Origin chain          | `originData.originChainId`, `input[].token.chainId`, history `input[].chainId`                             |
| Origin tx hash        | `originData.txHash`, history `sourceTransactionHash`                                                       |
| Sender                | `originData.userAddress`, history `sender`                                                                 |
| Input tokens/amounts  | `originData.input[]`, history `input[]`                                                                    |
| Origin status         | `originData.status`, history `status` / `statusCode`                                                       |
| Destination chain     | `destinationData.destinationChainId`, `destinationData.output[].token.chainId`, history `output[].chainId` |
| Destination tx hash   | `destinationData.txHash`, history `destinationTransactionHash`                                             |
| Receiver              | `destinationData.receiverAddress`, history `recipient`                                                     |
| Output tokens/amounts | `destinationData.output[]`, history `output[]`                                                             |
| Destination status    | `destinationData.status`, history `status` / `statusCode`                                                  |
| Route / bridge        | `routeDetails.name`, history `routeDetails.name` if present                                                |
| Bungee status code    | `bungeeStatusCode`, history `statusCode`                                                                   |
| Timestamps            | `createdAt`, `updatedAt`, `orderTimestamp`, `srcTimestamp`, `destTimestamp`                                |
| Refund                | `refund.chainId`, `refund.txHash`, `refund` object fields                                                  |

Token amounts are raw integer units. Convert with token decimals when present, and preserve raw values when decimals are absent.

## Status Codes

Interpret `bungeeStatusCode` / history `statusCode` as:

| Code | Name        | Meaning                 |
| ---- | ----------- | ----------------------- |
| 0    | `PENDING`   | In progress             |
| 1    | `ASSIGNED`  | In progress             |
| 2    | `EXTRACTED` | In progress             |
| 3    | `FULFILLED` | Success terminal        |
| 4    | `SETTLED`   | Success terminal        |
| 5    | `EXPIRED`   | Failure/refund terminal |
| 6    | `CANCELLED` | Failure/refund terminal |
| 7    | `REFUNDED`  | Failure/refund terminal |

## Failure Handling

- Empty `result` or an explicit no-match/not-found response: continue normal Etherscan, Blockscout, explorer, or RPC analysis and say Bungee had no record.
- Rate limits and 5xx responses: mention the public sandbox is shared/limited, capture `server-req-id` when available, and continue explorer/RPC analysis.
- Other `success=false` Bungee API errors: report `statusCode`, `message`, and `server-req-id` when available, then continue explorer/RPC analysis.
- Same-chain manual swaps: verify the submitted transaction receipt directly because Bungee manual same-chain swaps complete in the submitted on-chain transaction.

## Sources

- https://docs.bungee.exchange/integrate/get-api-access
- https://docs.bungee.exchange/api-reference/core-api/get-request-status
- https://docs.bungee.exchange/integrate/integration-guides/check-status
- https://docs.bungee.exchange/llms.txt
- https://docs.socket.tech/about/supported-providers.md
- https://public-backend.bungee.exchange/swagger-json
