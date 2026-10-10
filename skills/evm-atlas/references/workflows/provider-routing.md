# Provider Routing

Read this reference only after you resolve a chain in `references/generated/target-mainnets.json`.

## Discrete Read Contract

This workflow owns every bounded JSON-RPC read, including reads requested by `cli-cast` for transaction preparation,
simulation evidence, fee or nonce resolution, and post-broadcast verification. Accept the resolved chain, method and
exact parameters or call object, block selector or checkpoint requirement, and purpose. Return the resolved chain name
and ID, exact provider route, result, observed block or checkpoint, and coverage gaps.

Keep the read here until it completes. Never invoke `cli-cast` for JSON-RPC transport or return a signing, mutation, or
broadcast command. For two or more compatible contract reads on one chain, use Multicall3 at
`0xcA11bde05977b3631167028862bE2a173976CA11`. Do not batch calls whose result depends on the original `msg.sender`.

## Account and Transaction Data

Choose one authoritative history provider per chain and sweep. The selected provider is authoritative for that result.
It is not a globally canonical source. Keep a second provider only as a fallback:

1. On covered overlaps, use Blockscout. Prefer it especially when the detected Etherscan plan cannot serve the chain or
   its pagination/rate/PRO limits reduce sweep completeness. Also prefer it when Blockscout's native holdings/counters
   avoid those limits.
2. Otherwise, use Etherscan through the official `etherscan` CLI when the chain is in
   `references/generated/etherscan-chains.md`, the detected plan can query it after dated access notes, and the needed
   actions accept the fixed cutoff.
3. Use the other indexed provider as fallback when the authoritative provider is unavailable, malformed, behind the
   cutoff, rate/plan limited, or missing a required action. A valid empty response is a completed negative, not a
   fallback trigger. Move the affected result to the fallback. Do not silently combine two negative responses into one
   complete result.
4. If neither indexed provider covers the target, use its listed public RPC only for facts that JSON-RPC can prove.
   Missing indexed history remains unknown, never empty.

On overlaps, prefer Blockscout if the generated Etherscan table and dated notes require paid access that the detected
plan lacks. Also prefer Blockscout if its holdings/counters make the requested result more complete.

A community Free quota error means the chain-wide shared pool is exhausted. Retain its reset time. Use the indexed
fallback, or wait until that reset. It is not a zero result or a per-key throttle. Rotating keys or briefly retrying
cannot restore the pool.

Do not infer API support or UI availability from an Etherscan-shaped explorer URL. Gnosis (`100`) still has Etherscan
API access on paid plans after Gnosisscan's UI closure. Use its listed Blockscout instance for browser evidence and
explorer links.

For Etherscan CLI 1.1.1 commands, plan gating, manual pagination, output validation, and direct API exceptions, see
`references/explorers/etherscan-api.md`. Pass the resolved numeric `--chain` and `--output json`. CLI stdout is the
unwrapped result, not an API envelope. Capture exit code and stderr. Blank stdout or a failed command does not prove
empty history. A bounded `--all` result can exit 0 while truncated. Prefer explicit pages for complete evidence.

Use existing CLI authentication without inspecting its saved key. Do not require an environment key for CLI reads. Never
pass `--api-key`, call `whoami`, or change login/configuration. The CLI's static chain metadata does not override the
target registry or dated access gates.

For raw Blockscout endpoint parameters, plan gating, and error handling, see `references/explorers/blockscout-api.md`.

## Checkpoints and State

Fix one required ISO-8601 UTC cutoff for the whole sweep. Resolve it once per chain to an exact finalized or otherwise
independently verified block at or before that time. Record the requested cutoff, resolution kind (`finalized` or
`verified`), block number, hash, timestamp, and observation time.

For a timestamp lookup, prove that the returned `B` is the greatest block at or before the cutoff. Check
`B.timestamp <= requestedAt` and either `B+1.timestamp > requestedAt` or independent evidence that `B` is the current
finalized head.

Reuse that exact checkpoint in every request. Do not mix `latest`, different provider heads, or a newly resolved block
into the same result. If no route can establish the checkpoint, mark the result unknown. Do not invent a checkpoint.

Batch `eth_getTransactionCount` and `eth_getBalance` with the EIP-1898 `{ blockHash, requireCanonical: true }` selector
before indexed history. If a provider rejects that selector, a numeric fallback requires matching block-number/hash
headers from the same endpoint immediately before and after the batch. Otherwise, try the next RPC or report unknown.

