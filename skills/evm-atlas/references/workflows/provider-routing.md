# Provider Routing

Read this reference only after resolving a chain in `references/generated/target-mainnets.json`.

## Discrete Read Contract

This workflow owns every bounded JSON-RPC read, including reads requested by `cli-cast` for transaction preparation,
simulation evidence, fee or nonce resolution, and post-broadcast verification. Accept the resolved chain, method and
exact parameters or call object, block selector or checkpoint requirement, and purpose. Return the resolved chain name
and ID, exact provider route, result, observed block or checkpoint, and coverage gaps.

Keep the read here until it completes. Never invoke `cli-cast` for JSON-RPC transport or return a signing, mutation, or
broadcast command. For two or more compatible contract reads on one chain, use Multicall3 at
`0xcA11bde05977b3631167028862bE2a173976CA11`; do not batch calls whose result depends on the original `msg.sender`.

## Account and Transaction Data

Choose one authoritative history provider per chain and sweep. A provider is authoritative for that result, not a
globally canonical source. Keep a second provider only as a fallback:

1. Use Blockscout on covered overlaps, especially when Etherscan cannot serve the chain on the detected plan, its
   pagination/rate/PRO limits make the requested sweep less complete, or Blockscout's native holdings/counters avoid
   those limits.
2. Otherwise use Etherscan V2 when the chain is in `references/generated/etherscan-chains.md`, the detected plan can
   query it after applying dated access notes, and the needed actions accept the fixed cutoff.
3. Use the other indexed provider as fallback when the authoritative provider is unavailable, malformed, behind the
   cutoff, rate/plan limited, or missing a required action. A valid empty response is a completed negative, not a
   fallback trigger. Move the affected result to the fallback; do not silently splice two negative responses into one
   complete result.
4. If neither indexed provider covers the target, use its listed public RPC only for facts that JSON-RPC can prove.
   Missing indexed history remains unknown, never empty.

On overlaps, prefer Blockscout when the generated Etherscan table and its dated notes require paid access that the
detected plan lacks, or its holdings/counters make the requested result more complete. A community Free quota error
exhausts a chain-wide shared pool: retain its reset time and use the indexed fallback, or wait until that reset. It is
not a zero result or a per-key throttle; rotating keys or briefly retrying cannot restore the pool.

Do not infer API support or UI availability from an Etherscan-shaped explorer URL. Gnosis (`100`) still has Etherscan
API access on paid plans after Gnosisscan's UI closure; use its listed Blockscout instance for browser evidence and
explorer links.

For raw Etherscan V2 endpoint parameters, plan gating, and error handling, see `references/explorers/etherscan-api.md`.
For raw Blockscout endpoint parameters, plan gating, and error handling, see `references/explorers/blockscout-api.md`.

## Checkpoints and State

Fix one required ISO-8601 UTC cutoff for the whole sweep. Resolve it once per chain to an exact finalized or otherwise
independently verified block at or before that time. Record the requested cutoff, resolution kind (`finalized` or
`verified`), block number, hash, timestamp, and observation time. For a timestamp lookup, prove that the returned `B` is
the greatest block at or before the cutoff by checking `B.timestamp <= requestedAt` and either
`B+1.timestamp > requestedAt` or that `B` is independently the current finalized head. Reuse that exact checkpoint in
every request. Do not mix `latest`, different provider heads, or a newly resolved block into the same result. If no
route can establish the checkpoint, mark the result unknown rather than inventing one.

Batch `eth_getTransactionCount` and `eth_getBalance` with the EIP-1898 `{ blockHash, requireCanonical: true }` selector
before indexed history. If a provider rejects that selector, a numeric fallback requires matching block-number/hash
headers from the same endpoint immediately before and after the batch; otherwise try the next RPC or report unknown. The
target row's `accountActivityModel` controls whether zero nonce plus zero balance may satisfy a profile's native-history
shortcut:

