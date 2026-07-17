# Address Sweeps

Use this reference when the prompt gives an EVM address and asks across "any chain", "all chains", "where has this
address been active?", "has this address ever been used?", or "does this address hold anything now?".

For a current native or fungible-token portfolio, enter through `blockscan-balances.md`. Use the current-balance
workflow below when Blockscan needs an API fallback or when the request requires NFT enumeration.

Scope is exactly `./references/target-mainnets.json`. Do not expand to every Etherscan, Blockscout, Chainscout,
Chainlist, or RPC-supported EVM chain. If the user wants non-target coverage, ask for a feature request in
<https://github.com/PaulRBerg/agent-skills>.

## Select the Coverage Profile

Name the profile before querying. Its required channels define what a complete negative means:

- **General activity:** normal transactions, internal transactions, ERC-20 transfers, ERC-721 transfers, and ERC-1155
  transfers. This is the default five-channel evm-atlas sweep.
- **prb-finance bootstrap:** finalized native state plus normal transactions, internal transactions, ERC-20 transfers,
  and ERC-721 transfers. ERC-1155 is deliberately excluded by prb-finance policy. Use this profile only when that repo
  or its bootstrap/discovery workflow is explicitly in scope.

Do not silently apply the bootstrap exclusion to a general sweep. A complete bootstrap result is complete for that
profile only; it is not a claim that the address has no ERC-1155 or other EVM activity.

## Fix the Checkpoint and Provider

For each target chain:

1. Fix one required ISO-8601 UTC cutoff for the whole sweep; if the prompt does not supply one, freeze the workflow
   start time once and report it. Resolve it once per chain to an exact finalized or otherwise independently verified
   block at or before that time. Record the requested cutoff, resolution kind (`finalized` or `verified`), chain ID,
   block number, block hash, block timestamp, and observation time. Reuse that exact numeric block for state and every
   history channel; never mix it with `latest` or a later provider head.
2. Batch `eth_getTransactionCount(<addr>, <cutoff>)` and `eth_getBalance(<addr>, <cutoff>)` before history. For an
   address set, include both calls for every address in bounded per-chain JSON-RPC batches. Validate every response ID
   and hex quantity. A nonzero nonce or balance is immediately positive state evidence.
3. Select one authoritative indexed-history provider for the chain using `provider-routing.md`. Blockscout is primary on
   every declared overlap; Etherscan is fallback there and primary only when Blockscout is unavailable. Trigger the
   fallback only on an error, malformed response, lag, plan/rate limit, or unsupported required action. A valid empty
   primary response is authoritative and must not trigger fallback.
4. Require the history provider to be indexed through the cutoff and bound every query to `endblock=<cutoff>`. For a
   newest-first native endpoint without a server-side cutoff, discard rows above the cutoff and paginate until a row at
   or below it is reached. Otherwise the result is incomplete.

If no RPC can establish the finalized/verified checkpoint, or no indexed provider can cover a required history channel
through it, report the chain as unknown/partial. Do not substitute `latest`, an explorer page count, or a provider error
for a negative result.

Example state batch; use the pinned block number, not the symbolic `finalized` tag, after checkpoint resolution:

```json
[
  {
    "jsonrpc": "2.0",
    "id": "nonce",
    "method": "eth_getTransactionCount",
    "params": ["<addr>", "<cutoff-hex>"]
  },
  {
    "jsonrpc": "2.0",
    "id": "balance",
    "method": "eth_getBalance",
    "params": ["<addr>", "<cutoff-hex>"]
  }
]
```

### Zero-State Shortcut

Zero nonce plus zero native balance can satisfy a profile's native-history shortcut only when the target row's
`accountActivityModel` is exactly `ethereum-eoa`. Default-deny the shortcut for `native-account-abstraction`,
`cross-vm`, `unknown`, a missing field, or any unrecognized value.

The shortcut does not cover ERC-20, ERC-721, or ERC-1155 transfers. For the prb-finance bootstrap profile, record both
`txlist` and `txlistinternal` as omitted by the `ethereum-eoa` zero-state invariant, then still query `tokentx` plus
`tokennfttx`. This profile-level shortcut deliberately does not claim that zero-value normal/internal calls are absent.
For a general sweep where those calls count as activity, query normal and internal history as well.