Etherscan CLI 1.1.1 `--tag` accepts a hex height, not an EIP-1898 object. Its numeric state reads still require the
same-route header checks. Use `proxy eth_getBlockByNumber --tag CHECKPOINT_HEX --boolean false` for those headers. CLI
output alone does not establish hash binding or finality. Holdings endpoints without a block selector are separately
timed evidence, not checkpointed history.

The target row's `accountActivityModel` controls whether zero nonce plus zero balance may satisfy a profile's
native-history shortcut:

- Allow the shortcut only for exact `ethereum-eoa`.
- Default-deny it for `native-account-abstraction`, `cross-vm`, `unknown`, a missing field, or an unrecognized value.
- Under the `bootstrap-discovery` profile, the exact `ethereum-eoa` zero-state invariant may omit both `txlist` and
  `txlistinternal` wholesale. That profile counts a successful outgoing normal row or a successful positive-value
  normal/internal row touching the address. Zero-value inbound normal/internal noise is outside it. The invariant never
  covers token/NFT transfers. Apply the profile rules in `references/workflows/address-sweeps.md` before you call an
  address inactive. A general policy that counts zero-value calls must still query those channels.

For `cross-vm`, scope all state, history, and negative claims to the chain's EVM execution environment. EVM evidence
does not cover the native non-EVM account environment and cannot prove whole-chain inactivity.

An indexer result is cutoff-complete only when the provider is synced through the checkpoint and the query is bounded to
it. Filter or paginate past post-cutoff rows. An unbounded newest-first empty/non-empty page is not equivalent to a
checkpointed result.

Quorum is optional and must be explicit. When requested, enforce it strictly across independent indexed providers that
cover the same checkpoint and channel set. PRO and per-instance Blockscout surfaces backed by the same index are one
provider. Descending one-row probes establish existence only.

For a positive quorum, every provider must query every required channel ascending from genesis or fully paginate its
bounded result. Every provider must apply the same profile predicates. Every provider must return the same earliest
qualifying transaction hash, block, action/channel, and timestamp.

A negative quorum requires valid empty coverage from every provider. Errors and unsupported channels are not votes.
Never weaken the requested quorum. Report disagreement as unknown.

## RouteMesh and Public RPC

Use the `routemesh` CLI exclusively for RouteMesh. For HTTP RPC, require the target row's `routeMesh: true` and the
exact chain ID in `routemesh chains --transport rpc`. The `routeMesh` flag describes HTTP coverage. Check WebSocket
coverage separately with `routemesh chains --transport ws`. Never construct a RouteMesh URL or inspect, request, or
print an API key.

`routemesh chains` prints the full catalog as one JSON array of `{"chain_id":"<decimal string>","name":"..."}` objects.
Never print the full list. Check one chain on both transports with a bounded selector:

```sh
for transport in rpc ws; do
  routemesh chains --transport "$transport" \
    | jq -c --arg id CHAIN_ID --arg t "$transport" '{transport: $t, listed: any(.[]; .chain_id == $id)}'
done
```

Each line reports one transport, such as `{"transport":"rpc","listed":true}`. Compare `chain_id` as a string. Do not
`grep` the single-line JSON.

Assume the user has already run `routemesh init`. On `insufficient_credits`, stop the route. Report that the RouteMesh
account needs credits.

For other credential errors, stop that route. Tell the user to obtain an API key from
<https://routeme.sh/app/consumer/api-keys>. Tell the user to run `routemesh init`, then retry. Do not run initialization
or fall back to a raw RouteMesh endpoint.

If `routemesh` is unavailable, report that the CLI is required. Do not use direct HTTP as a workaround.

Use `routemesh ping CHAIN_ID` to verify a RouteMesh chain route. Use `routemesh rpc CHAIN_ID METHOD --params=JSON` for
one read-only JSON-RPC request, or `--json=JSON` for a complete request or batch. Never pass `--allow-write`.

For an activity watch or a bounded wait before you recheck a pending receipt, confirm that `routemesh schema subscribe`
is available. Confirm that the exact target chain is in `chains --transport ws`. Then use:

```sh
routemesh --timeout 60s subscribe CHAIN_ID newHeads --count 1
routemesh --timeout 60s subscribe CHAIN_ID logs --json=FILTER --count 1
routemesh --timeout 30s subscribe CHAIN_ID newPendingTransactions --count 5
```

