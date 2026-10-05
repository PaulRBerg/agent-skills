# Blockscan Balances

Use this reference for the current native or fungible-token balance of a public EVM wallet address on a named target
chain. For a wallet-wide or cross-chain check, use `references/workflows/debank-portfolio.md` first. Use Blockscan for
target chains DeBank lacks, when DeBank fails, or as an Etherscan-family cross-check. Historical balances and NFT
inventories remain on the existing provider routes. For per-chain and total USD value of one or more addresses, use
`references/workflows/address-usd-value.md`.

## Chromium Workflow

1. Validate the address. Then resolve any named chain against `references/generated/target-mainnets.json`. Do not send a
   non-target chain to Blockscan.
2. Open `https://blockscan.com/address/<addr>` with Chrome DevTools `new_page`, using Chromium rather than a direct
   Blockscan API or a general web search.
3. Wait for the address page and `Token Holdings` portfolio to replace any initial `Just a moment...` challenge. Then
   take a current accessibility snapshot.
4. Use the chain card for its current portfolio value. Use `#js-chain-table` for token amount, price, and value. Select
   the requested chain before reading the table. When the user requests complete holdings, paginate.
5. Capture the page's `Last updated` value when present. Treat displayed amounts and fiat values as Blockscan's current,
   formatted portfolio data rather than exact raw-unit balances.

Use Chrome DevTools `evaluate_script` when the accessibility snapshot does not expose a stable chain identifier. Match
support by exact chain ID with:

```text
input.address-transaction-chain[data-chainid="<chain-id>"]
```

Its `data-search` value includes the Blockscan chain symbol. Use that symbol to locate the corresponding
`.js-chain[data-chain="<symbol>"]` card. Do not infer support from a similar display name.

## Coverage and Scope

- For a named chain, the exact `data-chainid` match proves Blockscan currently offers that chain. A matching card with a
  zero token count or `$0.00` is a successful zero result, not a fallback condition.
- For a wallet-wide fallback, intersect Blockscan's `data-chainid` values with the target chains still uncovered.
  Blockscan covers only Etherscan-family chains. For target chains outside that intersection, query the existing API
  routes.
- Ignore Blockscan chains outside the target-mainnet list. Do not report the page-wide `NET WORTH` as a target-only
  total because it can include those chains.
- Keep successful Blockscan results when only some target chains or requested details require fallback.

## Fallbacks

Use `references/workflows/provider-routing.md` for a named chain and `references/workflows/address-sweeps.md` for a
wallet-wide check when:

- Chrome DevTools MCP or Chromium is unavailable
- navigation fails, a challenge or error persists, the page is rate limited, or the required portfolio DOM is absent
- the resolved target chain ID is absent from Blockscan's supported-chain inputs, or
- Blockscan cannot provide the exact/raw precision or current fungible-asset detail the user requested.

Route historical balances and NFT inventory requests directly to those existing references. Report which Blockscan
condition caused each fallback. Then retain the existing Etherscan, Blockscout, and RPC order within the selected
fallback reference.

## Output

Return the resolved target chain names and IDs, current native or fungible-token amounts requested, Blockscan's
displayed fiat values when relevant, freshness text, and the address-page URL. Identify fallback-derived facts by
provider. Separate them from Blockscan results.
