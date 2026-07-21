# Etherscan API V2

## Overview

Query blockchain data using Etherscan's unified API V2. This skill covers:

- Native ETH balance queries
- ERC-20 token balance queries (single contract on every plan; full holdings on PRO)
- Transaction history queries (normal, internal, ERC-20/ERC-721/ERC-1155 transfers)
- First-funding lookup for an address (PRO `fundedby` with a 2-call free-tier fallback)
- Target-chain support via the `chainid` parameter
- Auto-detection of Free vs Lite vs PRO so paid-only chains and PRO-only endpoints are used when available
- Token reputation availability: Etherscan exposes this only through paid metadata surfaces, not Lite

**Scope:** Read-only account queries. For other Etherscan API features, consult the fallback documentation.

## Prerequisites

### API Key Validation

Before making any API call, verify the `ETHERSCAN_API_KEY` environment variable is set:

```bash
if [ -z "$ETHERSCAN_API_KEY" ]; then
  echo "Error: ETHERSCAN_API_KEY environment variable is not set."
  echo "Get a free API key at: https://etherscan.io/myapikey"
  exit 1
fi
```

If the environment variable is missing, inform the user and halt execution.

### Plan Detection

Run the detection helper **once per session** and cache the result. It maps `getapilimit` → plan tier and probes a Base
balance call to disambiguate Free from Lite:

```bash
scripts/etherscan-detect-plan.sh
```

Output (key=value lines):

```
plan=lite
credit_limit=100000
credits_used=4
credits_available=99996
limit_interval=daily
interval_expiry=14:38:10
pro_endpoints=false
paid_chains=true
```

`plan` is one of `free`, `lite`, `standard`, `advanced`, `professional`, `pro_plus`, `enterprise`, `unknown`. Two
boolean fields gate behavior:

- `paid_chains=true` — paid-only chains (Base, OP, Avalanche, BNB) are queryable. True for Lite and all higher tiers.
- `pro_endpoints=true` — PRO-only actions (`addresstokenbalance`, `balancehistory`, `tokenholderlist`, `fundedby`,
  daily-stats endpoints, etc.) are callable. True for Standard and higher; **false on Lite**.