- Allow the shortcut only for exact `ethereum-eoa`.
- Default-deny it for `native-account-abstraction`, `cross-vm`, `unknown`, a missing field, or an unrecognized value.
- Under the `bootstrap-discovery` profile, the exact `ethereum-eoa` zero-state invariant may omit both `txlist` and
  `txlistinternal` wholesale. That profile counts a successful outgoing normal row or a successful positive-value
  normal/internal row touching the address; zero-value inbound normal/internal noise is outside it. The invariant never
  covers token/NFT transfers. Apply the profile rules in `references/workflows/address-sweeps.md` before calling an
  address inactive; a general policy that counts zero-value calls must still query those channels.

For `cross-vm`, scope all state, history, and negative claims to the chain's EVM execution environment. EVM evidence
does not cover the native non-EVM account environment and cannot prove whole-chain inactivity.

An indexer result is cutoff-complete only when the provider is synced through the checkpoint and the query is bounded to
it. Filter or paginate past post-cutoff rows; an unbounded newest-first empty/non-empty page is not equivalent to a
checkpointed result.

Quorum is optional and must be explicit. When requested, enforce it strictly across independent indexed providers that
cover the same checkpoint and channel set. PRO and per-instance Blockscout surfaces backed by the same index are one
provider. Descending one-row probes establish existence only. For a positive quorum, every provider must query every
required channel ascending from genesis or fully paginate its bounded result, apply the same profile predicates, and
return the same earliest qualifying transaction hash, block, action/channel, and timestamp. A negative quorum requires
valid empty coverage from every provider. Errors and unsupported channels are not votes; never weaken the requested
quorum, and report disagreement as unknown.

## RouteMesh and Public RPC

Use the `routemesh` CLI exclusively for RouteMesh. For HTTP RPC, require the target row's `routeMesh: true` and the
exact chain ID in `routemesh chains --transport rpc`. The `routeMesh` flag describes HTTP coverage; check WebSocket
coverage separately with `routemesh chains --transport ws`. Never construct a RouteMesh URL or inspect, request, or
print an API key.

Assume the user has already run `routemesh init`. On `insufficient_credits`, stop the route and report that the
RouteMesh account needs credits. For other credential errors, stop that route and tell the user to obtain an API key
from <https://routeme.sh/app/consumer/api-keys>, run `routemesh init`, then retry. Do not run initialization or fall
back to a raw RouteMesh endpoint. If `routemesh` is unavailable, report that the CLI is required; do not work around it
with direct HTTP.

Use `routemesh ping CHAIN_ID` to verify a RouteMesh chain route. Use `routemesh rpc CHAIN_ID METHOD --params=JSON` for
one read-only JSON-RPC request, or `--json=JSON` for a complete request or batch. Never pass `--allow-write`.

For an activity watch or a bounded wait before rechecking a pending receipt, confirm `routemesh schema subscribe` is
available and the exact target chain is in `chains --transport ws`, then use:

```sh
routemesh --timeout 60s subscribe CHAIN_ID newHeads --count 1
routemesh --timeout 60s subscribe CHAIN_ID logs --json=FILTER --count 1
routemesh --timeout 30s subscribe CHAIN_ID newPendingTransactions --count 5
```

`FILTER` accepts only `address` and `topics`; use `--json -` for stdin. Choose the subscription and filter that answer
the request. Type availability is provider-dependent. If the local CLI lacks `schema subscribe`, report that it needs
updating. Keep a finite count (1–1000) and timeout; both JSON and NDJSON buffer until the full count arrives. A timeout
or disconnect fails with no partial stdout and no automatic reconnect. Never interpret it as evidence of no activity.

Preserve reorg heads and logs with `removed: true`. Notifications do not prove finality, historical completeness, or a
successful transaction. After a notification, verify the relevant receipt, state, or explicit log range through the
existing evidence commands. A new subscription does not recover events missed while disconnected; subscriptions do not
replace historical sweeps or bridge-protocol status checks.

