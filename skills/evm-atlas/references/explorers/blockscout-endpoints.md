# Endpoints, Credits & Limits

Bases:

- Unified PRO: `https://api.blockscout.com/{chain_id}/api/v2/...` (key required)
- Etherscan-V2 alias: `https://api.blockscout.com/v2/api?chain_id={id}&module=...&action=...` (key required.
  `{status,message,result}` shape)
- Per-instance: `https://{instance}/api/v2/...` and `https://{instance}/api?module=...` (keyless traffic has rate
  limits. See below)

Blockscout ended keyless access in July 2026. Hosted `*.blockscout.com` instance subdomains enforce keyless rate limits.
They return `429` under sweep-shaped traffic. Thus, the keyed `https://api.blockscout.com/{chain_id}` gateway is the
correct route for every Blockscout-hosted chain. It is also the correct fallback after a `429`. Reserve per-instance
hosts for self-hosted or third-party instances the gateway does not serve.

## Native REST v2 — Endpoint Catalog

Address (the core of this skill):

| Path                                                   | Returns                                      |
| ------------------------------------------------------ | -------------------------------------------- |
| `addresses/{hash}`                                     | Native balance, metadata, creation info      |
| `addresses/{hash}/token-balances`                      | Full holdings (array, single call)           |
| `addresses/{hash}/tokens?type=ERC-20,ERC-721,ERC-1155` | Paginated/filtered holdings                  |
| `addresses/{hash}/transactions?filter=to\|from`        | Normal transactions                          |
| `addresses/{hash}/internal-transactions`               | Internal transactions                        |
| `addresses/{hash}/token-transfers?type=ERC-20`         | Token transfers (also `ERC-721`, `ERC-1155`) |
| `addresses/{hash}/coin-balance-history`                | Native balance over time                     |
| `addresses/{hash}/logs`                                | Logs emitted by the address                  |
| `addresses/{hash}/nft?type=ERC-721,ERC-1155`           | Owned NFT instances                          |

Beyond address (use the fallback docs for full schemas):

| Path                                        | Returns                  |
| ------------------------------------------- | ------------------------ |
| `transactions/{hash}`                       | Transaction detail       |
| `transactions/{hash}/token-transfers`       | Transfers within a tx    |
| `transactions/{hash}/logs`                  | Logs within a tx         |
| `transactions/{hash}/internal-transactions` | Internal txs within a tx |
| `blocks/{number_or_hash}`                   | Block detail             |
| `tokens/{hash}`                             | Token metadata           |
| `tokens/{hash}/holders`                     | Token holder list        |
| `smart-contracts/{hash}`                    | ABI + verified source    |
| `search?q=...`                              | Unified search           |
| `stats`                                     | Chain-level stats        |

Pagination uses keysets. Responses include `next_page_params` (50/page). For the next page, append those fields as query
parameters. `null` means the last page. The API has no `sort` parameter. It returns newest-first.

## Etherscan-Compatible Actions

Available on both `/{chain_id}/api?module=...` and the `/v2/api?chain_id=...` alias. `module=account` actions:
`balance`, `balancemulti`, `tokenbalance`, `tokenlist`, `txlist`, `txlistinternal`, `tokentx`, `tokennfttx`,
`token1155tx`. Other modules: `logs/getLogs`, `contract/getabi`, `contract/getsourcecode`, `block/*`, `stats/*`,
`token/*`. The compat layer is legacy and does not implement every Etherscan action. Prefer native v2. The compat layer
supports `page`/`offset`/`sort` (asc/desc), which native v2 does not.

## Credit Costs (PRO host)

Default **20 credits** per call. Exceptions:

| Endpoint                                           | Credits |
| -------------------------------------------------- | ------- |
| (default — all unlisted)                           | 20      |
| `api/v2/search/quick`                              | 25      |
| `api/v2/tokens`                                    | 30      |
| `api/v2/tokens/{hash}/transfers`                   | 30      |
| `api/v2/transactions/{hash}/logs`                  | 30      |
| `api/v2/transactions/{hash}/token-transfers`       | 30      |
| `api/v2/transactions/{hash}/state-changes`         | 30      |
| `api/v2/addresses/{hash}/token-transfers`          | 30      |
| `api/v2/addresses/{hash}/logs`                     | 30      |
| `api/v2/transactions/{hash}/internal-transactions` | 40      |
| `api/v2/addresses/{hash}/internal-transactions`    | 40      |
| `api/v2/smart-contracts/verification/config`       | 40      |
| `api/v2/transactions/{hash}/summary`               | 50      |
| `api/v2/transactions/{hash}/raw-trace`             | 50      |
| `api/v2/addresses/{hash}/coin-balance-history`     | 50      |

## Plans

| Plan         | Price   | Credits      | Rate limit (`x-ratelimit-limit`) |
| ------------ | ------- | ------------ | -------------------------------- |
| **Free**     | $0      | 100K / day   | 5 rps                            |
| **Builder**  | $49/mo  | 100M / month | 15 rps                           |
| **Pro**      | $199/mo | 500M / month | 30 rps                           |
| **Business** | $999/mo | 3B / month   | 50 rps                           |

Some chains are plan-gated on the keyed gateway: plans below Builder get HTTP `402` "requires Builder/Business/Pro plan"
for at least Polygon PoS (`137`), Base (`8453`), and ZKsync Era (`324`). Treat `402` as a coverage gap for that route.
Proceed to the next route. Do not retry. The gateway's bot protection also rejects Python's default `urllib` user-agent.
Scripted requests must send an explicit `User-Agent` header.

Public per-instance hosts do not meter credits but throttle keyless traffic per IP, including hosted `*.blockscout.com`
subdomains. The backend default is **300 requests per minute** (`API_RATE_LIMIT_BY_IP` over a `1m` window). Operators
may change it. Exceeding it returns `429`. Their bot protection can also return `403` with an HTML "Just a moment..."
challenge instead of JSON. Switch to the keyed gateway rather than backing off repeatedly.

## Response Headers (PRO host)

Every PRO call returns these headers. Read them instead of guessing the tier or remaining budget:

| Header                  | Meaning                                |
| ----------------------- | -------------------------------------- |
| `x-ratelimit-limit`     | Requests/sec for the plan (5/15/30/50) |
| `x-ratelimit-remaining` | Requests left in the current second    |
| `x-ratelimit-reset`     | Seconds until the window resets        |
| `x-credits-remaining`   | Credits left in the current window     |

## Authoritative Docs

- Index: <https://docs.blockscout.com/llms.txt>
- PRO routes & credits: <https://docs.blockscout.com/devs/pro-api-responses-and-routes>
- Per-instance schema: `https://{instance}/api-docs`
- OpenAPI: <https://docs.blockscout.com/openapi-specs/pro-api.yaml>