**Manual detection** (if the script is unavailable):

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=getapilimit&action=getapilimit&apikey=$ETHERSCAN_API_KEY"
# → {"status":"1","message":"OK","result":{"creditsUsed":1,"creditsAvailable":99999,"creditLimit":100000,"limitInterval":"daily","intervalExpiryTimespan":"07:20:05"}}
```

| `creditLimit` | Plan         | Paid-only chains | PRO endpoints |
| ------------- | ------------ | ---------------- | ------------- |
| 100,000       | Free or Lite | Probe to confirm | No            |
| 200,000       | Standard     | Yes              | Yes           |
| 500,000       | Advanced     | Yes              | Yes           |
| 1,000,000     | Professional | Yes              | Yes           |
| 1,500,000     | Pro Plus     | Yes              | Yes           |
| > 1,500,000   | Enterprise   | Yes              | Yes           |

Free and Lite both report `creditLimit: 100000`. Lite ($49/mo) raises rate-limit-per-second (5 vs 3) **and unlocks every
supported chain** (Base, OP, Avalanche, BNB), but does **not** add PRO endpoints — those start at Standard. To
disambiguate, attempt a paid-chain balance call (e.g., `chainid=8453`): status=1 → Lite, status=0 → Free. To probe PRO
instead, the failure response is `"Sorry, it looks like you are trying to access an API Pro endpoint."`.

`getapilimit` itself consumes 1 credit (plus 1 more for the paid-chain probe), so do not re-run mid-session.

## Chain Inference

Do not default to Ethereum Mainnet. Always infer the chain from the user's prompt before making any API call.

### Inference Rules

1. **Explicit chain mention** — If the user mentions a chain name (e.g., "on Polygon", "Arbitrum balance", "Base
   chain"), use that chain.
2. **Chain-specific tokens** — Some tokens exist primarily on specific chains:
   - POL → Polygon (137)
   - ARB → Arbitrum One (42161)
   - OP → OP Mainnet (10)
   - AVAX → Avalanche C-Chain (43114)
   - BNB → BNB Smart Chain (56)
   - SONIC → Sonic (146)
   - SEI → Sei (1329)
   - MON → Monad (143)
3. **Contract address patterns** — If the user provides a contract address, consider asking which chain it's deployed on
   (many contracts exist on multiple chains).
4. **Testnet keywords** — Testnets are outside this skill's target list. Ask the user to file a feature request instead
   of querying them.
5. **Ambiguous cases** — If the chain cannot be inferred, **ask the user** before proceeding. Do not assume Ethereum
   Mainnet.

### Unsupported Chains

If the user references a chain that is not in `references/generated/target-mainnets.json`, halt and ask them to file a
feature request in <https://github.com/PaulRBerg/agent-skills>. Do not query Etherscan, Blockscout, Bungee, Chainlist,
web search, or public RPCs for non-target chains.

If the user references a **target EVM chain** that Etherscan API V2 does not cover, do **not** halt. Prefer Blockscout
(`references/explorers/blockscout-api.md`) before direct RPC. If Blockscout doesn't index the target chain either, fall
back to direct RPC calls against the target chain's default public RPC:

1. Resolve the chain via `references/generated/target-mainnets.json` and `references/generated/chain-aliases.json` to
   get the default public RPC, chain ID, native currency symbol, and explorer URL.
2. Issue equivalent JSON-RPC calls (e.g., `eth_getBalance`, `eth_getLogs`, `eth_getTransactionByHash`) against that RPC
   using `curl` or the `cast` CLI from the `cli-cast` skill.
3. Note in the response that the data came from the chain's public RPC, not Etherscan, so PRO-style aggregations (full
   token holdings, first-funding lookup) are unavailable and must be derived manually from logs/transactions if needed.

If the user references a **non-EVM chain**, do not use this skill:

```
The chain "[chain name]" is outside the evm-atlas target list.
Please file a feature request in https://github.com/PaulRBerg/agent-skills.
```

For the target-filtered list of Etherscan-supported chains and their IDs, see
`references/generated/etherscan-chains.md`.

## API Base URL

All requests use the unified V2 endpoint:

```
https://api.etherscan.io/v2/api
```

The `chainid` parameter determines which blockchain to query.

## ETH Balance Query

Query native ETH (or native token) balance for an address.

### Endpoint Parameters

| Parameter | Required | Default  | Description                                                                                                                                           |
| --------- | -------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `chainid` | No       | `1`      | Chain ID (see etherscan-chains.md)                                                                                                                    |
| `module`  | Yes      | -        | Set to `account`                                                                                                                                      |
| `action`  | Yes      | -        | Set to `balance`                                                                                                                                      |
| `address` | Yes      | -        | Wallet address (supports up to 20 comma-separated)                                                                                                    |
| `tag`     | No       | `latest` | `latest` or hex block number. On free/Lite, only the last 128 blocks are queryable; older history needs the `balancehistory` PRO endpoint (Standard+) |
| `apikey`  | Yes      | -        | API key from `$ETHERSCAN_API_KEY`                                                                                                                     |

### Single Address Query

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=balance&address=0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe&tag=latest&apikey=$ETHERSCAN_API_KEY"
```

### Multi-Address Query (up to 20)

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=balancemulti&address=0xaddress1,0xaddress2,0xaddress3&tag=latest&apikey=$ETHERSCAN_API_KEY"
```

### Response Format

**Single address:**

```json
{
  "status": "1",
  "message": "OK",
  "result": "172774397764084972158218"
}
```

**Multi-address:**

```json
{
  "status": "1",
  "message": "OK",
  "result": [
    { "account": "0xaddress1", "balance": "1000000000000000000" },
    { "account": "0xaddress2", "balance": "2500000000000000000" }
  ]
}
```

## ERC-20 Token Balance Query

Query ERC-20 token balance for an address.

### Endpoint Parameters

| Parameter         | Required | Default  | Description                        |
| ----------------- | -------- | -------- | ---------------------------------- |
| `chainid`         | No       | `1`      | Chain ID (see etherscan-chains.md) |
| `module`          | Yes      | -        | Set to `account`                   |
| `action`          | Yes      | -        | Set to `tokenbalance`              |
| `contractaddress` | Yes      | -        | ERC-20 token contract address      |
| `address`         | Yes      | -        | Wallet address to query            |
| `tag`             | No       | `latest` | Block tag                          |
| `apikey`          | Yes      | -        | API key from `$ETHERSCAN_API_KEY`  |

### Example Query

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=tokenbalance&contractaddress=0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48&address=0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe&tag=latest&apikey=$ETHERSCAN_API_KEY"
```

### Response Format

```json
{
  "status": "1",
  "message": "OK",
  "result": "135499000000"
}
```

### Full Holdings

