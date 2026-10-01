# Address USD Value

Use this reference for the current USD value of one or more public EVM addresses per target chain and in total: native
balances plus fungible ERC-20 tokens. NFTs, DeFi positions, and historical values are out of scope; route them to
`references/workflows/provider-routing.md`. All steps are read-only. Callers own any threshold (for example, a "drained"
or dust cutoff) and any token allow/deny policy; this workflow only returns values and coverage.

## Scope

- Validate every address (20-byte hex). Use `<addr>` placeholders in saved specs and examples.
- Check native balances on every row of `references/generated/target-mainnets.json`; native reads are cheap. A caller
  may narrow token lookups to a chain subset; report unchecked chains as out of the requested token scope, not as zero.
- For `cross-vm` rows, scope values to the chain's EVM execution environment.
- Before keyed API calls, check presence value-free: `[ -n "$BLOCKSCOUT_API_KEY" ] && echo set || echo unset`. Without
  the key, the Blockscout token route is a coverage gap for every chain that needs it.

## Native Balances

1. Per chain, pin one block: `routemesh rpc <chainId> eth_getBlockByNumber --params='["finalized",false]'` (use
   `"latest"` only when the chain rejects `finalized`, and say so) and record its tag, number, hash, and timestamp.
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
- When `cg` is unavailable or omits an asset, use Blockscout `addresses/<addr>` `exchange_rate` for that chain's native
  asset on Blockscout-covered chains. Otherwise list the native as unpriced.
- Record each price's source and the UTC observation time.

## Fungible Tokens

Indexed holdings can be stale (a Blockscout list has shown a USDT balance whose on-chain `balanceOf` was zero), so use
Blockscout and Blockscan only to discover token contracts and prices. Pick one discovery route per chain and address, in
this order:

1. **Blockscout.** For targets the keyed gateway serves (see `references/generated/blockscout-chains.md` and
   `references/explorers/blockscout-api.md` for the per-instance exception), page
   `https://api.blockscout.com/<chainId>/api/v2/addresses/<addr>/tokens?type=ERC-20` until `next_page_params` is `null`.
   Keep each holding's `token.address_hash`, `decimals`, and `exchange_rate`. An HTTP `402` (plan-gated chain; see
   `references/explorers/blockscout-endpoints.md`) is a coverage gap for this route: do not retry it; fall through to
   Blockscan.
2. **Blockscan.** For target chains Blockscout does not cover, use the Chromium flow in
   `references/workflows/blockscan-balances.md`: one `https://blockscan.com/address/<addr>` page per address covers all
   its Blockscan chains. Match chains by exact `data-chainid`, take each row's token contract and price from
   `#js-chain-table`, and record `Last updated`.
3. **Gap.** If neither route covers the chain, or the route fails, report an ERC-20 coverage gap for that chain. Never
   assume zero tokens.

Confirm every discovered holding the result counts on-chain: per chain, batch `eth_call` `balanceOf(<addr>)` (selector
`0x70a08231`) for each token at the pinned block hash, through the same route as that chain's native reads. Add
`decimals()` (`0x313ce567`) when discovery did not supply it. Use the RPC amount: USD value =
`balanceOf / 10^decimals × price`. An RPC zero drops the holding; a failed or malformed confirmation is a coverage gap
for that token, never the indexed amount.

## Bulk Mode

For many addresses, run API passes first: native batches across all target chains, then Blockscout token lists, then one
`balanceOf` confirmation batch per chain. Open Blockscan only for addresses that still have gap chains, one page at a
time with pacing. Keep request concurrency at or below each provider's limit (Blockscout `x-ratelimit-limit`; CoinGecko
plan quota); back off on `429` as the provider references direct. Never run unbounded parallel requests.

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