RouteMesh routing is method-specific, so a successful command proves only that exact method, parameters, and block. It
does not establish archive coverage for the key, chain, or another method.

| Class                 | Representative methods                                                                         | Archive-state requirement                                                                            |
| --------------------- | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Current or block data | `eth_chainId`, `eth_blockNumber`, `eth_getBlockByNumber`                                       | No historical state trie required                                                                    |
| Historical chain data | `eth_getTransactionByHash`, `eth_getTransactionReceipt`, `eth_getBlockReceipts`, `eth_getLogs` | Not inherently an archive-state read, but an upstream can prune or incompletely serve old chain data |
| Historical state      | Old-block `eth_call`, `eth_getBalance`, `eth_getCode`, `eth_getStorageAt`, `eth_getProof`      | Requires an archive-capable state path for that method and block                                     |

For historical state requests:

1. Resolve and verify one exact block number, hash, and timestamp before querying state. Reuse that checkpoint for every
   request.
2. Send the EIP-1898 `{ blockHash, requireCanonical: true }` selector with `routemesh rpc` where the method and provider
   support it.
3. When a numeric block fallback is required, use the verified block number and retain the same-endpoint
   block-number/hash consistency checks immediately before and after the request batch.
4. Treat pruning, missing-trie/state-unavailable errors, malformed data, or a failed block-identity check as a coverage
   failure. Try the ordered independent RPC fallback; if that cannot serve the request, report the state as unknown.
   Never turn the failure into a zero balance or empty state.
5. On an error or suspicious `null`, retain the CLI-provided batch ID for traceability, but never persist a credential.

Do not rerun a failed RouteMesh command arbitrarily. The CLI already performs its bounded retry policy, and another
attempt does not promise a different archive-capable pathway on repeat.

For a specific transaction receipt:

1. Use `routemesh receipt CHAIN_ID TX_HASH`. It verifies the transaction, receipt, and exact block header and recovers a
   proven receipt through `eth_getBlockReceipts` when the direct receipt is `null`.
2. Accept its receipt only after the full transaction hash, block number, and block hash match the target checkpoint.
3. On a CLI evidence or provider failure, use the ordered indexed-provider or public-RPC fallback. If none supplies the
   receipt, report receipt coverage as unknown.

A method-specific `null` is inconclusive when the transaction is otherwise proven; it is not proof that the transaction
does not exist. A successful receipt lookup repairs only that receipt path, never unrelated missing historical-state
evidence.

For `eth_getLogs`, pass an exact checkpoint-bounded filter to `routemesh logs --json=FILTER CHAIN_ID`. The CLI splits
larger inclusive ranges into deterministic 10,000-block chunks and returns its checkpoint evidence. Do not treat a CLI
error as an empty log result.

Otherwise issue bounded direct HTTP JSON-RPC requests against the target's `primaryPublicRpc`, first verifying it with
`eth_chainId`, then try `references/generated/target-fallback-rpcs.json` in order. Do not hand public-RPC reads to
`cli-cast`. Public RPCs are best-effort and may be rate limited.

### Exact simulation failures