`tokenbalance` returns the balance for **one** ERC-20 contract at a time. To list **every** token an address holds:

| Action                   | Returns                                                    |
| ------------------------ | ---------------------------------------------------------- |
| `addresstokenbalance`    | All ERC-20 holdings (token, quantity, decimals, USD price) |
| `addresstokennftbalance` | All ERC-721 collection holdings and counts                 |

**Use only when `pro_endpoints=true`** from plan detection. Both require Standard plan or higher and are throttled to
**2 calls/second** regardless of tier.

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=addresstokenbalance&address=0x...&page=1&offset=100&apikey=$ETHERSCAN_API_KEY"
```

When `pro_endpoints=false`, fall back to looping `tokenbalance` over a known token contract list.

## Transaction History Queries

Query an address's transaction history. Five actions are available under `module=account`:

| Action           | Returns                                    |
| ---------------- | ------------------------------------------ |
| `txlist`         | Normal (external) transactions             |
| `txlistinternal` | Internal transactions (contract-initiated) |
| `tokentx`        | ERC-20 token transfer events               |
| `tokennfttx`     | ERC-721 (NFT) token transfer events        |
| `token1155tx`    | ERC-1155 token transfer events             |

### Endpoint Parameters

| Parameter         | Required | Default     | Description                                                  |
| ----------------- | -------- | ----------- | ------------------------------------------------------------ |
| `chainid`         | No       | `1`         | Chain ID (see etherscan-chains.md)                           |
| `module`          | Yes      | -           | Set to `account`                                             |
| `action`          | Yes      | -           | One of the actions above                                     |
| `address`         | Yes      | -           | Wallet address                                               |
| `contractaddress` | No       | -           | Token contract filter (`tokentx`/`tokennfttx`/`token1155tx`) |
| `startblock`      | No       | `0`         | Starting block number                                        |
| `endblock`        | No       | `999999999` | Ending block number                                          |
| `page`            | No       | `1`         | Page number for pagination                                   |
| `offset`          | No       | `100`       | Results per page (see free-tier limit note below)            |
| `sort`            | No       | `asc`       | `asc` or `desc` by block number                              |
| `apikey`          | Yes      | -           | API key from `$ETHERSCAN_API_KEY`                            |

> **Pagination cap by plan (effective July 1, 2026):** `offset` maximum is `1000` for free-tier accounts and `10000` for
> paid tiers (Lite included) on `txlist`, `txlistinternal`, `tokentx`, `tokennfttx`, `token1155tx`, and other list
> endpoints. When `plan=free`, paginate in batches ≤ 1,000.

### Example Query

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=txlist&address=0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe&startblock=0&endblock=999999999&page=1&offset=100&sort=desc&apikey=$ETHERSCAN_API_KEY"
```

### Response Format

`result` is an array of transaction objects. Each contains a Unix `timeStamp` (seconds, as a string) and chain-specific
fields (`hash`, `from`, `to`, `value`, `gasUsed`, etc.).

```json
{
  "status": "1",
  "message": "OK",
  "result": [
    {
      "blockNumber": "18000000",
      "timeStamp": "1693526400",
      "hash": "0x...",
      "from": "0x...",
      "to": "0x...",
      "value": "1000000000000000000",
      "gasUsed": "21000"
    }
  ]
}
```

### Timestamp Conversion

`timeStamp` is a Unix epoch in seconds. Always produce **timezone-aware UTC datetimes**.

```python
from datetime import datetime, timezone

dt = datetime.fromtimestamp(int(tx["timeStamp"]), tz=timezone.utc)
```

Do **not** use `datetime.utcfromtimestamp()` — it returns a naive datetime and is deprecated in Python 3.12+.

```bash
# Shell equivalent (GNU date)
date -u -d "@1693526400" --iso-8601=seconds
# macOS / BSD date
date -u -r 1693526400 +"%Y-%m-%dT%H:%M:%SZ"
```

## NFT Transfer History

Fetch historical ERC-721 or ERC-1155 transfers for an address. Both actions share the parameter table in the previous
section; pass `contractaddress` to filter by collection. Pagination caps (1,000 free / 10,000 paid) and the
`startblock`/`endblock`/`page`/`offset`/`sort` semantics are identical to `txlist`.

