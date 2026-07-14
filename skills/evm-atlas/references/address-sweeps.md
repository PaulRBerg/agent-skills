# Address Sweeps

Use this reference when the prompt gives an EVM address and asks across "any chain", "all chains", "where has this
address been active?", "has this address ever been used?", or "does this address hold anything now?".

For a current native or fungible-token portfolio, enter through `blockscan-balances.md`. Use the current-balance
workflow below when Blockscan needs an API fallback or when the request requires NFT enumeration.

Scope is exactly `./references/target-mainnets.json`. Do not expand to every Etherscan, Blockscout, Chainscout,
Chainlist, or RPC-supported EVM chain. If the user wants non-target coverage, ask for a feature request in
<https://github.com/PaulRBerg/agent-skills>.

## Provider Routing

For each target chain:

1. Prefer Etherscan V2 when the chain is listed in `./references/etherscan-chains.md` and the current Etherscan plan can
   query it.
2. Use Blockscout for target chains not served by Etherscan, and for paid Etherscan chains when
   `./scripts/etherscan-detect-plan.sh` reports `paid_chains=false`.
3. If neither indexed provider can cover the target chain, use the target chain's public RPC only for native balance
   checks. Historical activity and token/NFT holdings require an indexer; report those fields as unavailable for that
   chain instead of silently treating them as empty.

For Etherscan, run plan detection once per session before querying Etherscan, then cache it:

```bash
./scripts/etherscan-detect-plan.sh
```

For Blockscout PRO, run plan detection once per session before using the PRO host, then cache it:

```bash
./scripts/blockscout-detect-plan.sh
```

If `$ETHERSCAN_API_KEY` is missing, route Etherscan-supported chains to Blockscout where Blockscout covers them;
otherwise report the Etherscan gap. If `$BLOCKSCOUT_API_KEY` is missing, skip Blockscout PRO detection and use
per-instance Blockscout hosts from `./scripts/resolve-chain.sh` where available.

## Historical Activity Sweep

Goal: answer whether an address has ever had indexed activity on any target mainnet. Count a chain as active when any of
these exists:

- normal transaction involving the address
- internal transaction involving the address
- ERC-20 transfer where the address is sender or recipient
- ERC-721 transfer where the address is sender or recipient
- ERC-1155 transfer where the address is sender or recipient

Use `sort=desc&page=1&offset=1` wherever the provider supports it. This proves existence without full pagination. Stop
at the first positive result when the user only asks "has this address ever been active?". Continue across all target
chains when the user asks which chains or wants a report.

### Etherscan

Use the advanced filter parameters when accepted by the endpoint. Set `from=<addr>&to=<addr>&fromto_opr=or` for
direction-agnostic checks:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=txlist&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=latest&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=txlistinternal&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=latest&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=tokentx&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=latest&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=tokennfttx&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=latest&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=token1155tx&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=latest&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
```

If an action rejects the advanced filter shape, fall back to the standard address-list shape:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=<action>&address=<addr>&startblock=0&endblock=999999999&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
```

Treat `status=1` with a non-empty `result` array as positive. Treat Etherscan's "No transactions found" / empty result
as negative for that action only; still check the remaining activity actions before marking the chain inactive.

### Blockscout

For existence probes, prefer the Etherscan-compatible layer because it supports `sort`, `page`, and `offset`. On the PRO
host, pass `$BLOCKSCOUT_API_KEY` with `authorization: Bearer $BLOCKSCOUT_API_KEY` rather than a query parameter:

```text
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=txlist&address=<addr>&sort=desc&page=1&offset=1
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=txlistinternal&address=<addr>&sort=desc&page=1&offset=1
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=tokentx&address=<addr>&sort=desc&page=1&offset=1
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=tokennfttx&address=<addr>&sort=desc&page=1&offset=1
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=token1155tx&address=<addr>&sort=desc&page=1&offset=1
```

