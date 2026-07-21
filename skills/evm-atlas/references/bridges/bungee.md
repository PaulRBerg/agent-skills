# Bungee API

## Overview

Use Bungee as a read-only enrichment source for bridge transactions, cross-chain swaps, and Socket/Bungee route status
after the known origin and destination chains are confirmed against `references/generated/target-mainnets.json`. Bungee
can connect the source transaction to destination execution, route metadata, and refund details, but it does not replace
explorer, RPC, or receipt verification.

## Router Caveat

Bungee/Socket is a routing layer, not a single bridge protocol. Socket Swap V3 exposes bridge providers including
`cctp-v2` and `cctp-v2-slow`, so a Bungee/Socket route may use Circle CCTP under the hood. When logs, provider metadata,
or destination mint events indicate CCTP, read `references/bridges/circle.md` to interpret Circle CCTP contracts, fee
recipients, and `MintAndWithdraw` logs.

## API Generations

Identify the route generation before choosing a status API. The indexes and identifiers are not interchangeable:

- **Socket Swap V3:** `GET /v3/swap/status?quoteId=<quoteId>` is keyed by the `quoteId` returned by a V3 quote. Use the
  quote response's `statusCheck.endpoint` when available. Do not substitute an origin transaction hash for `quoteId`,
  and do not treat a V3 quote-not-found response as evidence that a legacy bridge transaction never existed.
- **Bungee v1 / Auto:** use `/api/v1/bungee/status` with a known `txHash` or `requestHash`, or
  `/api/v1/bungee-auto/history` with a known sender.
- **Legacy Socket Aggregator v2:** use `/v2/bridge-status` for a source transaction sent through the legacy Socket
  Registry/Gateway or another decoded v2 route. This includes historical routes that predate Bungee v1 or V3 indexing.

For Bungee v1, default to the public sandbox API:

```text
https://public-backend.bungee.exchange
```

Allow an override via `$BUNGEE_API_BASE_URL`. Only send paid or dedicated headers when the corresponding variables are
present:

```bash
-H "x-api-key: $BUNGEE_API_KEY"
-H "affiliate: $BUNGEE_AFFILIATE_ID"
```

The public sandbox is shared and rate-limited. On rate-limit responses or 5xx errors, report that limitation, include
the `server-req-id` response header when available, and continue normal explorer/RPC analysis.

## Lookup Router

Use this router when the user mentions bridging, bridge tx, cross-chain swap, Bungee, Socket, or the transaction looks
bridge-related from logs, counterparties, calldata, or token movement. If Bungee returns an origin, destination, or
refund chain outside the target list, report that the non-target leg is outside this skill and ask for a feature request
rather than continuing analysis on that leg.

1. **Known Socket V3 quote ID:** query its `statusCheck.endpoint`, or `GET /v3/swap/status?quoteId=<quoteId>` on the
   matching Socket V3 backend.
2. **Known Bungee Auto request hash:** query `GET /api/v1/bungee/status?requestHash=<hash>`.
3. **Known Bungee v1 source tx hash:** query `GET /api/v1/bungee/status?txHash=<hash>`.
4. **Known legacy Socket v2 source tx:** query the credentialed `/v2/bridge-status` request below.
5. **Ambiguous hash:** try Bungee v1 `txHash`, then `requestHash`. Try legacy v2 only when the origin chain and decoded
   contracts or calldata indicate a v2 Socket route. Do not try V3 without a `quoteId`.
6. **Known sender, no tx hash:** query `GET /api/v1/bungee-auto/history?sender=<addr>&pageNumber=1&pageSize=10`;
   paginate only when the first page does not cover the relevant time window or result count.

Example:

```bash
base="${BUNGEE_API_BASE_URL:-https://public-backend.bungee.exchange}"
headers_file="$(mktemp)"
trap 'rm -f "$headers_file"' EXIT
curl -sS -D "$headers_file" "$base/api/v1/bungee/status?txHash=0xTX_HASH"
```

If `$BUNGEE_API_KEY` or `$BUNGEE_AFFILIATE_ID` is set, add those headers to the request. Do not require them for public
sandbox reads.

For legacy Socket v2, require `$SOCKET_API_KEY`; never copy a demo key from documentation. Send the known source and
destination chains even though the live schema makes `toChainId` optional. Add `bridgeName` only when logs or decoded
route data identify it:

