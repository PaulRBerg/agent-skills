# Blockscan Balances

Use this reference for the current native or fungible-token balance of an existing public EVM wallet address. Prefer
Blockscan in Chromium for both a named target chain and a wallet-wide portfolio check. Historical balances and NFT
inventories remain on the existing provider routes.

## Chromium Workflow

1. Validate the address, then resolve any named chain against `references/generated/target-mainnets.json`. Do not send a
   non-target chain to Blockscan.
2. Open `https://blockscan.com/address/<addr>` with `mcp__chrome_devtools.new_page`, using Chromium rather than a direct
   Blockscan API or a general web search.
3. Wait for the address page and `Token Holdings` portfolio to replace any initial `Just a moment...` challenge, then
   take a current accessibility snapshot.
4. Use the chain card for its current portfolio value and `#js-chain-table` for token amount, price, and value. Select
   the requested chain before reading the table and paginate when the user requests complete holdings.
5. Capture the page's `Last updated` value when present. Treat displayed amounts and fiat values as Blockscan's current,
   formatted portfolio data rather than exact raw-unit balances.

Use `mcp__chrome_devtools.evaluate_script` when the accessibility snapshot does not expose a stable chain identifier.
Match support by exact chain ID with:

```text
input.address-transaction-chain[data-chainid="<chain-id>"]
```

Its `data-search` value includes the Blockscan chain symbol. Use that symbol to locate the corresponding
`.js-chain[data-chain="<symbol>"]` card; do not infer support from a similar display name.

## Coverage and Scope

- For a named chain, the exact `data-chainid` match proves Blockscan currently offers that chain. A matching card with a
  zero token count or `$0.00` is a successful zero result, not a fallback condition.
- For a wallet-wide check, intersect Blockscan's `data-chainid` values with `references/generated/target-mainnets.json`.
  Query the existing API routes for target chains outside that intersection.
- Ignore Blockscan chains outside the target-mainnet list. Do not report the page-wide `NET WORTH` as a target-only
  total because it can include those chains.
- Keep successful Blockscan results when only some target chains or requested details require fallback.

## Fallbacks

Use `references/workflows/provider-routing.md` for a named chain and `references/workflows/address-sweeps.md` for a
wallet-wide check when:

- Chrome DevTools MCP or Chromium is unavailable;
- navigation fails, a challenge or error persists, the page is rate limited, or the required portfolio DOM is absent;
- the resolved target chain ID is absent from Blockscan's supported-chain inputs; or
- Blockscan cannot provide the exact/raw precision or current fungible-asset detail the user requested.

Route historical balances and NFT inventory requests directly to those existing references. Report which Blockscan
condition caused each fallback, then retain the existing Etherscan, Blockscout, and RPC order within the selected
fallback reference.

## Output

Return the resolved target chain names and IDs, current native or fungible-token amounts requested, Blockscan's
displayed fiat values when relevant, freshness text, and the address-page URL. Identify fallback-derived facts by
provider and separate them from Blockscan results.
