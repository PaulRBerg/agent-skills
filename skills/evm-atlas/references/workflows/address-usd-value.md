# Address USD Value

Use this reference for the current USD value of one or more public EVM addresses per target chain and in total: native
balances plus fungible ERC-20 tokens. NFTs and historical values are out of scope; route them to
`references/workflows/provider-routing.md`. DeFi positions stay out of every total; list them separately from
`references/workflows/debank-portfolio.md` when requested. All steps are read-only. Callers own any threshold (for
example, a "drained" or dust cutoff) and any token allow/deny policy; this workflow only returns values and coverage.

## Scope

- Validate every address (20-byte hex). Use `<addr>` placeholders in saved specs and examples.
- Check native balances on every row of `references/generated/target-mainnets.json`; native reads are cheap. A caller
  may narrow token lookups to a chain subset; report unchecked chains as out of the requested token scope, not as zero.
- For `cross-vm` rows, scope values to the chain's EVM execution environment.
- Before keyed API calls, check presence value-free: `[ -n "$BLOCKSCOUT_API_KEY" ] && echo set || echo unset`. Without
  the key, skip the Blockscout discovery route and fall through to DeBank.

## Native Balances

1. Per chain, pin one block: `routemesh rpc <chainId> eth_getBlockByNumber --params='["finalized",false]'` and record
   its tag, number, hash, and timestamp. Use `"latest"` when the chain rejects `finalized` or returns block `0x0` for
   it, and say so: RouteMesh served Chiliz (`88888`) genesis for both `finalized` and `safe` (verified 2026-10-01). For
   a post-transfer check, when the finalized head predates the caller's verified receipt block, use a canonical
   `"latest"` checkpoint at or after that receipt block; record the reason and label it unfinalized. Never use
   pre-transfer state as the post-transfer balance.
2. Send one `routemesh rpc <chainId> --json -` batch with an `eth_getBalance` request per address, each using the
   EIP-1898 `{ "blockHash": "<hash>", "requireCanonical": true }` selector. Apply the numeric-block fallback and
   same-endpoint block-identity checks from `provider-routing.md` when a provider rejects that selector.
3. For a row without RouteMesh HTTP coverage, or after a RouteMesh coverage failure, follow the `primaryPublicRpc` then
   `references/generated/target-fallback-rpcs.json` order in `provider-routing.md`. A failed native read is a gap, never
   zero.
4. Convert wei to native units with fixed-precision decimal arithmetic (`bc` with `scale=18`, or Python `decimal`).
   Never use binary floats for amounts, prices, or products.

## Prices

- Price natives with one `cg price --ids <id,...> -o json` call covering every distinct native asset (ETH-native chains
  share one ID). Resolve unknown CoinGecko IDs once with `cg search <symbol-or-name> -o json`; never treat a symbol as
  unique.
- Price each confirmed token holding by contract from CoinGecko. Resolve each chain's platform once from
  `https://api.coingecko.com/api/v3/asset_platforms`, matching `chain_identifier` to the chain ID. Map contracts to coin
  IDs with one keyless `https://api.coingecko.com/api/v3/coins/list?include_platform=true` download (match the platform
  and lowercased contract), then price every mapped ID in one `cg price --ids <id,...> -o json` call.
- For a contract the coin list does not map, request
  `https://api.coingecko.com/api/v3/simple/token_price/<platform>?contract_addresses=<contract>&vs_currencies=usd`. The
  keyless endpoint accepts one contract per request, so query only holdings with a nonzero confirmed balance and pace
  requests. It can also quote contracts CoinGecko does not list (`coins/<platform>/contract/<contract>` returns
  `coin not found`), including a dead token quoted at hundreds of dollars; treat such a quote as unlisted, count the
  token as unpriced, and report the quote separately.
- Use an indexer price (Blockscout `exchange_rate` on `addresses/<addr>` for natives or on a token holding, a DeBank or
  Blockscan row price) only when CoinGecko omits the native asset, has no platform for the chain, or has no price for
  the contract. Label that price with its source and apply Pricing Hygiene; Blockscout has priced tokens it lists with a
  zero market cap.
- Record each price's source and the UTC observation time.

## Fungible Tokens

