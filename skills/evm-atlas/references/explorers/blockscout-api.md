# Blockscout API

## Overview

Query blockchain data across any EVM chain that Blockscout indexes. Blockscout exposes three compatible surfaces:

- **Native REST API v2** (`/api/v2/...`) — rich JSON, the recommended surface. Returns balances, full token holdings,
  transactions, and transfers with embedded token/exchange-rate metadata.
- **Etherscan-compatible RPC** (`/api?module=...&action=...`) — legacy `{status,message,result}` shape. Useful for
  porting existing Etherscan code; superseded by v2.
- **Unified PRO API** (`https://api.blockscout.com/...`) — a single keyed host fronting both of the above across major
  chains, selected by `chain_id`.

This skill covers read-only account/address queries: native balance, ERC-20/721/1155 holdings and transfers, transaction
history, and first-funding tracing.

**Relationship to Etherscan (`references/explorers/etherscan-api.md`):** Same problem space, different explorer. Prefer
Blockscout when the target chain is **not** on Etherscan, when a paid Etherscan target chain needs free-tier data, when
you want full token holdings on the free tier, or when the user names Blockscout/Chainscout. The two surfaces are
interchangeable for native-balance and transfer queries.

## Prerequisites

### API Key

A free Blockscout PRO key (`proapi_…`) is expected in `$BLOCKSCOUT_API_KEY`:

```bash
if [ -z "$BLOCKSCOUT_API_KEY" ]; then
  echo "Error: BLOCKSCOUT_API_KEY is not set."
  echo "Get a free key at: https://dev.blockscout.com/"
  exit 1
fi
```