### ERC-721 Transfers (`tokennfttx`)

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=tokennfttx&address=0x6975be450864c02b4613023c2152ee0743572325&contractaddress=0x06012c8cf97bead5deae237070f9587f8e7a266d&startblock=0&endblock=999999999&page=1&offset=100&sort=asc&apikey=$ETHERSCAN_API_KEY"
```

Response entry (one per `Transfer` event involving the address):

```json
{
  "blockNumber": "4708120",
  "timeStamp": "1512907118",
  "hash": "0x031e6968...",
  "nonce": "0",
  "blockHash": "0x4be19c27...",
  "from": "0xb1690c08e213a35ed9bab7b318de14420fb57d8c",
  "contractAddress": "0x06012c8cf97bead5deae237070f9587f8e7a266d",
  "to": "0x6975be450864c02b4613023c2152ee0743572325",
  "tokenID": "202106",
  "tokenName": "CryptoKitties",
  "tokenSymbol": "CK",
  "tokenDecimal": "0",
  "transactionIndex": "81",
  "gas": "158820",
  "gasPrice": "40000000000",
  "gasUsed": "60508",
  "cumulativeGasUsed": "4880352",
  "input": "deprecated",
  "methodId": "0x454a2ab3",
  "functionName": "bid(uint256 _tokenId)",
  "confirmations": "18759540"
}
```

NFT-specific fields: `contractAddress` (collection), `tokenID` (per-NFT identifier), `tokenName`, `tokenSymbol`,
`tokenDecimal` (always `"0"` for ERC-721).

### ERC-1155 Transfers (`token1155tx`)

Same parameter shape — swap `action=token1155tx`. ERC-1155 differs from ERC-721 in two response fields:

- **`tokenValue`** (string) — quantity transferred for this `tokenID`. Required because ERC-1155 is semi-fungible; a
  single transfer can move N copies of one ID. **Not present in ERC-721 responses.**
- **`tokenDecimal`** is omitted (ERC-1155 has no decimals concept).

```json
{
  "blockNumber": "...",
  "timeStamp": "...",
  "hash": "...",
  "from": "...",
  "to": "...",
  "contractAddress": "0x76be3b62873462d2142405439777e971754e8e77",
  "tokenID": "10371",
  "tokenValue": "1",
  "tokenName": "...",
  "tokenSymbol": "...",
  "...": "(other tx-level fields identical to tokennfttx)"
}
```

`TransferBatch` events (multiple IDs in one tx) appear as **multiple result entries sharing the same `hash`** — one per
`(tokenID, tokenValue)` pair. Group by `hash` to reconstruct the batch.

### Filtering by Collection or Token ID

- **By collection** — pass `contractaddress=<collection>`. The API filters server-side; omit to fetch transfers across
  all collections.
- **By token ID** — no server-side filter exists. Fetch the collection's transfers and filter `result[].tokenID == <id>`
  client-side. For high-volume collections, narrow with `startblock`/`endblock` first.
- **Mint vs burn vs transfer** — derive from `from`/`to`:
  - `from == 0x0000...0000` → mint
  - `to == 0x0000...0000` → burn
  - otherwise → transfer

### Cost & Limits

Standard list-endpoint pricing — 1 credit per call, same rate-limit tier as `txlist`. Not a PRO endpoint; available on
Free and Lite for Etherscan-supported target chains (paid-chain restriction still applies to Base/OP/Avalanche/BNB).

## First Funding Transaction

Identify the earliest transaction that sent native value to an address — useful for fund-origin tracing, provenance, or
compliance checks. Cost is **1 API call** (PRO) or **2 API calls** (fallback).

### Preferred: `fundedby` (PRO endpoint)

Returns the address, tx hash, block, timestamp, and value of the transaction that first funded an EOA. Single call,
structured response.

| Parameter | Required | Default | Description                         |
| --------- | -------- | ------- | ----------------------------------- |
| `chainid` | No       | `1`     | Chain ID (see etherscan-chains.md)  |
| `module`  | Yes      | -       | Set to `account`                    |
| `action`  | Yes      | -       | Set to `fundedby`                   |
| `address` | Yes      | -       | EOA address (contracts unsupported) |
| `apikey`  | Yes      | -       | API key from `$ETHERSCAN_API_KEY`   |

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=fundedby&address=0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97&apikey=$ETHERSCAN_API_KEY"
```

Response:

```json
{
  "status": "1",
  "message": "OK",
  "result": {
    "block": 53708500,
    "timeStamp": "1708349932",
    "fundingAddress": "0x6969174fd72466430a46e18234d0b530c9fd5f49",
    "fundingTxn": "0xbc0ca4a67eb1555920552246409626cd60df01314dd2bcdb99718b506d9c9946",
    "value": "1000000000000000"
  }
}
```