Preserve the exact transaction object, block selector, CLI exit code, stdout JSON-RPC response, and redacted stderr
diagnostics. A nonzero exit can accompany useful JSON-RPC error evidence; capture both streams before handling it.
Retain every correlation ID: RouteMesh uses `X-Batch-Id` for individual requests and comma-separated `X-Batch-Ids` for
batches, as documented in its [debugging guide](https://routeme.sh/docs/intro/debugging).

When a simulation contradicts its supplied fields or checkpointed state:

1. Verify the CLI payload with `--dry-run`, including sender, target, value, calldata, nonce, transaction type, gas, and
   fee fields. For an affordability error, compare the reported requirement with the exact transaction's upfront
   reserve. A reported gas allowance different from the supplied limit is a simulation-integrity warning.
2. Replay the unchanged call and estimate through the ordered independent public-RPC fallback at the same verified
   checkpoint. Verify its chain ID first. A successful estimate does not validate a failed call: the two methods may use
   different upstreams. Do not repeatedly retry the same route or switch transaction type to explain a mismatch.
3. If needed, use a bounded synthetic probe with different explicit gas limits to test whether the route honors gas. For
   a verified ordinary EOA with empty calldata, a below-intrinsic limit must fail. Where state overrides are supported,
   an isolated GAS-opcode probe can establish the executed allowance. These probes diagnose the provider; zeroing value
   or fees, changing gas, or overriding state never substitutes for the exact transaction simulation.
4. When authenticated logs are available, use `$chromium-browser` to correlate request IDs with the logged parameters,
   result, and upstream. Preserve the distinction between the service's received payload and any unobserved forwarded
   payload. A provider label alone does not prove which layer changed execution semantics.

Return the exact simulation outcome and any provider discrepancy separately. Keep the consuming workflow's simulation
and approval requirements; diagnostic success does not authorize signing or broadcast. A failing RPC route does not
establish chain-wide transaction-type incompatibility or justify changing static chain metadata.

## Explorer Links

For address and transaction links, substitute `{address}` in the target row's `explorerAddressUrl` or `{tx_hash}` in
`explorerTxUrl`. Preserve the full template, including query parameters: address history may use a different service
from the transaction explorer. For block and token links, use `explorerUrl` plus
`references/explorers/explorer-paths.json`. Verify nonstandard explorers in their UI; Ronin does not reliably follow
Etherscan paths and its chain ID collides with a non-target Chainscout entry. Use `$chromium-browser` for OKLink and
Ronin browser evidence. Browser availability does not establish a supported programmatic API route; use documented
credentials for API access and never extract or reproduce the site's private request-signing headers.

### ZKsync official explorer fallback

For resolved ZKsync Era (`324`), a Blockscout plan-gating error does not exhaust historical sources. Inspect
`https://explorer.zksync.io/address/<address>` in Chromium. Its Transactions and Transfers tabs use
`https://block-explorer-api.mainnet.zksync.io`; preserve the actual request parameters observed in the browser. Verified
on 2026-10-02, the newest-first lists crossed a requested monthly boundary and matched independently verified token
receipts. Stop pagination only after covering the whole requested interval, and verify the fixed cutoff block and
successor through RPC.

The official [API docs](https://block-explorer-api.mainnet.zksync.io/docs) and
[OpenAPI schema](https://block-explorer-api.mainnet.zksync.io/docs-json) document Etherscan-compatible `/api` queries:
`module=account` with `action=txlist`, `txlistinternal`, `tokentx`, or `tokennfttx`; use `address`, `startblock`,
`endblock`, `page`, `offset`, and `sort`. The documented cap is 1000 items, with at most 100 per page; partition block
ranges when necessary. Validate HTTP, JSON status, pagination, and timestamps before treating an empty period as
covered.

Keep the channel limits explicit: `txlistinternal` covers transfers, not every zero-value internal call. The
[explorer token model](https://github.com/matter-labs/block-explorer/blob/main/packages/api/src/token/token.entity.ts)
supports native currency, ERC-20, and ERC-721; do not infer ERC-1155 coverage from its Transfers tab or `tokennfttx`.
When ERC-1155 transfer coverage is required, query standard `TransferSingle` and `TransferBatch` logs through provider
routing over the verified block range, filtering the wallet separately in indexed `from` and `to` positions (topics 2
and 3). Preserve any RPC range or provider failures and use the normal public-RPC fallback.

### OKLink historical fallback

For resolved Scroll (`534352`) and Ronin (`2020`) targets, [OKLink](https://www.oklink.com/) is an independent browser
source for block details and indexed transaction history. Verified on 2026-09-08:

| Target | Browser route and useful evidence                                                                                                                                                                                                                                      | Coverage boundary                                                                                                                                                                            |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scroll | `https://www.oklink.com/scroll/address/<address>/internal` returned historical internal ETH transfers that matched Blockscout transaction hashes.                                                                                                                      | Set the requested date window explicitly and inspect the zero-value filter; a default recent window or value-only list cannot prove complete internal-call history.                          |
| Ronin  | `https://www.oklink.com/ronin/block/<number>` serves pre-migration blocks, including the linked [2025 cutoff](https://www.oklink.com/ronin/block/51786916) and [successor](https://www.oklink.com/ronin/block/51786917). Address pages use `/ronin/address/<address>`. | Account-history pages disclose a lower bound of [block 25350000](https://www.oklink.com/ronin/block/25350000), 2023-06-27. They cannot prove earlier inactivity or genesis-complete history. |

Record the chain, source URL, observation time, requested range, displayed filters, pagination/caps, and block
identities. Convert displayed local times to UTC explicitly. Positive rows establish only the facts they show; a
complete negative still requires every required channel and interval through the fixed cutoff. Do not splice partial
provider negatives into a complete result.

Scrollscan now serves Blockscout. Its native v2 internal-transaction list can return HTTP 200 with exhausted pagination
while the compatibility API reports unprocessed internal transactions, even when global indexing indicators report
completion. Preserve that semantic failure; the native list is useful positive evidence, not an independent fallback or
proof that the missing traces are empty.

For a bounded-period task, a warning on a broader historical query does not localize missing traces to that period.
Resolve and independently verify the period's start and end blocks, then repeat the keyed compatibility query with those
exact bounds. A valid, exhausted response without the incomplete status can establish period coverage while the original
historical warning remains recorded. Verified on 2026-10-02: a genesis-to-cutoff query returned status `2`, while the
requested month alone returned status `0`, `message="No internal transactions found"`, and `result=[]`. Accept that
exact empty-result shape; do not treat arbitrary status `0` errors as successful negatives.

Ronin's [2026 migration announcement](https://blog.roninchain.com/p/ronin-is-home) sunsets the legacy explorer. The new
`explorer.roninchain.com` Blockscout deployment did not serve the pre-migration 2025 cutoff; the legacy
`app.roninchain.com/explorer` UI reproduced Skynet 503 responses during the check above. Verify historical coverage
separately from current-chain availability. OKLink can supply a legacy block boundary without supplying pre-2023 account
history or exact historical account state.

## Exceptional History

For HyperEVM (`999`) exact historical native-balance and nonce reads, do not use public JSON-RPC or RouteMesh: those
routes can silently serve latest state for historical selectors. At the verified checkpoint, use Etherscan V2 `account`
module's `balancehistory` action for the native balance and the `proxy` module's `eth_getTransactionCount` action with
the checkpoint's hex block tag for the nonce. If an Etherscan route is unavailable or plan-limited, report that fact as
unknown; do not fall back to RPC.

For Fantom Opera (`250`) account history, do not use the unsafe FTMScout route returned by Chainscout. Read
`references/explorers/fantom-opera.md` and preserve its partial-index boundary: GraphQL rows can provide positive
evidence, but empty account lists cannot establish historical inactivity.

For OP Mainnet data before `2021-11-11`, read `references/explorers/optimism-pre-regenesis.md` before interpreting
provider or RPC results.

For IoTeX (`4689`) nonce reads, `eth_getTransactionCount` with `latest` returns the actpool pending nonce, identical to
`pending`, on both RouteMesh and the public RPC; only a numeric block selector returns the confirmed count. The nodes
expose no `txpool_*` methods, and `eth_getBlockByNumber("pending")` omits actpool transactions. Report the confirmed
nonce from the numeric checkpoint and treat `pending - confirmed` as the count of queued transactions. Verified
2026-09-30: `latest` and `pending` both returned `11` while the numeric block returned `10`, and the sender's last mined
transaction had nonce `9`.