`FILTER` accepts only `address` and `topics`. Use `--json -` for stdin. Choose the subscription and filter that answer
the request. Type availability depends on the provider. If the local CLI lacks `schema subscribe`, report that it needs
updating.

Keep a finite count (1–1000) and timeout. Both JSON and NDJSON buffer until the full count arrives. A timeout or
disconnect fails with no partial stdout and no automatic reconnect. Never interpret it as evidence of no activity.

Preserve reorg heads and logs with `removed: true`. Notifications do not prove finality, historical completeness, or a
successful transaction. After a notification, verify the relevant receipt, state, or explicit log range through the
existing evidence commands. A new subscription does not recover events missed during a disconnect. Subscriptions do not
replace historical sweeps or bridge-protocol status checks.

RouteMesh routing depends on the method. A successful command proves only that exact method, parameters, and block. It
does not establish archive coverage for the key, chain, or another method.

| Class                 | Representative methods                                                                         | Archive-state requirement                                                                          |
| --------------------- | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Current or block data | `eth_chainId`, `eth_blockNumber`, `eth_getBlockByNumber`                                       | No historical state trie required                                                                  |
| Historical chain data | `eth_getTransactionByHash`, `eth_getTransactionReceipt`, `eth_getBlockReceipts`, `eth_getLogs` | Does not inherently read archive state. An upstream can prune or incompletely serve old chain data |
| Historical state      | Old-block `eth_call`, `eth_getBalance`, `eth_getCode`, `eth_getStorageAt`, `eth_getProof`      | Requires an archive-capable state path for that method and block                                   |

For historical state requests:

1. Resolve and verify one exact block number, hash, and timestamp before querying state. Reuse that checkpoint for every
   request.
2. Send the EIP-1898 `{ blockHash, requireCanonical: true }` selector with `routemesh rpc` where the method and provider
   support it.
3. When a numeric block fallback is required, use the verified block number and retain the same-endpoint
   block-number/hash consistency checks immediately before and after the request batch.
4. Treat pruning, missing-trie/state-unavailable errors, malformed data, or a failed block-identity check as a coverage
   failure. Try the ordered independent RPC fallback. If that cannot serve the request, report the state as unknown.
   Never turn the failure into a zero balance or empty state.
5. On an error or suspicious `null`, retain the CLI-provided batch ID for traceability. Never persist a credential.

Do not rerun a failed RouteMesh command arbitrarily. The CLI already applies its bounded retry policy. Another attempt
does not guarantee a different archive-capable pathway.

For a specific transaction receipt:

1. Use `routemesh receipt CHAIN_ID TX_HASH`. It verifies the transaction, receipt, and exact block header. It recovers a
   proven receipt through `eth_getBlockReceipts` when the direct receipt is `null`.
2. Accept its receipt only after the full transaction hash, block number, and block hash match the target checkpoint.
3. On a CLI evidence or provider failure, use the ordered indexed-provider or public-RPC fallback. If none supplies the
   receipt, report receipt coverage as unknown.

When other evidence proves the transaction, a method-specific `null` is inconclusive. It does not prove that the
transaction does not exist. A successful receipt lookup repairs only that receipt path, never unrelated missing
historical-state evidence.

For `eth_getLogs`, pass an exact checkpoint-bounded filter to `routemesh logs --json=FILTER CHAIN_ID`. The CLI splits
larger inclusive ranges into deterministic 10,000-block chunks and returns its checkpoint evidence. Do not treat a CLI
error as an empty log result.

Otherwise, first verify the target's `primaryPublicRpc` with `eth_chainId`. Issue bounded direct HTTP JSON-RPC requests
against it. Then try `references/generated/target-fallback-rpcs.json` in order. Do not hand public-RPC reads to
`cli-cast`. Public RPCs are best-effort and may be rate limited.

### Exact simulation failures