### Provider Detection

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

### Optional Quorum

Quorum is off by default: one complete authoritative-provider result is enough. If the user requests quorum, require
that many independent indexers to cover the same cutoff, profile, and channels. Blockscout PRO and a per-instance
surface backed by the same index count once. A positive quorum requires the same earliest transaction hash, block,
action/channel, and timestamp from every provider; a negative quorum requires valid empty coverage from every provider.
Unsupported actions, errors, lag, and state RPC calls are not history votes. Never weaken the requested quorum; provider
disagreement is unknown, not a majority decision.

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
chains when the user asks which chains or wants a report. A negative is valid only after every channel required by the
selected profile is covered through the pinned cutoff or satisfied by an allowed invariant.

### Etherscan

Use the advanced filter parameters when accepted by the endpoint. Set `from=<addr>&to=<addr>&fromto_opr=or` for
direction-agnostic checks:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=txlist&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=txlistinternal&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=tokentx&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=tokennfttx&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=token1155tx&from=<addr>&to=<addr>&fromto_opr=or&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
```

If an action rejects the advanced filter shape, fall back to the standard address-list shape:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=<action>&address=<addr>&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1&apikey=$ETHERSCAN_API_KEY
```

Treat `status=1` with a non-empty `result` array as positive. Treat Etherscan's "No transactions found" / empty result
as negative for that action only; still check the remaining activity actions before marking the chain inactive.

For the prb-finance bootstrap profile, a conformant `logs/getLogs` query may combine the ERC-20 and ERC-721 existence
checks because both use `Transfer(address,address,uint256)`. Set the Transfer signature as `topic0`, the left-padded
address as both `topic1` and `topic2`, `topic0_1_opr=and`, `topic0_2_opr=and`, and `topic1_2_opr=or`:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=logs&action=getLogs&fromBlock=<vector-start>&toBlock=<vector-end>&topic0=0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef&topic0_1_opr=and&topic1=<padded-addr>&topic0_2_opr=and&topic2=<padded-addr>&topic1_2_opr=or&page=1&offset=1000&apikey=$ETHERSCAN_API_KEY
```

First run that shape over a known public vector whose bounded response contains both an inbound and outbound Transfer.
Validate the saved response with `bash ./scripts/validate-etherscan-transfer-topics.sh <address> < response.json`. After
conformance, existence probes use the same topic/operator shape with `fromBlock=0`, `toBlock=<cutoff>`, and `offset=1`.
Treat an empty response as negative only when that shape is known to work for the selected chain and plan; otherwise use
separate `tokentx` and `tokennfttx` calls. This optimization does not cover ERC-1155 (`TransferSingle`/`TransferBatch`)
and never changes the general five-channel profile. Treat a returned raw log as a candidate until the emitting contract
is confirmed as ERC-20 or ERC-721; the shared signature alone does not prove the token standard.

### Blockscout

When native v2 exposes `addresses/<addr>/counters`, validate its required string counters with
`bash ./scripts/validate-blockscout-address-counters.sh < response.json`. If the instance is indexed through the cutoff,
`transactions_count=0` can satisfy the normal-transaction channel and `token_transfers_count=0` can satisfy all token
transfer channels. The endpoint has no cutoff parameter: a nonzero counter requires a bounded item query, and a zero
counter is usable only when the indexer's head is at or above the pinned cutoff. Counters do not cover internal
transactions. A missing, malformed, or unvalidated counter is only an optimization miss.

For bounded existence probes, prefer the Etherscan-compatible layer because it supports `sort`, `page`, `offset`, and
block bounds. On the PRO host, pass `$BLOCKSCOUT_API_KEY` with `authorization: Bearer $BLOCKSCOUT_API_KEY` rather than a
query parameter:

```text
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=txlist&address=<addr>&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=txlistinternal&address=<addr>&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=tokentx&address=<addr>&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=tokennfttx&address=<addr>&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1
https://api.blockscout.com/v2/api?chain_id=<id>&module=account&action=token1155tx&address=<addr>&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1
```

If the PRO host returns `404` or no key is available, resolve the per-instance URL and repeat the same five compat
actions against that host:

```bash
CS_instance_url=$(./scripts/resolve-chain.sh <id> | sed -n 's/^instance_url=//p')
curl -s "${CS_instance_url}api?module=account&action=txlist&address=<addr>&startblock=0&endblock=<cutoff>&sort=desc&page=1&offset=1"
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
target mainnets. Report only positive balances by default, plus explicit provider gaps. Pin the finalized/verified
checkpoint before reading native state. Holdings endpoints that expose only the provider's current indexed head are not
cutoff-exact; record their own observed/indexed freshness and do not merge them into the checkpointed native-state
claim.