If the PRO host returns `404` or no key is available, resolve the per-instance URL and repeat the same five compat
actions against that host:

```bash
CS_instance_url=$(./scripts/resolve-chain.sh <id> | sed -n 's/^instance_url=//p')
curl -s "${CS_instance_url}api?module=account&action=txlist&address=<addr>&sort=desc&page=1&offset=1"
```

If the compat action is unavailable on that instance, use native v2 newest-first endpoints and treat a non-empty `items`
array as positive:

```text
api/v2/addresses/<addr>/transactions
api/v2/addresses/<addr>/internal-transactions
api/v2/addresses/<addr>/token-transfers?type=ERC-20
api/v2/addresses/<addr>/token-transfers?type=ERC-721
api/v2/addresses/<addr>/token-transfers?type=ERC-1155
```

## Current Balance Sweep: API Fallback and NFT Coverage

Goal: answer whether an address currently holds any native balance, ERC-20 balance, ERC-721 NFT, or ERC-1155 item on
target mainnets. Report only positive balances by default, plus explicit provider gaps.

### Etherscan

Always check native balance when the chain is queryable:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=balance&address=<addr>&tag=latest&apikey=$ETHERSCAN_API_KEY
```

When `pro_endpoints=true`, enumerate token and NFT holdings:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=addresstokenbalance&address=<addr>&page=1&offset=100&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=addresstokennftbalance&address=<addr>&page=1&offset=100&apikey=$ETHERSCAN_API_KEY
```

Paginate PRO holdings while returned pages are full. If `pro_endpoints=false`, Etherscan can only check native balance
and known single-token balances (`tokenbalance` with a provided contract). Use Blockscout for full holdings when
Blockscout covers the chain; otherwise report token/NFT holdings as not enumerable through available indexed providers.

### Blockscout

Use native v2 with bearer auth on the PRO host. `addresses/{hash}` gives native balance;
`addresses/{hash}/token-balances` gives full ERC-20/721/1155 holdings where indexed:

```text
https://api.blockscout.com/<id>/api/v2/addresses/<addr>
https://api.blockscout.com/<id>/api/v2/addresses/<addr>/token-balances
```

On `404` or missing key, resolve the per-instance URL and use:

```bash
CS_instance_url=$(./scripts/resolve-chain.sh <id> | sed -n 's/^instance_url=//p')
curl -s "${CS_instance_url}api/v2/addresses/<addr>"
curl -s "${CS_instance_url}api/v2/addresses/<addr>/token-balances"
```

Count `coin_balance > 0` as native holdings. Count token entries with `value > 0` as holdings; include `token.type`,
`token.symbol`, `token.address_hash`, `token_id` when present, and raw `value` plus decimals where available.

### RPC Fallback

For chains with no usable indexed provider, query only the native balance:

```text
eth_getBalance(<addr>, latest)
```

Do not infer token/NFT emptiness from RPC-only coverage. Token and NFT holdings require a token contract list or indexed
provider.

## Output

For a yes/no answer, lead with the boolean result and the first positive chain/action found. For reports, group by
chain:

```markdown
| Chain | Chain ID | Activity | Native Balance | Token/NFT Holdings | Source |
| ----- | -------- | -------- | -------------- | ------------------ | ------ |
```

Include explicit gaps such as "Etherscan paid-chain unavailable on free plan; Blockscout absent" or "RPC native balance
only". Do not say "inactive on all EVM chains" unless the checked scope is clear; say "no indexed activity found across
the target mainnets".

## Provider Docs

- Etherscan introduction: <https://docs.etherscan.io/introduction>
- Etherscan advanced normal transaction filter:
  <https://docs.etherscan.io/api-reference/endpoint/advanced-filter-txlist>
- Etherscan advanced ERC-20 transfer filter: <https://docs.etherscan.io/api-reference/endpoint/advanced-filter-tokentx>
- Blockscout PRO API: <https://docs.blockscout.com/devs/pro-api>
