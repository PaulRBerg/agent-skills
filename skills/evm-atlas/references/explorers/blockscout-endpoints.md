# Endpoints, Credits & Limits

Bases:

- Unified PRO: `https://api.blockscout.com/{chain_id}/api/v2/...` (key required)
- Etherscan-V2 alias: `https://api.blockscout.com/v2/api?chain_id={id}&module=...&action=...` (key required;
  `{status,message,result}` shape)
- Per-instance: `https://{instance}/api/v2/...` (no key) and `https://{instance}/api?module=...` (no key)

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

Pagination is keyset: responses include `next_page_params` (50/page); append those fields as query params for the next
page. `null` means last page. No `sort` param — newest-first.

## Etherscan-Compatible Actions

Available on both `/{chain_id}/api?module=...` and the `/v2/api?chain_id=...` alias. `module=account` actions:
`balance`, `balancemulti`, `tokenbalance`, `tokenlist`, `txlist`, `txlistinternal`, `tokentx`, `tokennfttx`,
`token1155tx`. Other modules: `logs/getLogs`, `contract/getabi`, `contract/getsourcecode`, `block/*`, `stats/*`,
`token/*`. The compat layer is legacy and does not implement every Etherscan action — prefer native v2; it supports
`page`/`offset`/`sort` (asc/desc), which native v2 does not.

## Credit Costs (PRO host)

Default **20 credits** per call. Exceptions:

| Endpoint                                           | Credits |
| -------------------------------------------------- | ------- |
| (default — all unlisted)                           | 20      |
| `api/v2/search/quick`                              | 25      |
| `api/v2/transactions/{hash}/logs`                  | 30      |
| `api/v2/transactions/{hash}/token-transfers`       | 30      |
| `api/v2/transactions/{hash}/internal-transactions` | 40      |
| `api/v2/transactions/{hash}/raw-trace`             | 50      |
| `api/v2/addresses/{hash}/coin-balance-history`     | 50      |

## Plans

| Plan         | Price   | Credits      | Rate limit (`x-ratelimit-limit`) |
| ------------ | ------- | ------------ | -------------------------------- |
| **Free**     | $0      | 100K / day   | 5 rps                            |
| **Standard** | $49/mo  | 100M / month | 15 rps                           |
| **Pro**      | $199/mo | 500M / month | 30 rps                           |

Public per-instance hosts (no key) are not credit-metered but limited to **3 rps / 300 per minute** per IP.

## Response Headers (PRO host)

Returned on every PRO call — read them instead of guessing tier or remaining budget:

| Header                  | Meaning                             |
| ----------------------- | ----------------------------------- |
| `x-ratelimit-limit`     | Requests/sec for the plan (5/15/30) |
| `x-ratelimit-remaining` | Requests left in the current second |
| `x-ratelimit-reset`     | Seconds until the window resets     |
| `x-credits-remaining`   | Credits left in the current window  |

## Authoritative Docs

- Index: <https://docs.blockscout.com/llms.txt>
- PRO routes & credits: <https://docs.blockscout.com/devs/pro-api-responses-and-routes>
- Per-instance schema: `https://{instance}/api-docs`
- OpenAPI: <https://docs.blockscout.com/openapi-specs/pro-api.yaml>