**Requirements & limits:**

- PRO endpoint — requires Standard plan or higher (`pro_endpoints=true` from plan detection).
- Throttled to **2 calls/second** regardless of paid tier.
- **EOA only.** Contract addresses return an error; use the fallback below.

### Fallback: scan ASC normal + internal transactions

When `pro_endpoints=false` (free/Lite) or the address is a contract, scan both transaction lists ascending and pick the
earliest qualifying incoming entry. Two API calls per address.

```bash
# Earliest normal txs involving the address
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=txlist&address=0x...&startblock=0&endblock=999999999&page=1&offset=10&sort=asc&apikey=$ETHERSCAN_API_KEY"

# Earliest internal txs involving the address
curl -s "https://api.etherscan.io/v2/api?chainid=1&module=account&action=txlistinternal&address=0x...&startblock=0&endblock=999999999&page=1&offset=10&sort=asc&apikey=$ETHERSCAN_API_KEY"
```

For each response, pick the first entry where **all** of the following hold:

- `to.toLowerCase() == address.toLowerCase()` — incoming, not outgoing.
- `value` (in wei) is greater than `0` — actual funding, not a zero-value call.
- `isError == "0"` (omit this filter for internal txs, which use `isError` differently or not at all).

The funding tx is whichever match has the lower `blockNumber`; break ties by `transactionIndex` (normal txs) or by list
order (internal txs).

**Why both lists:** An address may be funded externally (normal tx) or internally (contract sent ETH — common for CEX
withdrawals routed through proxy/router contracts, contract deployments with non-zero `msg.value`, or SELFDESTRUCT
refunds). Checking only `txlist` will miss internally-funded addresses.

**Why `offset=10`, not `1`:** A `txlist` query returns every tx involving the address, including outgoing ones. The very
first entry is occasionally outgoing (e.g., the address was internally pre-funded), so fetch a small window and scan for
the first incoming match.

**Edge cases:**

- **No qualifying entry in the first 10** — extend with `offset=100` and `page=1`, or paginate further. In practice, >
  10 outgoing-before-incoming is exceedingly rare.
- **Genesis allocation** — pre-mined balances do not appear in either list. The address shows a balance with no funding
  tx; report this explicitly.
- **Token-only funding** — `fundedby` and this fallback only consider native value. If the address was bootstrapped with
  ERC-20 transfers alone (rare for EOAs since gas is needed), repeat the fallback against `tokentx`.

## Multi-Chain Usage

Specify the `chainid` parameter to query different blockchains.

### Target Chain IDs (Free Tier)

See `references/generated/etherscan-chains.md` for the free-tier target chain list with chain IDs.

### Example: Polygon Query

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=137&module=account&action=balance&address=0x...&tag=latest&apikey=$ETHERSCAN_API_KEY"
```

For the target-filtered list of supported chains, see `references/generated/etherscan-chains.md`.

## Wei to Human-Readable Conversion

API responses return balances in the smallest unit (wei for ETH, smallest decimals for tokens).

### ETH Conversion

Divide by 10^18:

```bash
# Using bc for precision
echo "scale=18; 172774397764084972158218 / 1000000000000000000" | bc
# Result: 172774.397764084972158218
```

### ERC-20 Conversion

Divide by 10^decimals (typically 18, but varies per token):

| Token       | Decimals |
| ----------- | -------- |
| Most tokens | 18       |
| USDC, USDT  | 6        |
| WBTC        | 8        |

```bash
# USDC example (6 decimals)
echo "scale=6; 135499000000 / 1000000" | bc
# Result: 135499.000000
```

## Output Formatting

**Default behavior:** Present results in a Markdown table:

```markdown
| Address       | Balance (ETH) | Chain    |
| ------------- | ------------- | -------- |
| 0xde0B...7BAe | 172,774.40    | Ethereum |
| 0xabc1...2def | 50.25         | Polygon  |
```

**User preference:** If the user requests a specific format (JSON, CSV, plain text, etc.), use that format instead. Do
not generate a Markdown table when the user specifies an alternative output format.

## Plan-Gated Capabilities

Decisions in this section depend on the cached output of `scripts/etherscan-detect-plan.sh`.

### Paid-Only Chains

Four target mainnets require any paid Etherscan plan. **Lite ($49/mo) is sufficient** — it grants access to every
Etherscan-supported target chain at the same 100,000 daily-credit limit as Free. Data endpoints (balance, txlist, logs,
etc.) fail only when `plan=free` (i.e., `paid_chains=false`). See `references/generated/etherscan-chains.md` for the
paid-plan target chain list with chain IDs.

**Exception:** `module=contract` endpoints (`getsourcecode`, `getabi`, etc.) work on **all** chains for every plan
including free. The paid-plan requirement applies only to data endpoints.

If `paid_chains=false` (i.e., `plan=free`) and the user requests a data query on the chains above, route to Blockscout
(`references/explorers/blockscout-api.md`) before direct RPC. Only mention upgrading to Lite or higher if the user
specifically needs Etherscan as the source.

### PRO-Only Endpoints

When `pro_endpoints=true`, the following actions become available (non-exhaustive — see
`https://docs.etherscan.io/api-pro/api-pro` for the full list):