```bash
test -n "${SOCKET_API_KEY:-}" || { echo 'SOCKET_API_KEY is required for Socket v2' >&2; exit 1; }
curl -sS -G 'https://api.socket.tech/v2/bridge-status' \
  -H "API-KEY: $SOCKET_API_KEY" \
  --data-urlencode 'transactionHash=0xSOURCE_TX_HASH' \
  --data-urlencode 'fromChainId=42161' \
  --data-urlencode 'toChainId=137'
```

When known, append `--data-urlencode 'bridgeName=hyphen'` or the decoded provider name. A successful source receipt from
`/v2/tx-receipt` proves only source-chain execution; it does not prove destination completion.

## Report Fields

Extract and report these fields when present:

V1 status lookups return rows under `result[]`; history lookups may return rows under `result.data[]`. Legacy v2 returns
one row under `result`. V3 status fields are top-level.

| Field                 | Status path examples                                                                      |
| --------------------- | ----------------------------------------------------------------------------------------- |
| Origin chain          | V1 `originData.originChainId`; v2 `fromChainId`; history `input[].chainId`                |
| Origin tx hash        | V1 `originData.txHash`; v2 `sourceTransactionHash`; history `sourceTransactionHash`       |
| Sender                | V1 `originData.userAddress`; v2/history `sender`                                          |
| Input tokens/amounts  | V1 `originData.input[]`; v2 `fromAsset` / `fromAmount`; history `input[]`                 |
| Origin status         | V1 `originData.status`; v2 `sourceTxStatus`; history `status` / `statusCode`              |
| Destination chain     | V1 `destinationData.destinationChainId`; v2 `toChainId`; history `output[].chainId`       |
| Destination tx hash   | V1 `destinationData.txHash`; v2/history `destinationTransactionHash`                      |
| Receiver              | V1 `destinationData.receiverAddress`; v2/history `recipient`                              |
| Output tokens/amounts | V1 `destinationData.output[]`; v2 `toAsset` / `toAmount`; history `output[]`              |
| Destination status    | V1 `destinationData.status`; v2 `destinationTxStatus`; history `status` / `statusCode`    |
| Route / bridge        | V1 `routeDetails.name`; v2 `bridgeName`; history `routeDetails.name` if present           |
| Bungee status code    | V1 `bungeeStatusCode`; history `statusCode`                                               |
| Timestamps            | V1 `createdAt` / `updatedAt`; history `orderTimestamp` / `srcTimestamp` / `destTimestamp` |
| Refund                | V1 `refund.chainId` / `refund.txHash`; v2 `refuel`; history `refund` fields               |

Token amounts are raw integer units. Convert with token decimals when present, and preserve raw values when decimals are
absent.

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

Apply the `PENDING` meaning only to a populated record. Some Bungee v1 and legacy Socket v2 lookups return an echoed
source hash with `PENDING` statuses while chain IDs, assets, amounts, sender, recipient, route/bridge, timestamps, and
destination hash are all null. Classify that shape as **unresolved / not indexed**, not as an in-progress transaction.
It can appear for both historical real transactions and fabricated hashes.

## Failure Handling

- Empty `result` or an explicit no-match/not-found response: continue normal Etherscan, Blockscout, explorer, or RPC
  analysis and say Bungee had no record.
- Hash-only or metadata-empty `PENDING`: report that the applicable Socket/Bungee index did not resolve the route; do
  not report the transaction as pending, failed, or nonexistent.
- Rate limits and 5xx responses: mention the public sandbox is shared/limited, capture `server-req-id` when available,
  and continue explorer/RPC analysis.
- Other `success=false` Bungee API errors: report `statusCode`, `message`, and `server-req-id` when available, then
  continue explorer/RPC analysis.
- Same-chain manual swaps: verify the submitted transaction receipt directly because Bungee manual same-chain swaps
  complete in the submitted on-chain transaction.
- Explorer/RPC receipts, decoded bridge logs, and the exact terminal destination transfer remain authoritative when an
  API generation lacks historical coverage or disagrees with confirmed chain evidence.

## Sources

- https://docs.bungee.exchange/integrate/get-api-access
- https://docs.bungee.exchange/api-reference/core-api/get-request-status
- https://docs.bungee.exchange/integrate/integration-guides/check-status
- https://docs.bungee.exchange/llms.txt
- https://docs.socket.tech/integrate/integration-guides/socket-api.md
- https://docs.socket.tech/integrate/migration-guide.md
- https://docs.socket.tech/integrate/migration-guide-v2.md
- https://docs.socket.tech/about/supported-providers.md
- https://api.socket.tech/v2/swagger/
- https://public-backend.bungee.exchange/swagger-json