Indexed holdings can be stale (a Blockscout list has shown a USDT balance whose on-chain `balanceOf` was zero), so
indexers only discover token contracts: amounts come from RPC and prices from CoinGecko. Per chain and address, take the
union of these discovery sources:

1. **Caller candidates.** Always include contracts the caller supplies, such as tokens from its own transfer history.
2. **Blockscout.** For targets the keyed gateway serves (see `references/generated/blockscout-chains.md` and
   `references/explorers/blockscout-api.md` for the per-instance exception), page
   `https://api.blockscout.com/<chainId>/api/v2/addresses/<addr>/tokens?type=ERC-20` until `next_page_params` is `null`.
   Keep each holding's `token.address_hash` and `decimals`, plus `exchange_rate` as a fallback price. An HTTP `402`
   (plan-gated chain; see `references/explorers/blockscout-endpoints.md`) or other failure falls through to DeBank; do
   not retry a `402`.
3. **DeBank.** For target chains Blockscout does not cover, gates, or fails on, run the collector per
   `references/workflows/debank-portfolio.md` (token discovery for one address or many; no `Show all` click). From each
   `ok` record take the token contracts per target chain ID (`chainId` is the `chain/list` `network_id`), and the price
   as a fallback, and record its `observedAt`. A `failed` record is a discovery gap for that address's DeBank chains.
4. **Blockscan.** For remaining target chains DeBank lacks or fails on, use the Chromium flow in
   `references/workflows/blockscan-balances.md`: match chains by exact `data-chainid`, take each row's token contract
   (and price as a fallback) from `#js-chain-table`, and record `Last updated`.

When no indexer lists a chain's tokens, report an ERC-20 discovery gap for that chain even if caller candidates were
confirmed. Never assume zero tokens.

Confirm every discovered holding on-chain: per chain, batch `eth_call` `balanceOf(<addr>)` (selector `0x70a08231`) for
each token at the pinned block hash, through the same route as that chain's native reads. Add `decimals()`
(`0x313ce567`) when discovery did not supply it. Use the RPC amount: USD value = `balanceOf / 10^decimals × price`. An
RPC zero drops the holding; a failed or malformed confirmation is a coverage gap for that token, never the indexed
amount.

## Bulk Mode

For many addresses, run API passes first: native batches across all target chains, Blockscout token lists, one
`balanceOf` confirmation batch per chain, then CoinGecko contract prices for confirmed holdings. Run the DeBank
collector for the addresses that still have gap chains, across at most two owned pages as `debank-portfolio.md` directs,
then Blockscan one page at a time for what remains. Keep request concurrency at or below each provider's limit
(Blockscout `x-ratelimit-limit`; CoinGecko plan quota); back off on `429` as the provider references direct. Never run
unbounded parallel requests.

## Pricing Hygiene

- A token without a provider price contributes `$0` and is listed as unpriced with its amount and contract.
- Flag a priced token as suspicious when its price looks spoofed: an unverified or unknown contract using a major symbol
  or name, a value implausible for its market (for example, exceeding its market cap or liquidity), or a price that
  disagrees sharply with CoinGecko for the same asset. List suspicious tokens separately and exclude them from every
  total.

## Nil and Totals

- A chain is `nil` when its native balance is exactly zero and it has no confirmed, priced, non-suspicious token
  holdings. A chain with a native or ERC-20 coverage gap is never `nil`; report its value as a lower bound or unknown.
- The address total sums chain values excluding suspicious tokens. It is `nil` only when every checked chain is `nil`
  and no gaps remain; with gaps, label it a lower bound.

## Output

Lead each address with `### ⛓️ <addr> — <status word>`. Use one table row per chain with value:

| Chain (ID) | Native (amount / USD) | Priced tokens (amount / USD) | Chain USD | Source | Block / checkpoint |
| ---------- | --------------------- | ---------------------------- | --------- | ------ | ------------------ |

Collapse `nil` chains into a count with their chain IDs. Then list the address total, unpriced tokens, suspicious
tokens, `⚠️ Coverage gaps` (chain, channel, cause), and price source with UTC timestamp. Show native amounts at full
precision without exponent notation. When a caller requests machine-readable output, emit the same fields as JSON with
decimal strings for every amount and USD value.