Preserve the exact transaction object, block selector, CLI exit code, stdout JSON-RPC response, and redacted stderr
diagnostics. A nonzero exit can accompany useful JSON-RPC error evidence. Capture both streams before you handle it.
Retain every correlation ID. RouteMesh uses `X-Batch-Id` for individual requests and comma-separated `X-Batch-Ids` for
batches. Its [debugging guide](https://routeme.sh/docs/intro/debugging) documents these IDs.

When a simulation contradicts its supplied fields or checkpointed state:

1. Verify the CLI payload with `--dry-run`, including sender, target, value, calldata, nonce, transaction type, gas, and
   fee fields. For an affordability error, compare the reported requirement with the exact transaction's upfront
   reserve. A reported gas allowance different from the supplied limit is a simulation-integrity warning.
2. Replay the unchanged call and estimate through the ordered independent public-RPC fallback at the same verified
   checkpoint. Verify its chain ID first. A successful estimate does not validate a failed call. The two methods may use
   different upstreams. Do not repeatedly retry the same route or switch transaction type to explain a mismatch.
3. If needed, use a bounded synthetic probe with different explicit gas limits to test whether the route honors gas. For
   a verified ordinary EOA with empty calldata, a below-intrinsic limit must fail. Where state overrides are supported,
   an isolated GAS-opcode probe can establish the executed allowance. These probes diagnose the provider. Zeroing value
   or fees, changing gas, or overriding state never substitutes for the exact transaction simulation.
4. When authenticated logs are available, use `$chromium-browser` to correlate request IDs with the logged parameters,
   result, and upstream. Preserve the distinction between the service's received payload and any unobserved forwarded
   payload. A provider label alone does not prove which layer changed execution semantics.

Return the exact simulation outcome and any provider discrepancy separately. Keep the consuming workflow's simulation
and approval requirements. Diagnostic success does not authorize signing or broadcast. A failing RPC route does not
establish chain-wide transaction-type incompatibility or justify changing static chain metadata.

## Explorer Links

For address and transaction links, substitute `{address}` in the target row's `explorerAddressUrl` or `{tx_hash}` in
`explorerTxUrl`. Preserve the full template, including query parameters. Address history may use a different service
from the transaction explorer. For block and token links, use `explorerUrl` plus
`references/explorers/explorer-paths.json`.

Verify nonstandard explorers in their UI. Ronin does not reliably follow Etherscan paths, and its chain ID collides with
a non-target Chainscout entry. Use `$chromium-browser` for OKLink and Ronin browser evidence.

Browser availability does not establish a supported programmatic API route. Use documented credentials for API access.
Never extract or reproduce the site's private request-signing headers.

### ZKsync official explorer fallback

For resolved ZKsync Era (`324`), a Blockscout plan-gating error does not exhaust historical sources. Inspect
`https://explorer.zksync.io/address/<address>` in Chromium. Its Transactions and Transfers tabs use
`https://block-explorer-api.mainnet.zksync.io`. Preserve the actual request parameters observed in the browser.

Verified on 2026-10-02, the newest-first lists crossed a requested monthly boundary and matched independently verified
token receipts. Stop pagination only after you cover the whole requested interval. Verify the fixed cutoff block and
successor through RPC.

The official [API docs](https://block-explorer-api.mainnet.zksync.io/docs) and
[OpenAPI schema](https://block-explorer-api.mainnet.zksync.io/docs-json) document Etherscan-compatible `/api` queries:
`module=account` with `action=txlist`, `txlistinternal`, `tokentx`, or `tokennfttx`. Use `address`, `startblock`,
`endblock`, `page`, `offset`, and `sort`. The documented cap is 1000 items, with at most 100 per page. Partition block
ranges when necessary. Validate HTTP, JSON status, pagination, and timestamps before treating an empty period as
covered.

Keep the channel limits explicit: `txlistinternal` covers transfers, not every zero-value internal call. The
[explorer token model](https://github.com/matter-labs/block-explorer/blob/main/packages/api/src/token/token.entity.ts)
supports native currency, ERC-20, and ERC-721. Do not infer ERC-1155 coverage from its Transfers tab or `tokennfttx`.
When ERC-1155 transfer coverage is required, query standard `TransferSingle` and `TransferBatch` logs through provider
routing over the verified block range. Filter the wallet separately in indexed `from` and `to` positions (topics 2 and
3). Preserve any RPC range or provider failures and use the normal public-RPC fallback.

### OKLink historical fallback

For resolved Scroll (`534352`) and Ronin (`2020`) targets, [OKLink](https://www.oklink.com/) is an independent browser
source for block details and indexed transaction history. Verified on 2026-09-08:

| Target | Browser route and useful evidence                                                                                                                                                                                                                                      | Coverage boundary                                                                                                                                                                            |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scroll | `https://www.oklink.com/scroll/address/<address>/internal` returned historical internal ETH transfers that matched Blockscout transaction hashes.                                                                                                                      | Set the requested date window explicitly. Inspect the zero-value filter. A default recent window or value-only list cannot prove complete internal-call history.                             |
| Ronin  | `https://www.oklink.com/ronin/block/<number>` serves pre-migration blocks, including the linked [2025 cutoff](https://www.oklink.com/ronin/block/51786916) and [successor](https://www.oklink.com/ronin/block/51786917). Address pages use `/ronin/address/<address>`. | Account-history pages disclose a lower bound of [block 25350000](https://www.oklink.com/ronin/block/25350000), 2023-06-27. They cannot prove earlier inactivity or genesis-complete history. |

Record the chain, source URL, observation time, requested range, displayed filters, pagination/caps, and block
identities. Convert displayed local times to UTC explicitly. Positive rows establish only the facts they show. A
complete negative still requires every required channel and interval through the fixed cutoff. Do not splice partial
provider negatives into a complete result.

Scrollscan now serves Blockscout. Its native v2 internal-transaction list can return HTTP 200 with exhausted pagination
while the compatibility API reports unprocessed internal transactions, even when global indexing indicators report
completion. Preserve that semantic failure. The native list provides useful positive evidence. It is not an independent
fallback or proof that the missing traces are empty.

For a bounded-period task, a warning on a broader historical query does not localize missing traces to that period.
Resolve and independently verify the period's start and end blocks. Then repeat the keyed compatibility query with those
exact bounds. A valid, exhausted response without the incomplete status can establish period coverage while the original
historical warning remains recorded.

Verified on 2026-10-02: a genesis-to-cutoff query returned status `2`, while the requested month alone returned status
`0`, `message="No internal transactions found"`, and `result=[]`. Accept that exact empty-result shape. Do not treat
arbitrary status `0` errors as successful negatives.

Ronin's [2026 migration announcement](https://blog.roninchain.com/p/ronin-is-home) sunsets the legacy explorer. The new
`explorer.roninchain.com` Blockscout deployment did not serve the pre-migration 2025 cutoff. The legacy
`app.roninchain.com/explorer` UI reproduced Skynet 503 responses during the check above. Verify historical coverage
separately from current-chain availability. OKLink can supply a legacy block boundary without supplying pre-2023 account
history or exact historical account state.

For post-migration Ronin history (after legacy block 55,577,490), use that Blockscout deployment: REST v2
`/api/v2/addresses/<address>/{transactions,token-transfers,internal-transactions}` or the Etherscan-compatible
`https://explorer.roninchain.com/api`, preferably through the keyed `https://api.blockscout.com/2020/api` gateway
because the keyless instance rate-limits after about ten requests. Post-migration receipts carry OP Stack `l1Fee`.
Archive state for legacy blocks is readable through RouteMesh; `api.roninchain.com/rpc` does not serve it, and Ronin
RPCs return no `trace_transaction` or `debug_traceTransaction` results. Verified 2026-10-10.

## Exceptional History

For HyperEVM (`999`) exact historical native-balance and nonce reads, do not use public JSON-RPC or RouteMesh. Those
routes can silently serve latest state for historical selectors. At the verified checkpoint, use:

```sh
etherscan --chain 999 --output json account balancehistory ADDRESS --blockno CHECKPOINT_NUMBER
etherscan --chain 999 --output json proxy eth_getTransactionCount ADDRESS --tag CHECKPOINT_HEX
```

Require `pro_endpoints=true` for `balancehistory`. Preserve the exact checkpoint and same-route header checks. If the
CLI cannot serve a required field, use only the documented Etherscan direct API exception. If the Etherscan route is
unavailable or plan-limited, report unknown. Do not fall back to RPC for these historical state facts.

For Fantom Opera (`250`) account history, do not use the unsafe FTMScout route returned by Chainscout. Read
`references/explorers/fantom-opera.md`. Preserve its partial-index boundary. GraphQL rows can provide positive evidence,
but empty account lists cannot establish historical inactivity.

For OP Mainnet data before `2021-11-11`, read `references/explorers/optimism-pre-regenesis.md` before interpreting
provider or RPC results.

For IoTeX (`4689`) nonce reads, `eth_getTransactionCount` with `latest` returns the actpool pending nonce, identical to
`pending`, on both RouteMesh and the public RPC. Only a numeric block selector returns the confirmed count. The nodes
expose no `txpool_*` methods, and `eth_getBlockByNumber("pending")` omits actpool transactions. Report the confirmed
nonce from the numeric checkpoint. Treat `pending - confirmed` as the count of queued transactions. Verified 2026-09-30:
`latest` and `pending` both returned `11` while the numeric block returned `10`, and the sender's last mined transaction
had nonce `9`.
