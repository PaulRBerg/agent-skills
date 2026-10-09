# Etherscan API V2

## Overview

Use the official `etherscan` CLI for routine Etherscan reads. These commands were verified with CLI **1.1.1** on
2026-10-09. Read the installed command's `--help` when its version differs.

This reference covers native/token balances, transaction and transfer history, funding, holdings, transaction/receipt
facts, blocks, logs, and contract ABI/source. Preserve the read-only boundary. Never call
`proxy eth_sendRawTransaction`, contract verification submissions, or other state-changing commands.

Keep Blockscout first on covered overlaps. CLI availability does not change provider priority, the target allowlist,
checkpoint proof, quorum, or plan gates. User instructions take precedence over this skill.

The dated access rules reflect the [changelog](https://docs.etherscan.io/changelog) check on 2026-10-05. Apply scheduled
changes by their effective date. Provider membership alone does not prove plan access.

Sources: [official CLI guide](https://docs.etherscan.io/build-with-ai/cli) and
[CLI 1.1.1 source](https://github.com/etherscan/etherscan-cli/tree/v1.1.1).

## Prerequisites

### API Key Validation

Check `etherscan version`. The CLI accepts `ETHERSCAN_API_KEY` or an existing saved login. A missing environment
variable does not prove that CLI authentication is unavailable.

Never pass a key through `--api-key`, inspect saved credentials, or run `whoami`, `config`, or `login` to discover a
key. Those commands can disclose or alter credentials. Let a read command use existing authentication. On an
authentication failure, report the route unavailable and use the supported provider fallback.

For a documented direct API exception, require `ETHERSCAN_API_KEY` without printing its value. Do not read the CLI's
saved configuration to obtain it. See [API Base URL](#api-base-url) for the credential-safe request pattern.

### Plan Detection

Run the helper **once per session**. Cache its `key=value` result. It uses `etherscan apilimit` and a Base balance probe
to distinguish Free from Lite:

```sh
scripts/etherscan-detect-plan.sh
```

```text
plan=lite
credit_limit=100000
credits_used=4
credits_available=99996
limit_interval=daily
interval_expiry=14:38:10
pro_endpoints=false
paid_chains=true
```

`plan` is `free`, `lite`, `standard`, `advanced`, `professional`, `pro_plus`, `enterprise`, or `unknown`. `paid_chains`
and `pro_endpoints` are `true`, `false`, or `unknown`. Only `true` authorizes the corresponding gate.

- `paid_chains=true`: paid-chain community endpoints are available. Apply the generated chain table's dated notes.
- `pro_endpoints=true`: PRO actions are available. These start at Standard and remain unavailable on Lite.

If the helper is unavailable, inspect only the returned credit fields:

```sh
etherscan --chain 1 --output json apilimit
```

The CLI returns the result object directly, including `creditLimit`, `creditsUsed`, `creditsAvailable`, `limitInterval`,
and `intervalExpiryTimespan`. Do not look for an outer `status`, `message`, or `result`.

| `creditLimit` | Plan         | Paid-only chains | PRO endpoints |
| ------------- | ------------ | ---------------- | ------------- |
| 100,000       | Free or Lite | Probe to confirm | No            |
| 200,000       | Standard     | Yes              | Yes           |
| 500,000       | Advanced     | Yes              | Yes           |
| 1,000,000     | Professional | Yes              | Yes           |
| 1,500,000     | Pro Plus     | Yes              | Yes           |
| > 1,500,000   | Enterprise   | Yes              | Yes           |

For the 100,000-credit tier, use this paid-chain probe:

```sh
etherscan --chain 8453 --output json account balance 0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe --tag latest
```

A valid balance result indicates Lite. Only `Free API access is not supported for this chain` indicates Free. Transport,
authentication, throttle, quota, or other errors leave `plan` and `paid_chains` unknown. For the 100,000-credit tier,
`pro_endpoints=false` still follows from the limit.

Lite permits community endpoints on supported chains at 5 calls/second, versus Free's 3. It does not add PRO endpoints.
A PRO denial includes `Sorry, it looks like you are trying to access an API Pro endpoint.` Preserve that diagnostic.

`apilimit` consumes one credit. The Base probe consumes another. Do not repeat detection mid-session.

## Chain Inference

Do not default to Ethereum. Resolve the target with `scripts/chain-lookup.sh <name|alias|slug|chain_id>` before
querying. Pass the resolved numeric ID explicitly with `--chain` on every CLI command. Never rely on saved chain
settings.

### Inference Rules

1. Use explicit chain context, such as Polygon, Arbitrum One, or Base.
2. Use unambiguous chain-specific token context: POL→137, ARB→42161, OP→10, AVAX→43114, BNB→56, SONIC→146, SEI→1329,
   MON→143.
3. An address can exist on multiple chains. If the prompt does not resolve its chain, ask.
4. Testnets are outside the target list. Ask for a feature request instead of querying them.
5. If the chain remains ambiguous, ask before making the data query.

### Unsupported Chains

Query only `references/generated/target-mainnets.json`. If the chain is absent, stop and request a feature in
<https://github.com/PaulRBerg/agent-skills>. Do not query another provider, web search, Chainlist, or an unlisted RPC to
bypass this boundary. Non-EVM chains are outside this skill.

For a target absent from Etherscan, use Blockscout before the public-RPC route in
`references/workflows/provider-routing.md`. RPC proves only its supported facts. Missing indexed history, full holdings,
or funding coverage remains unknown.

Use `references/generated/etherscan-chains.md` and its dated notes for provider coverage and access gates. CLI 1.1.1's
`chains` metadata includes static `free_tier` and symbol values. Those values do not override the registry or live
access rules. Provider additions do not expand the target list. Provider deprecations do not remove targets.

## API Base URL

The CLI uses `https://api.etherscan.io/v2/api`. Its numeric `--chain` maps to the API's `chainid`.

Use direct API requests only for a documented exception:

- CLI 1.1.1 has no ENS `forwardresolve` command.
- The installed CLI lacks a required endpoint or flag. Verify the gap with `--help` and official documentation.
- A validator or evidence task requires the original API envelope, HTTP status, or fields the CLI omits.
- The CLI is unavailable and an authenticated direct request can complete the authorized read.

State the exception reason in the evidence record. Keep the target, plan, rate, checkpoint, and pagination rules. Never
use direct API access to bypass a denial or convert a CLI error into empty evidence.

The ENS example below reads the key inside Python. It keeps credentials out of argv and error diagnostics. For another
exception, change only the documented non-secret parameters. Do not enable request tracing or print the URL.

```sh
python3 - <<'PY'
import os
import sys
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode
from urllib.request import urlopen

key = os.environ.get("ETHERSCAN_API_KEY")
if not key:
    raise SystemExit("ETHERSCAN_API_KEY is required for this direct API exception.")
params = {
    "chainid": "1", "module": "ens", "action": "forwardresolve",
    "name": "etherscan.eth", "apikey": key,
}
try:
    with urlopen("https://api.etherscan.io/v2/api?" + urlencode(params), timeout=30) as response:
        http_code, body = response.status, response.read().decode()
except HTTPError as error:
    http_code, body = error.code, error.read().decode()
except (URLError, TimeoutError):
    raise SystemExit("Etherscan transport failed; coverage is unknown.")
for secret in (key, quote(key, safe="")):
    body = body.replace(secret, "<redacted>")
print(f"http_status={http_code}", file=sys.stderr)
print(body)
raise SystemExit(0 if 200 <= http_code < 300 else 1)
PY
```

Direct API responses retain `status`, `message`, and `result`, or the JSON-RPC envelope. Validate the HTTP status and
full response before accepting evidence. API errors can arrive with HTTP 200.

## ENS Forward Resolution

Use the direct API exception above for an onchain ENS name or subdomain. `forwardresolve` is Ethereum-mainnet-only and
available on Free and paid plans. Throttle Free requests to one call/second.

Success requires `status="1"` and a valid address in `result`. Results may stay cached for five minutes. This endpoint
cannot prove historical resolution or an immediately changed record.

Offchain CCIP Read (EIP-3668) and wildcard resolution (ENSIP-10), such as `jesse.base.eth`, are unsupported. When
needed, use a verified resolver-capable read route. Otherwise, report the gap. The name does not select the chain for
later balance or history queries.

Source: [ENS endpoint](https://docs.etherscan.io/api-reference/endpoint/forwardresolve).

## ETH Balance Query

Query the native currency in its smallest unit. The symbol comes from the resolved target row.

### Endpoint Parameters

Use `account balance ADDRESS --tag TAG` for one address and `account balancemulti ADDRESSES --tag TAG` for up to 20
comma-separated addresses. For old native state, `account balancehistory ADDRESS --blockno BLOCK_NUMBER` requires
`pro_endpoints=true`.

A `balance` hex tag covers only the last 128 blocks on Free/Lite. Older native history needs Standard+ or the permitted
state fallback. Check `provider-routing.md` for exact checkpoints and exceptional chains.

### Single Address Query

```sh
etherscan --chain 1 --output json account balance 0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe --tag latest
```

For checkpointed history, replace `latest` with its verified hex tag. For PRO historical balance:

```sh
etherscan --chain CHAIN_ID --output json account balancehistory ADDRESS --blockno CHECKPOINT_NUMBER
```

### Multi-Address Query (up to 20)

```sh
etherscan --chain CHAIN_ID --output json account balancemulti 'ADDRESS_1,ADDRESS_2' --tag CHECKPOINT_HEX
```

### Response Format

Single-address stdout is a JSON quantity string, such as `"172774397764084972158218"`. Multi-address stdout is an array
of `{ "account": ADDRESS, "balance": QUANTITY }` objects. Preserve the integers before unit conversion.

## ERC-20 Token Balance Query

### Endpoint Parameters

`account tokenbalance ADDRESS --contractaddress CONTRACT --tag TAG` queries one ERC-20 contract. Use verified token
decimals. For exact historical state, preserve the provider's checkpoint and archive limits.

### Example Query

```sh
etherscan --chain 1 --output json account tokenbalance 0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe \
  --contractaddress 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 --tag latest
```

### Response Format

Stdout is the token's smallest-unit quantity as a JSON string. It is not an API envelope or a decimal token amount.

### Full Holdings

| CLI command                        | Returns                                  |
| ---------------------------------- | ---------------------------------------- |
| `account addresstokenbalance`      | ERC-20 holdings, quantities and metadata |
| `account addresstokennftbalance`   | ERC-721 collection holdings and counts   |
| `account addresstokennftinventory` | Inventory for one NFT collection         |

Require `pro_endpoints=true` for these PRO holdings endpoints. Set both `--page` and `--offset` and exhaust pagination.
Throttle PRO reads to two calls/second. Example:

```sh
etherscan --chain CHAIN_ID --output json account addresstokenbalance ADDRESS --page 1 --offset 100
etherscan --chain CHAIN_ID --output json account addresstokennftinventory ADDRESS \
  --contractaddress COLLECTION --page 1 --offset 100
```

Holdings endpoints have no checkpoint selector. Record their observation time separately. Do not use them as historical
cutoff proof. On Free/Lite, use a known contract list with `tokenbalance`, or the ordered holdings fallback. A known
list cannot prove complete holdings.

## Transaction History Queries

| CLI command              | Channel                      |
| ------------------------ | ---------------------------- |
| `account txlist`         | Normal external transactions |
| `account txlistinternal` | Internal transactions        |
| `account tokentx`        | ERC-20 transfers             |
| `account tokennfttx`     | ERC-721 transfers            |
| `account token1155tx`    | ERC-1155 transfers           |

Address-filtered `txlistinternal` is a community endpoint, even with block bounds. The block-range-only variant requires
`pro_endpoints=true` since 2026-07-01. Do not omit `--address` during an address sweep. For one transaction's internal
rows, use `account txlistinternal --txhash TX_HASH`.

### Endpoint Parameters

Pass the resolved `--chain`, address, fixed `--startblock` and `--endblock`, `--sort`, and both `--page` and `--offset`.
`txlistinternal` takes `--address ADDRESS`. The other four address-history commands also accept a positional address.
Token-transfer commands accept `--contractaddress CONTRACT`. For supported direction filters, inspect `--help` and set
`--fromto-opr` explicitly with `--from` or `--to`.

For Free or unknown plans, use `--offset` at most 1,000. Since 2026-07-01, that cap also applies to Free event logs and
other listed endpoints. Paid plans retain endpoint-specific caps. Do not assume every endpoint accepts 10,000. Source:
[record-limit change](https://docs.etherscan.io/changelog#upcoming-change-reduced-maximum-records-per-request-on-the-free-api-tier).

Prefer **manual paging** for evidence. Keep the cutoff bounds, sort, filters, and page size fixed while advancing
`--page`. A full page requires another request. Stop only after a validated short or empty page under the applicable
cap. On timeout, partition inclusive block ranges without losing boundary records.

CLI 1.1.1 `--all` starts at page 1 and ignores the supplied start page. Its hidden `--max-pages` defaults to 20.
Reaching that bound returns partial data, a stderr warning, and exit 0. If used, set `--max-pages` explicitly and retain
stderr. Neither exit 0 nor the absence of a warning proves exhaustion. An all-empty `--all` result can have no stdout.

### Example Query

These are first-page requests, not complete-history claims:

```sh
etherscan --chain CHAIN_ID --output json account txlist ADDRESS \
  --startblock 0 --endblock CHECKPOINT_NUMBER --page 1 --offset 100 --sort asc
etherscan --chain CHAIN_ID --output json account txlistinternal --address ADDRESS \
  --startblock 0 --endblock CHECKPOINT_NUMBER --page 1 --offset 100 --sort asc
```

### Response Format

Stdout is an array of native provider rows. Preserve `hash`, `blockNumber`, `timeStamp`, `from`, `to`, `value`, success
fields, and channel-specific identifiers. Do not query `.result[]` on CLI output. A successful manual empty history page
is `[]`. Validate row shape and range before accepting it.

### Timestamp Conversion

`timeStamp` is Unix seconds, usually a string. Produce timezone-aware UTC values:

```python
from datetime import datetime, timezone

dt = datetime.fromtimestamp(int(tx["timeStamp"]), tz=timezone.utc)
```

Do not use naive `datetime.utcfromtimestamp()`. For macOS/BSD date:

```sh
date -u -r 1693526400 +"%Y-%m-%dT%H:%M:%SZ"
```

## NFT Transfer History

Apply the same checkpoint bounds, manual paging, and plan caps. Omit `--contractaddress` only when the request covers
all collections.

### ERC-721 Transfers (`tokennfttx`)

```sh
etherscan --chain CHAIN_ID --output json account tokennfttx ADDRESS --contractaddress COLLECTION \
  --startblock 0 --endblock CHECKPOINT_NUMBER --page 1 --offset 100 --sort asc
```

Preserve `contractAddress`, `tokenID`, `tokenName`, and `tokenSymbol`. ERC-721 `tokenDecimal` is `"0"`.

### ERC-1155 Transfers (`token1155tx`)

Use `account token1155tx` with the same flags. Preserve `tokenValue`, the quantity of the transferred `tokenID`.
ERC-1155 has no decimals field. A `TransferBatch` can produce several rows with the same transaction hash. Retain each
`(tokenID, tokenValue)` pair and its event identity when reconstructing a batch.

### Filtering by Collection or Token ID

Use `--contractaddress` for a collection. No server-side token-ID filter exists. Filter the returned rows by `tokenID`
after covering the required block range. Derive mint and burn from the zero address in `from` and `to` respectively.

### Cost & Limits

These transfer endpoints are community actions, not PRO. They cost one credit per call under the normal plan rate,
subject to dated chain access and shared Free quotas.

## First Funding Transaction

Identify the earliest successful incoming native-value transfer through the required checkpoint. Distinguish provider
funding metadata from independently verified first-funding evidence.

### Preferred: `fundedby` (PRO endpoint)

With `pro_endpoints=true`, query an EOA:

```sh
etherscan --chain CHAIN_ID --output json account fundedby ADDRESS
```

The result object includes `fundingAddress`, `fundingTxn`, `block`, `timeStamp`, and `value`. The endpoint costs one
call and is throttled to two calls/second. Contracts are unsupported. Verify the full transaction/receipt and the
requested cutoff before using the result as historical evidence.

### Fallback: scan ASC normal + internal transactions

On Free/Lite, unsupported EOAs, or contract targets, manually paginate both `txlist` and address-filtered
`txlistinternal` from genesis through the fixed cutoff with `--sort asc`. Use the bounded commands above. Two channels
do not imply two calls.

For each channel, stop at its first qualifying row only after covering every preceding row and proving same-block
transaction/trace order. If that order is unproven, cover the entire candidate block and select its earliest qualifying
row. If no candidate appears, exhaust that channel through the cutoff. Both complete ranges are required only to prove
no funding.

Qualifying rows must satisfy all these conditions:

- The recipient is the target address, compared case-insensitively. For a creation row, inspect `contractAddress` when
  `to` is empty.
- `value` is a positive integer in the native smallest unit.
- `isError == "0"` for both normal and internal rows. For missing or ambiguous fields, prove normal success from the
  receipt and internal success from trace evidence. Parent receipt success alone does not prove an internal call
  succeeded.

Compare qualifying rows by block number and transaction index. For internal rows without that index, obtain it from the
parent receipt. Preserve `traceId` for ordering within one transaction. Do not order same-block transactions by hash or
timestamp. Report an unresolved tie instead of inventing an order.

Check both channels because contract calls, creation, and SELFDESTRUCT can deliver native value. Genesis allocations
appear in neither list. A balance without funding rows alone does not prove a genesis allocation. Report that gap. For
token-only origin, query `tokentx` separately and label it token funding, not native funding.

Sources: [funding endpoint](https://docs.etherscan.io/api-reference/endpoint/fundedby) and
[internal row fields](https://docs.etherscan.io/api-reference/endpoint/txlistinternal).

## Transaction, Receipt, Block, and Log Reads

Use the resolved numeric chain ID and preserve full hashes:

```sh
etherscan --chain CHAIN_ID --output json proxy eth_getTransactionByHash TX_HASH
etherscan --chain CHAIN_ID --output json proxy eth_getTransactionReceipt TX_HASH
etherscan --chain CHAIN_ID --output json proxy eth_getTransactionCount ADDRESS --tag CHECKPOINT_HEX
etherscan --chain CHAIN_ID --output json proxy eth_getBlockByNumber --tag CHECKPOINT_HEX --boolean false
etherscan --chain CHAIN_ID --output json transaction receipt-status TX_HASH
```

Proxy stdout is the unwrapped JSON-RPC result. A missing transaction or receipt can produce no stdout with exit 0. Treat
that as inconclusive coverage. When other evidence proves the transaction, it never establishes nonexistence. Match
transaction hash, block number, and block hash across the transaction, receipt, and header. A receipt-status summary
does not replace the full receipt or logs.

For timestamp lookup, `block bytime UNIX_SECONDS --closest before` returns a candidate block number. Prove it and its
successor under `provider-routing.md` before adopting the checkpoint.

Query logs with explicit inclusive block bounds and manual pages:

```sh
etherscan --chain CHAIN_ID --output json logs get --address CONTRACT \
  --from-block START_BLOCK --to-block CHECKPOINT_NUMBER --topic0 EVENT_TOPIC \
  --page 1 --offset 100
```

`logs get` supports `--topic0` through `--topic3` and operators such as `--topic0-1-opr and`. Inspect `--help` for each
operator. For wallet transfers, query indexed sender and recipient positions separately with the correct event topics.
Do not assume one topic position proves both directions. Keep full event identities and deduplicate only exact events.
For raw-envelope conformance, use the documented direct API exception. Never pass an unwrapped CLI array to a validator
that expects an API envelope.

## Contract ABI and Source

```sh
etherscan --chain CHAIN_ID --output json contract getabi CONTRACT
etherscan --chain CHAIN_ID --output json contract getsourcecode CONTRACT
```

These read commands remain available on supported chains for every plan, including Free on paid-only chains. They do not
restore deprecated provider support. Preserve returned ABI/source fields. ABI or explorer decoding alone does not prove
execution semantics.

## Multi-Chain Usage

### Target Chain IDs (Free Tier)

Use the target-filtered generated table and dated access notes. Always pass the resolved numeric `--chain` explicitly.

### Example: Polygon Query

```sh
etherscan --chain 137 --output json account balance 0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe --tag latest
```

## Wei to Human-Readable Conversion

Preserve decimal/hex quantities as integers. Never convert balances, gas, prices, or token quantities through binary
floating point. Use the resolved native decimals and verified token decimals.

### ETH Conversion

For 18-decimal native currency:

```sh
echo "scale=18; 172774397764084972158218 / 1000000000000000000" | bc
```

### ERC-20 Conversion

USDC/USDT commonly use 6 decimals and WBTC 8. Verify the actual contract's decimals before converting:

```sh
echo "scale=6; 135499000000 / 1000000" | bc
```

## Output Formatting

Use the completion format in `SKILL.md`. Preserve full identifiers. Use a compact table only when fields repeat.

## Plan-Gated Capabilities

Use the cached detection result. A CLI command's existence does not authorize a gated endpoint.

### Paid-Only Chains

Lite grants community access on supported chains with 100,000 daily credits. Standard+ adds PRO actions. Gnosis requires
Lite+ since 2026-09-01. Robinhood Chain is free through 2026-10-15 and requires Lite+ from 2026-10-16. Apply
`references/generated/etherscan-chains.md` and its dated notes before querying.

ABI and source reads are the Free-plan exception described above. For denied data reads, use Blockscout before RPC.
Mention a paid upgrade only when the user requires Etherscan as the source.

### PRO-Only Endpoints

Require `pro_endpoints=true` for historical balances, full token/NFT holdings, `fundedby`, block-range-only internal
history, token-holder analytics, token metadata, and PRO daily statistics. Check the installed command's `--help` and
[PRO catalog](https://docs.etherscan.io/api-pro/api-pro) for the exact action. On Free/Lite, use community equivalents
or report the aggregation gap.

### Token Reputation / Metadata

| Surface                          | Minimum plan | Evidence boundary                                                   |
| -------------------------------- | ------------ | ------------------------------------------------------------------- |
| Address Metadata `getaddresstag` | Pro Plus     | Numeric `reputation` and `other_attributes`                         |
| Metadata CSV `exportaddresstags` | Enterprise   | Bulk attributes can include token-reputation `TR`                   |
| `etherscan token info CONTRACT`  | Standard     | Project/social metadata and `blueCheckmark`, not a reputation badge |

Lite does not unlock any of these paid metadata surfaces. Do not promise token reputation from Lite access.

### All Plans

Shared community quotas are per chain across all Free users, separate from the key's daily credits. Celo and Linea
already use shared pools. Arbitrum One starts on 2026-11-01. Successful detection does not guarantee remaining quota.

## Error Handling

### Common Error Responses

Capture the CLI exit code, stdout, and credential-safe stderr. Nonempty API errors and JSON-RPC errors become stderr
with nonzero exit. Do not interpret a failed command as an empty result. Validate successful output against the expected
schema. Blank stdout is not `[]` and cannot establish coverage.

For a manual history request, validated `[]` can establish only that page, channel, and fixed range. It does not prove
other channels or checkpoint completeness. If raw API evidence is needed, inspect HTTP status, `status`, `message`, and
`result`. Only documented empty shapes qualify. Arbitrary `status="0"` is not success.

- `Community Free API limit reached`: preserve the UTC reset time. Use the indexed fallback or wait until reset. Do not
  rotate keys or retry briefly.
- `Free API access is not supported for this chain`: use the supported fallback when paid access is unavailable.
- Invalid credentials, PRO denial, exhausted credits, unsupported chains, malformed output, and indexing gaps: retain
  the diagnostic and report incomplete coverage.
- Missing/unsupported chain: check the dated registry notes. Do not retry retired explorer API hosts.

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

CLI 1.1.1's hidden `--rate-limit` defaults to 3 requests/second **per process**. It does not coordinate separate CLI
invocations. Pace all session requests within the plan and endpoint limits. PRO holdings/history actions are throttled
to two calls/second regardless of tier. ENS Free requests use one call/second. Endpoint-specific throttles override
plan-wide rates. See the [rate schedule](https://docs.etherscan.io/rate-limits).

For per-key throttling, use bounded backoff within the permitted rate. For daily credits or shared-quota exhaustion,
honor the reset time instead.

## Reference Files

- `references/generated/etherscan-chains.md`: target-filtered coverage and dated access rules.
- `scripts/etherscan-detect-plan.sh`: CLI-backed, output-compatible plan detection, once per session.
- `references/workflows/provider-routing.md`: checkpoints, provider priority, fallbacks, and exceptional history.
- `references/workflows/address-sweeps.md`: deterministic history plans, channel coverage, and quorum.

## Fallback Documentation

For another authorized read, inspect `etherscan MODULE --help`, then the exact action's help. Consult the
[official CLI guide](https://docs.etherscan.io/build-with-ai/cli),
[versioned CLI source](https://github.com/etherscan/etherscan-cli/tree/v1.1.1), or
[API documentation index](https://docs.etherscan.io/llms.txt) for gaps. Preserve the read-only boundary and document any
direct API exception.