| Module       | Action(s)                                                                     | Use case                                                 |
| ------------ | ----------------------------------------------------------------------------- | -------------------------------------------------------- |
| `account`    | `addresstokenbalance`, `addresstokennftbalance`, `balancehistory`, `fundedby` | Full holdings, historical balances, first-funding lookup |
| `token`      | `tokenholderlist`, `tokeninfo`, `tokensupplyhistory`, `tokenbalancehistory`   | Token analytics                                          |
| `block`      | `dailyavgblocksize`, `dailyblkcount`, `dailyblockrewards`, etc.               | Daily block stats                                        |
| `stats`      | `dailytxnfee`, `dailynewaddress`, `dailynetutilization`, etc.                 | Network-wide daily metrics                               |
| `gastracker` | `dailyavggaslimit`, `dailygasused`, `dailyavggasprice`                        | Daily gas metrics                                        |

When `pro_endpoints=false` (free or Lite), prefer the non-PRO equivalents listed in this skill or fall back to per-token
loops.

### Token Reputation / Metadata

Etherscan's token reputation badges are **not available on Lite**. The documented API surfaces are:

| Surface              | Endpoint/action                           | Minimum plan | Notes                                                                                            |
| -------------------- | ----------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------ |
| Address Metadata API | `module=nametag&action=getaddresstag`     | Pro Plus     | Query the token contract address; response includes numeric `reputation` and `other_attributes`. |
| Metadata CSV export  | `module=nametag&action=exportaddresstags` | Enterprise   | Bulk export; `other_attributes` can include `TR` token reputation values.                        |
| Token info           | `module=token&action=tokeninfo`           | Standard     | Returns project/social metadata and `blueCheckmark`, but not the token reputation badge.         |

Do not tell Lite users they can fetch token reputation from Etherscan API. Lite only unlocks paid Etherscan target
chains and higher community rate limits; it does not unlock API Pro endpoints, Pro Plus address metadata, or Enterprise
metadata CSV exports.

### All Plans

All other Etherscan-supported target chains are available on every plan including Free. On Lite and higher, the
paid-only target chains above also become available. See `references/generated/etherscan-chains.md` for the
target-filtered list with chain IDs.

## Error Handling

### Common Error Responses

| Status | Message                  | Cause                           |
| ------ | ------------------------ | ------------------------------- |
| `0`    | `NOTOK`                  | Invalid API key or rate limited |
| `0`    | `Invalid address format` | Malformed address               |
| `0`    | `No transactions found`  | Address has no activity         |

### Rate Limits by Plan

| Plan         | Calls/second | Daily calls |
| ------------ | ------------ | ----------- |
| Free         | 3            | 100,000     |
| Lite         | 5            | 100,000     |
| Standard     | 10           | 200,000     |
| Advanced     | 20           | 500,000     |
| Professional | 30           | 1,000,000   |
| Pro Plus     | 30           | 1,500,000   |
| Enterprise   | custom       | unmetered   |

PRO endpoints (`addresstokenbalance`, etc.) are throttled to **2 calls/second** regardless of tier. See
`https://docs.etherscan.io/resources/rate-limits` for the authoritative schedule.

If rate limited, wait briefly and retry.

## Reference Files

- **`references/generated/etherscan-chains.md`** - Target-filtered list of supported chains with chain IDs
- **`scripts/etherscan-detect-plan.sh`** - Plan-tier detection helper (run once per session)

## Fallback Documentation

For use cases not covered by this skill (transaction history, contract verification, gas estimates, etc.), fetch the
AI-friendly documentation:

```
https://docs.etherscan.io/llms.txt
```

Use `WebFetch` to retrieve this documentation for extended API capabilities.
