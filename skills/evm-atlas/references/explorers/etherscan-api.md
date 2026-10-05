# Etherscan API V2

## Overview

Query blockchain data using Etherscan's unified API V2. This skill covers:

- Native ETH balance queries
- ERC-20 token balance queries (single contract on every plan, full holdings on PRO)
- Transaction history queries (normal, internal, ERC-20/ERC-721/ERC-1155 transfers)
- First-funding lookup for an address (PRO `fundedby` with a 2-call free-tier fallback)
- Target-chain support via the `chainid` parameter
- Automatic Free vs Lite vs PRO detection to use paid-only chains and PRO-only endpoints when available
- Token reputation availability: Etherscan exposes this only through paid metadata surfaces, not Lite
- ENS forward resolution for Ethereum mainnet onchain names

**Scope:** Read-only account and ENS queries. For other Etherscan API features, consult the fallback documentation.

This guidance reflects a check against the [changelog](https://docs.etherscan.io/changelog) on 2026-10-05. Apply
scheduled changes by their effective date. Provider membership alone does not prove plan access.

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

If the environment variable is missing, inform the user. Under that condition, halt execution. Never print the key
itself. Presence checks must stay value-free. Do not echo `$ETHERSCAN_API_KEY` or use `${ETHERSCAN_API_KEY:+...}` /
`${ETHERSCAN_API_KEY:-...}` expansions in any command whose output reaches the transcript.

### Plan Detection

Run the detection helper **once per session**. Cache the result. It maps `getapilimit` → plan tier and probes a Base
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
capability fields (`true`, `false`, or `unknown`) control gated behavior. Only `true` authorizes a gated request:

- `paid_chains=true` — paid-chain community endpoints are queryable. True for Lite and all higher tiers. Use
  `references/generated/etherscan-chains.md` and its dated notes to determine which targets require it.
- `pro_endpoints=true` — PRO-only actions (`addresstokenbalance`, `balancehistory`, `tokenholderlist`, `fundedby`,
  daily-stats endpoints, etc.) are callable. True for Standard and higher. **False on Lite**.

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

Free and Lite both report `creditLimit: 100000`. Lite raises the rate limit per second (5 vs 3). It **unlocks every
supported chain's community endpoints**, but does **not** add PRO endpoints. Those start at Standard.

To distinguish Free from Lite, attempt a Base balance call (`chainid=8453`). Success indicates Lite. Only the explicit
`Free API access is not supported for this chain` denial indicates Free. Transport, rate-limit, quota, or other failures
leave `plan=unknown` and `paid_chains=unknown`. Under those failures, `pro_endpoints=false` still follows from the
100,000-credit tier. Do not classify generic `status=0` as Free.

For a PRO probe instead, the failure response is `"Sorry, it looks like you are trying to access an API Pro endpoint."`.

`getapilimit` itself consumes 1 credit (plus 1 more for the paid-chain probe). Do not re-run it mid-session.

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
3. **Contract address patterns** — If the user provides a contract address, consider asking which chain hosts it. Many
   contracts exist on multiple chains.
4. **Testnet keywords** — Testnets are outside this skill's target list. Ask the user to file a feature request instead
   of querying them.
5. **Ambiguous cases** — If the chain cannot be inferred, **ask the user** before proceeding. Do not assume Ethereum
   Mainnet.

### Unsupported Chains

If the user references a chain that is not in `references/generated/target-mainnets.json`, halt. Under that condition,
ask them to file a feature request in <https://github.com/PaulRBerg/agent-skills>. Do not query Etherscan, Blockscout,
Bungee, Chainlist, web search, or public RPCs for non-target chains.

If the user references a **target EVM chain** that Etherscan API V2 does not cover, do **not** halt. Prefer Blockscout
(`references/explorers/blockscout-api.md`) before direct RPC. If Blockscout does not index the target chain either, use
direct RPC calls against the target chain's default public RPC:

1. Resolve the chain via `references/generated/target-mainnets.json` and `references/generated/chain-aliases.json` to
   get the default public RPC, chain ID, native currency symbol, and explorer URL.
2. Issue bounded direct HTTP JSON-RPC calls (e.g., `eth_getBalance`, `eth_getLogs`, `eth_getTransactionByHash`) against
   that RPC. Do not hand the read to `cli-cast`.
3. Note in the response that the data came from the chain's public RPC, not Etherscan. With this source, PRO-style
   aggregations (full token holdings, first-funding lookup) are unavailable. If needed, derive them manually from
   logs/transactions.

If the user references a **non-EVM chain**, do not use this skill:

```
The chain "[chain name]" is outside the evm-atlas target list.
Please file a feature request in https://github.com/PaulRBerg/agent-skills.
```

For the target-filtered list of Etherscan-supported chains and their IDs, see
`references/generated/etherscan-chains.md`. Read its notes as well as its table grouping. Once effective, scheduled
access changes take precedence. A provider deprecation does not remove a chain from this skill's target registry. For a
deprecated provider, route that target to an available indexed fallback or RPC instead. Provider additions do not expand
the target list.

## API Base URL

All requests use the unified V2 endpoint:

```
https://api.etherscan.io/v2/api
```

The `chainid` parameter determines which blockchain to query.

## ENS Forward Resolution

Use `chainid=1&module=ens&action=forwardresolve&name=<name>` for an ENS name or onchain subdomain. This endpoint is
Ethereum-mainnet-only and available on Free and paid plans. Throttle Free requests to 1 call/second.

```bash
curl -sG 'https://api.etherscan.io/v2/api' \
  --data-urlencode 'chainid=1' \
  --data-urlencode 'module=ens' \
  --data-urlencode 'action=forwardresolve' \
  --data-urlencode 'name=etherscan.eth' \
  --data-urlencode "apikey=$ETHERSCAN_API_KEY"
```

On success, `status="1"` and `result` is the resolved address. Results may remain cached for five minutes. This endpoint
cannot prove resolution at a historical block or an immediately changed record.

Offchain CCIP Read (EIP-3668) and wildcard resolution (ENSIP-10), such as `jesse.base.eth`, are unsupported. When those
semantics are required, use a verified resolver-capable read route. Otherwise, report the resolution gap. The name does
not select the chain for subsequent balance/history queries.

Source: [ENS endpoint](https://docs.etherscan.io/api-reference/endpoint/forwardresolve).

## ETH Balance Query

Query native ETH (or native token) balance for an address.

### Endpoint Parameters

| Parameter | Required | Default  | Description                                                                                                                                           |
| --------- | -------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `chainid` | No       | `1`      | Chain ID (see etherscan-chains.md)                                                                                                                    |
| `module`  | Yes      | -        | Set to `account`                                                                                                                                      |
| `action`  | Yes      | -        | Set to `balance`                                                                                                                                      |
| `address` | Yes      | -        | Wallet address (supports up to 20 comma-separated)                                                                                                    |
| `tag`     | No       | `latest` | `latest` or hex block number. On free/Lite, queries cover only the last 128 blocks. Older history needs the `balancehistory` PRO endpoint (Standard+) |
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

**Use only when `pro_endpoints=true`** from plan detection. Both require Standard plan or higher. The API throttles both
to **2 calls/second** regardless of tier.

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

`txlistinternal` **by address** remains a community endpoint. The variant using only `startblock`/`endblock` without an
address is [PRO-only](https://docs.etherscan.io/api-reference/endpoint/txlistinternal-blockrange) since 2026-07-01. For
that variant, require `pro_endpoints=true` (Standard+). Adding block bounds to an address-filtered query does not make
it PRO.

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

Since 2026-07-01, Free requests return at most 1,000 records for address transaction/transfer history, beacon
withdrawals, validated blocks, node-size history, event logs, and Plasma deposits. For Free or unknown plans, set
`offset <= 1000` where supported. Paid plans retain endpoint-specific limits. Do not assume every list action allows
10,000. Source:
[record-limit change](https://docs.etherscan.io/changelog#upcoming-change-reduced-maximum-records-per-request-on-the-free-api-tier).

While advancing `page`, keep block bounds and sorting fixed. A full page requires another request. Never declare history
complete because a capped response contains fewer records than an oversized requested `offset`. If the endpoint times
out, narrow the block range without dropping boundary records.

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

Do **not** use `datetime.utcfromtimestamp()`. It returns a naive datetime. Python 3.12+ deprecates it.

```bash
# Shell equivalent (GNU date)
date -u -d "@1693526400" --iso-8601=seconds
# macOS / BSD date
date -u -r 1693526400 +"%Y-%m-%dT%H:%M:%SZ"
```

## NFT Transfer History

Fetch historical ERC-721 or ERC-1155 transfers for an address. Both actions share the parameter table in the previous
section. To filter by collection, pass `contractaddress`. Apply the pagination guidance above, including the 1,000
Free-tier cap and fixed `startblock`/`endblock`/`page`/`offset`/`sort` semantics.

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

Use the same parameter shape. Change to `action=token1155tx`. ERC-1155 differs from ERC-721 in two response fields:

- **`tokenValue`** (string) — quantity transferred for this `tokenID`. ERC-1155 requires it because it is semi-fungible.
  A single transfer can move N copies of one ID. **Not present in ERC-721 responses.**
- The API omits **`tokenDecimal`** (ERC-1155 has no decimals concept).

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

- **By collection** — pass `contractaddress=<collection>`. The API filters server-side. To fetch transfers across all
  collections, omit the parameter.
- **By token ID** — no server-side filter exists. Fetch the collection's transfers. Filter `result[].tokenID == <id>`
  client-side. For high-volume collections, narrow with `startblock`/`endblock` first.
- **Mint vs burn vs transfer** — derive from `from`/`to`:
  - `from == 0x0000...0000` → mint
  - `to == 0x0000...0000` → burn
  - otherwise → transfer

### Cost & Limits

Standard list-endpoint pricing — 1 credit per call, same rate-limit tier as `txlist`. This is not a PRO endpoint. It is
available on Free and Lite for Etherscan-supported target chains, subject to dated chain-access rules and shared Free
quotas.

## First Funding Transaction

Identify the earliest transaction that sent native value to an address — useful for fund-origin tracing, provenance, or
compliance checks. Cost is **1 API call** (PRO) or **2 API calls** (fallback).

### Preferred: `fundedby` (PRO endpoint)

The endpoint returns the address, tx hash, block, timestamp, and value of the transaction that first funded an EOA. One
call returns a structured response.

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
- **EOA only.** Contract addresses return an error. For those addresses, use the fallback below.

### Fallback: scan ASC normal + internal transactions

When `pro_endpoints=false` (free/Lite) or the address is a contract, scan both transaction lists in ascending order.
Under that condition, pick the earliest qualifying incoming entry. Two API calls per address.

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

The funding tx is whichever match has the lower `blockNumber`. Break ties by `transactionIndex` (normal txs) or by list
order (internal txs).

**Why both lists:** An external transaction (normal tx) or a contract (internal tx) may fund an address. Contract-sent
ETH is common for CEX withdrawals through proxy/router contracts, contract deployments with non-zero `msg.value`, or
SELFDESTRUCT refunds. Checking only `txlist` will miss internally-funded addresses.

**Why `offset=10`, not `1`:** A `txlist` query returns every tx involving the address, including outgoing ones. The very
first entry is occasionally outgoing (e.g., the address was internally pre-funded). Fetch a small window. Scan it for
the first incoming match.

**Edge cases:**

- **No qualifying entry in the first 10** — extend with `offset=100` and `page=1`, or paginate further. In practice, >
  10 outgoing-before-incoming is exceedingly rare.
- **Genesis allocation** — pre-mined balances do not appear in either list. The address shows a balance with no funding
  tx. Report this explicitly.
- **Token-only funding** — `fundedby` and this fallback only consider native value. If ERC-20 transfers alone
  bootstrapped the address, repeat the fallback against `tokentx`. This is rare for EOAs because they need gas.

## Multi-Chain Usage

Specify the `chainid` parameter to query different blockchains.

### Target Chain IDs (Free Tier)

See `references/generated/etherscan-chains.md` for the free-tier target chain list with chain IDs.

### Example: Polygon Query

```bash
curl -s "https://api.etherscan.io/v2/api?chainid=137&module=account&action=balance&address=0x...&tag=latest&apikey=$ETHERSCAN_API_KEY"
```

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

Use the completion format in `SKILL.md`. Preserve full identifiers. Use a compact table only when fields repeat.

## Plan-Gated Capabilities

Decisions in this section depend on the cached output of `scripts/etherscan-detect-plan.sh`.

### Paid-Only Chains

Use `references/generated/etherscan-chains.md` and its dated notes for the paid-plan target set. Lite grants community
endpoint access on supported chains at the same 100,000 daily-credit limit as Free. PRO endpoints still require
Standard+. Gnosis requires Lite+ since 2026-09-01. Robinhood Chain is free through 2026-10-15 and requires Lite+ from
2026-10-16.

**Exception:** Source code (`module=contract&action=getsourcecode`) and ABI (`action=getabi`) remain available on
supported chains for every plan, including Free. This exception does not restore deprecated chain support.

If `paid_chains=false` (i.e., `plan=free`) and the user requests a data query on the chains above, route to Blockscout
(`references/explorers/blockscout-api.md`) before direct RPC. Only if the user specifically needs Etherscan as the
source, mention upgrading to Lite or higher.

### PRO-Only Endpoints

When `pro_endpoints=true`, the following actions become available (non-exhaustive — see
`https://docs.etherscan.io/api-pro/api-pro` for the full list):

| Module       | Action(s)                                                                     | Use case                                                 |
| ------------ | ----------------------------------------------------------------------------- | -------------------------------------------------------- |
| `account`    | `addresstokenbalance`, `addresstokennftbalance`, `balancehistory`, `fundedby` | Full holdings, historical balances, first-funding lookup |
| `account`    | `txlistinternal` without `address` (block-range variant)                      | Internal transactions across a block range               |
| `token`      | `tokenholderlist`, `tokeninfo`, `tokensupplyhistory`, `tokenbalancehistory`   | Token analytics                                          |
| `block`      | `dailyavgblocksize`, `dailyblkcount`, `dailyblockrewards`, etc.               | Daily block stats                                        |
| `stats`      | `dailytxnfee`, `dailynewaddress`, `dailynetutilization`, etc.                 | Network-wide daily metrics                               |
| `gastracker` | `dailyavggaslimit`, `dailygasused`, `dailyavggasprice`                        | Daily gas metrics                                        |

When `pro_endpoints=false` (free or Lite), prefer the non-PRO equivalents listed in this skill or fall back to per-token
loops.

### Token Reputation / Metadata

Etherscan's token reputation badges are **not available on Lite**. The documented API surfaces are:

| Surface              | Endpoint/action                           | Minimum plan | Notes                                                                                                |
| -------------------- | ----------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------- |
| Address Metadata API | `module=nametag&action=getaddresstag`     | Pro Plus     | Query the token contract address. The response includes numeric `reputation` and `other_attributes`. |
| Metadata CSV export  | `module=nametag&action=exportaddresstags` | Enterprise   | Bulk export. `other_attributes` can include `TR` token reputation values.                            |
| Token info           | `module=token&action=tokeninfo`           | Standard     | Returns project/social metadata and `blueCheckmark`, but not the token reputation badge.             |

Do not tell Lite users they can fetch token reputation from Etherscan API. Lite only unlocks paid Etherscan target
chains and higher community rate limits. It does not unlock API Pro endpoints, Pro Plus address metadata, or Enterprise
metadata CSV exports.

### All Plans

Free-tier availability is subject to the generated chain table's dated notes and shared community quota. These quotas
are per chain across all Free users, independent of the key's remaining daily credits. Celo and Linea already use shared
pools. Arbitrum One starts on 2026-11-01. A successful plan-detection call does not guarantee quota remains for the
target chain.

## Error Handling

### Common Error Responses

| Status | Message                  | Cause                           |
| ------ | ------------------------ | ------------------------------- |
| `0`    | `NOTOK`                  | Invalid API key or rate limited |
| `0`    | `Invalid address format` | Malformed address               |
| `0`    | `No transactions found`  | Address has no activity         |

Inspect `result` as well as `status` and `message`. A documented empty result covers only the queried endpoint and
range. `NOTOK`, quota errors, plan denial, and unsupported-chain responses are coverage gaps, never empty activity.

- `Community Free API limit reached`: preserve the response's UTC reset time. Use the indexed fallback or wait until
  that time. All Free keys share the chain's pool. Do not retry briefly. Do not rotate API keys.
- `Free API access is not supported for this chain`: use a supported fallback when paid access is unavailable.
- Missing/unsupported `chainid`: check the current chainlist and dated deprecations. Then route the target to a
  supported provider. Do not retry retired explorer API hosts.

Source: [common errors](https://docs.etherscan.io/common-error-messages).

### Rate Limits by Plan

| Plan             | Calls/second | Daily calls       |
| ---------------- | ------------ | ----------------- |
| Free             | 3            | 100,000           |
| Lite             | 5            | 100,000           |
| Standard         | 10           | 200,000           |
| Advanced         | 20           | 500,000           |
| Professional     | 30           | 1,000,000         |
| Pro Plus         | 30           | 1,500,000         |
| Dedicated/Custom | custom       | contract-specific |

The API throttles PRO endpoints (`addresstokenbalance`, etc.) to **2 calls/second** regardless of tier. See
`https://docs.etherscan.io/rate-limits` for the authoritative schedule. Endpoint-specific throttles, including ENS
Free-tier 1 call/second, override the plan-wide rate.

For a per-key rate throttle, back off within the plan's rate. Under that condition, retry with a bounded budget. For
daily-credit or shared community quota exhaustion, honor the reset time instead.

## Reference Files

- **`references/generated/etherscan-chains.md`** - Target-filtered list of supported chains with chain IDs
- **`scripts/etherscan-detect-plan.sh`** - Plan-tier detection helper (run once per session)

## Fallback Documentation

For read-only use cases not covered by this skill (gas estimates, block or network stats, etc.), fetch the AI-friendly
documentation:

```
https://docs.etherscan.io/llms.txt
```