The key is **only** required for the unified PRO host (`api.blockscout.com`), which returns
`401 {"error":"Unauthorized"}` without it. Per-instance public hosts (e.g., `eth.blockscout.com`) need **no key** — see
[Per-Instance Fallback](#per-instance-fallback-any-chain-no-key). If the key is missing, fall back to per-instance hosts
rather than halting.

### Plan & Credit Detection

Run once per session and cache the result. It reads rate-limit/credit headers returned on every PRO response:

```bash
scripts/blockscout-detect-plan.sh
```

Output (key=value lines):

```
plan=free
rate_limit_rps=5
rate_limit_remaining=3
rate_limit_reset=441
credits_remaining=99880
```

`x-ratelimit-limit` maps directly to plan tier; see the Plans and Credit Costs tables in
`references/explorers/blockscout-endpoints.md`.

At the default 20 credits/call, the free 100K/day tier ≈ 5,000 calls/day. `blockscout-detect-plan.sh` itself costs ~20
credits — do not re-run mid-session.

Per-instance public hosts are not credit-metered but are rate-limited to **3 req/s (300/min) per IP**.

## Choosing an Endpoint

Decide per query:

| Situation                                                              | Use                                                                                 |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Chain is on the PRO host (eth, OP, Polygon, Base, Arbitrum, Gnosis, …) | **Unified PRO** `https://api.blockscout.com/{chain_id}/api/v2/...` + key            |
| Porting existing Etherscan **V2** code (minimal diff)                  | **Etherscan-V2 alias** `https://api.blockscout.com/v2/api?chain_id={id}&module=...` |
| Chain returns `404` on the PRO host, or no key available               | **Per-instance** `https://{instance}/api/v2/...` (no key) — resolve via Chainscout  |

The PRO host fronts major Blockscout-hosted target chains but **not all** of them. On any `404`, fall back to the
per-instance host resolved through `scripts/resolve-chain.sh`. If the target chain is absent from Chainscout, use
Etherscan (`references/explorers/etherscan-api.md`) or the `primaryPublicRpc` from
`references/generated/target-mainnets.json`. If the requested chain is not in
`references/generated/target-mainnets.json`, stop and ask the user to file a feature request in
<https://github.com/PaulRBerg/agent-skills>.

## Chain Resolution

Do **not** default to Ethereum Mainnet. Infer the chain from the prompt first (same rules as
`references/explorers/etherscan-api.md`: explicit chain mention, chain-specific tokens like POL→137 / ARB→42161, testnet
keywords). If ambiguous, ask.

Two-step resolution:

1. **Name → `chain_id`** — use `references/generated/target-mainnets.json` and
   `references/generated/chain-aliases.json`.
2. **`chain_id` -> instance URL** (only needed for the per-instance route) — use the target-gated Chainscout helper:

```bash
scripts/resolve-chain.sh 100
```

```
chain_id=100
name=Gnosis
native_currency=XDAI
instance_url=https://gnosis.blockscout.com/
hosted_by=blockscout
is_testnet=false
layer=1
rollup_type=
```

`hosted_by=blockscout` indicates the chain is a candidate for the PRO host; community-hosted chains (`hosted_by` other
than `blockscout`) are per-instance only. Chainscout indexes many networks, but this skill only uses target chains — see
`references/generated/blockscout-chains.md`.

## Authentication

On the PRO host, pass the key either way:

```bash
# Query parameter
curl -s "https://api.blockscout.com/1/api/v2/addresses/0xADDR?apikey=$BLOCKSCOUT_API_KEY"
# Authorization header (preferred — keeps the key out of URLs/logs)
curl -s -H "authorization: Bearer $BLOCKSCOUT_API_KEY" "https://api.blockscout.com/1/api/v2/addresses/0xADDR"
```

## Native REST API v2

The recommended surface. Base URL is `https://api.blockscout.com/{chain_id}/api/v2` (PRO) or `https://{instance}/api/v2`
(per-instance). Examples below use the PRO host; swap the base for the per-instance host and drop the key when needed.

### Address Overview (native balance + metadata)

```bash
curl -s -H "authorization: Bearer $BLOCKSCOUT_API_KEY" \
  "https://api.blockscout.com/1/api/v2/addresses/0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"
```

```json
{
  "coin_balance": "9774452722498812330011",
  "exchange_rate": "1974.4",
  "is_contract": true,
  "ens_domain_name": null,
  "creation_transaction_hash": "0x9c81…",
  "creator_address_hash": "0x5AbF…",
  "has_tokens": true,
  "has_token_transfers": true
}
```

`coin_balance` is the native balance in wei. See [Unit Conversion](#unit-conversion).

### Token Holdings

```bash
# Full holdings in one call (array) — no PRO gating, unlike Etherscan's addresstokenbalance
curl -s -H "authorization: Bearer $BLOCKSCOUT_API_KEY" \
  "https://api.blockscout.com/1/api/v2/addresses/0xADDR/token-balances"

# Paginated + filterable variant
curl -s -H "authorization: Bearer $BLOCKSCOUT_API_KEY" \
  "https://api.blockscout.com/1/api/v2/addresses/0xADDR/tokens?type=ERC-20,ERC-721,ERC-1155"
```

Each entry embeds full token metadata and balance:

```json
[
  {
    "token": {
      "address_hash": "0xC02aaA39…",
      "name": "WETH",
      "symbol": "WETH",
      "decimals": "18",
      "type": "ERC-20",
      "exchange_rate": "1977.19"
    },
    "value": "214140968121599991968",
    "token_id": null,
    "token_instance": null
  }
]
```

For ERC-721/1155, `token_id` and `token_instance` are populated. Divide `value` by `10^decimals` per token.

### Transaction History

```bash
# Normal transactions (filter=to|from to restrict direction)
curl -s -H "authorization: Bearer $BLOCKSCOUT_API_KEY" \
  "https://api.blockscout.com/1/api/v2/addresses/0xADDR/transactions"

# Internal transactions
curl -s -H "authorization: Bearer $BLOCKSCOUT_API_KEY" \
  "https://api.blockscout.com/1/api/v2/addresses/0xADDR/internal-transactions"
```

### Token Transfers (ERC-20 / 721 / 1155)

One endpoint, filtered by `type`:

```bash
curl -s -H "authorization: Bearer $BLOCKSCOUT_API_KEY" \
  "https://api.blockscout.com/1/api/v2/addresses/0xADDR/token-transfers?type=ERC-20"
```

`type` accepts `ERC-20`, `ERC-721`, or `ERC-1155`. Each item carries `block_number`, `timestamp` (ISO-8601 UTC), `from`,
`to`, `total` (`value`/`decimals` for fungible; `token_id` for NFTs), and embedded `token` metadata. Derive mint/burn
from `from`/`to` being the zero address.

### Pagination (keyset)

v2 returns 50 items plus a `next_page_params` object. To fetch the next page, append those fields as query params:

```json
{ "items": [ … ], "next_page_params": { "block_number": 25103884, "index": 1275, "items_count": 50 } }
```

```bash
curl -s -H "authorization: Bearer $BLOCKSCOUT_API_KEY" \
  "https://api.blockscout.com/1/api/v2/addresses/0xADDR/token-transfers?type=ERC-20&block_number=25103884&index=1275&items_count=50"
```

When `next_page_params` is `null`, the last page was reached. There is no `sort` parameter — v2 returns newest-first.

## Etherscan-Compatible Layer

For porting existing Etherscan V2 code (`references/explorers/etherscan-api.md`) with minimal changes, use the
**Etherscan-V2 alias**. It returns the familiar `{status,message,result}` shape:

```bash
curl -s "https://api.blockscout.com/v2/api?chain_id=1&module=account&action=balance&address=0xADDR&apikey=$BLOCKSCOUT_API_KEY"
# → {"message":"OK","result":"9774452722498812330011","status":"1"}
```

Porting checklist from Etherscan V2: change host `api.etherscan.io` → `api.blockscout.com`, use `chain_id` (canonical;
`chainid` is tolerated), and swap the key var. Action → v2 mapping:

| Need                 | Etherscan action            | Native v2 (preferred)                       | Compat action              |
| -------------------- | --------------------------- | ------------------------------------------- | -------------------------- |
| Native balance       | `balance`                   | `addresses/{h}` → `coin_balance`            | `balance`                  |
| Multi balance        | `balancemulti`              | —                                           | `balancemulti`             |
| Single token balance | `tokenbalance`              | —                                           | `tokenbalance`             |
| **All holdings**     | `addresstokenbalance` (PRO) | `addresses/{h}/token-balances` (**free**)   | `tokenlist`                |
| Normal txs           | `txlist`                    | `addresses/{h}/transactions`                | `txlist`                   |
| Internal txs         | `txlistinternal`            | `addresses/{h}/internal-transactions`       | `txlistinternal`           |
| ERC-20 transfers     | `tokentx`                   | `addresses/{h}/token-transfers?type=ERC-20` | `tokentx`                  |
| ERC-721 transfers    | `tokennfttx`                | `…token-transfers?type=ERC-721`             | `tokennfttx`               |
| ERC-1155 transfers   | `token1155tx`               | `…token-transfers?type=ERC-1155`            | `token1155tx`              |
| Logs                 | `getLogs`                   | —                                           | `getLogs`                  |
| ABI / source         | `getabi` / `getsourcecode`  | `smart-contracts/{h}`                       | `getabi` / `getsourcecode` |

Blockscout's compat layer does not implement every Etherscan action; when one is missing, use the native v2 equivalent.
Full endpoint catalog: `references/explorers/blockscout-endpoints.md`.

## First Funding Transaction

Blockscout has **no `fundedby` equivalent**. Use the compat `txlist`/`txlistinternal` with ascending sort (the native v2
surface only sorts newest-first, which is awkward for "earliest"):

```bash
curl -s "https://api.blockscout.com/v2/api?chain_id=1&module=account&action=txlist&address=0xADDR&sort=asc&page=1&offset=10&apikey=$BLOCKSCOUT_API_KEY"
curl -s "https://api.blockscout.com/v2/api?chain_id=1&module=account&action=txlistinternal&address=0xADDR&sort=asc&page=1&offset=10&apikey=$BLOCKSCOUT_API_KEY"
```

Pick the earliest entry where `to == address` (lowercased), `value > 0`, and (normal txs) `isError == "0"`. The funding
tx is the lower `blockNumber` across both lists. Check both because addresses are often funded internally (CEX
router/proxy withdrawals). Genesis-allocated balances appear in neither list — report explicitly.

## Per-Instance Fallback (any chain, no key)

For chains that `404` on the PRO host, or when no key is set, query the chain's own Blockscout instance directly — no
key, every chain Blockscout indexes:

```bash
# 1. Resolve the instance URL
eval "$(scripts/resolve-chain.sh 42170 | sed 's/^/CS_/')"   # CS_instance_url, CS_name, …

# 2. Hit native v2 on that host (no key)
curl -s "${CS_instance_url}api/v2/addresses/0xADDR/token-balances"

# Or the Etherscan-compatible layer on that host
curl -s "${CS_instance_url}api?module=account&action=balance&address=0xADDR"
```

Note `instance_url` already ends in `/`. Per-instance hosts are community-operated for many chains — uptime and indexing
depth vary. Prefer the PRO host when the chain is available there.

## Unit Conversion

Native balances and token `value`s are in the smallest unit. Divide by `10^decimals` (18 for native and most tokens;
USDC/USDT 6; WBTC 8):

```bash
echo "scale=18; 9774452722498812330011 / 1000000000000000000" | bc
# 9774.452722498812330011
```

## Output Formatting

Default to a Markdown table:

```markdown
| Address     | Balance  | Token | Chain    |
| ----------- | -------- | ----- | -------- |
| 0xde0B…7BAe | 9,774.45 | ETH   | Ethereum |
```

If the user requests JSON/CSV/plain text, use that instead.

## Error Handling

| Symptom                              | Cause / Action                                                                        |
| ------------------------------------ | ------------------------------------------------------------------------------------- |
| `401 {"error":"Unauthorized"}`       | Missing/invalid key on PRO host. Set `$BLOCKSCOUT_API_KEY` or use per-instance host.  |
| `404` on `api.blockscout.com/{id}/…` | Target chain not on PRO host. Resolve instance via Chainscout, use per-instance host. |
| `429` / `x-ratelimit-remaining: 0`   | Rate limited. Back off until `x-ratelimit-reset` (seconds).                           |
| `503`                                | Transient PRO-host hiccup. Retry; if persistent, use per-instance.                    |
| Compat `{"status":"0", …}`           | Etherscan-shaped error (`No transactions found`, bad address, etc.).                  |

## Reference Files

- **`references/generated/blockscout-chains.md`** — Target-gated Chainscout registry usage and target-chain
  observations.
- **`references/explorers/blockscout-endpoints.md`** — full native v2 endpoint catalog, compat action list, and
  per-endpoint credit costs.
- **`scripts/blockscout-detect-plan.sh`** — header-based plan/credit detection (run once per session).
- **`scripts/resolve-chain.sh`** — `chain_id` → Blockscout instance URL via Chainscout.

## Fallback Documentation

For features beyond this skill (blocks, smart contracts, search, stats, NFT instances):

- AI-friendly docs index: `https://docs.blockscout.com/llms.txt`
- Per-instance interactive schema: `https://{instance}/api-docs`
- PRO OpenAPI spec: `https://docs.blockscout.com/openapi-specs/pro-api.yaml`

Use `WebFetch` to retrieve these for extended capabilities.