### Etherscan

Always check native balance when the chain is queryable:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=balance&address=<addr>&tag=<cutoff-hex>&apikey=$ETHERSCAN_API_KEY
```

When `pro_endpoints=true`, enumerate token and NFT holdings:

```text
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=addresstokenbalance&address=<addr>&page=1&offset=100&apikey=$ETHERSCAN_API_KEY
https://api.etherscan.io/v2/api?chainid=<id>&module=account&action=addresstokennftbalance&address=<addr>&page=1&offset=100&apikey=$ETHERSCAN_API_KEY
```

Paginate PRO holdings while returned pages are full. If `pro_endpoints=false`, Etherscan can only check native balance
and known single-token balances (`tokenbalance` with a provided contract). Use Blockscout for full holdings when
Blockscout covers the chain; otherwise report token/NFT holdings as not enumerable through available indexed providers.
Etherscan's free/Lite historical-balance window may reject an older pinned cutoff; fall back to the already selected RPC
state result rather than changing the cutoff.

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
eth_getBalance(<addr>, <cutoff-hex>)
```

Do not infer token/NFT emptiness from RPC-only coverage. Token and NFT holdings require a token contract list or indexed
provider.

## Output

For a yes/no answer, lead with the boolean result and the first positive chain/action found. Every chain result must
identify its profile, fixed checkpoint, channel coverage, authoritative history provider, fallback use, and quorum
status. For reports, group by chain:

```markdown
| Chain | Checkpoint | Profile | Activity | Covered channels | Coverage | Provider/quorum |
| ----- | ---------- | ------- | -------- | ---------------- | -------- | --------------- |
```

Express the checkpoint as at least `chain_id:block_number:block_hash` plus the cutoff timestamp. Use `complete` only
when every profile channel is bounded through that checkpoint or covered by a named safe invariant. Use `partial` for
known gaps and `unknown` when no defensible negative can be formed. Include explicit gaps such as "Etherscan paid-chain
unavailable on free plan; Blockscout absent", "Blockscout index behind cutoff", or "RPC native balance only".

For machine-readable results, expose `checked` and `omitted` channel arrays rather than collapsing coverage into prose.
The prb-finance bootstrap vocabulary is `nonce`, `native-balance`, `txlist`, `txlistinternal`, `tokentx`, and
`tokennfttx`; the general profile also requires `token1155tx`. Record whether the row was reused `fromCheckpoint`, and
key reusable evidence by normalized address, chain ID, goal, requested cutoff, `accountActivityModel`, and the
checkpoint schema/semantics so a policy change cannot reuse an incompatible negative.

Do not say "inactive on all EVM chains" unless the checked target scope and profile are clear. For the bootstrap
profile, say "no prb-finance bootstrap-profile activity found"; reserve "no indexed activity found across the target
mainnets" for a complete general five-channel sweep.

## Provider Docs

- Etherscan introduction: <https://docs.etherscan.io/introduction>
- Etherscan advanced normal transaction filter:
  <https://docs.etherscan.io/api-reference/endpoint/advanced-filter-txlist>
- Etherscan advanced ERC-20 transfer filter: <https://docs.etherscan.io/api-reference/endpoint/advanced-filter-tokentx>
- Blockscout PRO API: <https://docs.blockscout.com/devs/pro-api>
