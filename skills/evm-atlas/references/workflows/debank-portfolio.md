# DeBank Portfolio

Use this reference for the wallet-wide current holdings of a public EVM address across target chains: native and
fungible-token balances plus DeFi protocol positions (liquidity, lending, staking, vesting). DeBank covers more target
chains than Blockscan, including non-Etherscan chains. For one named chain, use
`references/workflows/blockscan-balances.md`. Historical balances, NFT inventories, and transaction history stay on
`references/workflows/provider-routing.md`.

## Chromium Workflow

1. Validate the address, then open `https://debank.com/profile/<addr>` with Chrome DevTools `new_page`. Do not call
   `api.debank.com` balance endpoints directly: outside the page they return `429 Request too fast`, and DeBank's Cloud
   OpenAPI is paid.
2. When the cookie dialog appears, choose `Reject`. Wait for `Data updated`, then record its age (for example
   `3 mins ago`): DeBank renders cached balances first.
3. Read per-chain USD values from the `All Chain` summary; click `Unfold <n> chains` when present.
4. The wallet table hides small balances. When completeness matters (token discovery, dust, or drained checks), click
   `Show all` below it before extracting.
5. Extract wallet rows with `evaluate_script`:

<!-- prettier-ignore -->
```js
async () => {
  const { data } = await (await fetch("https://api.debank.com/chain/list")).json();
  const chainIds = Object.fromEntries(data.chains.map((c) => [c.id, c.network_id]));
  const header = [...document.querySelectorAll(".db-table-header")].find((h) =>
    /^Token\s*Price\s*Amount\s*USD Value$/.test(h.innerText.trim()),
  );
  if (!header) return { error: "wallet table not found" };
  const table = header.parentElement;
  return [...table.querySelectorAll('a[href*="/token/"]')].map((a) => {
    let row = a;
    while (row.parentElement !== table && row.innerText.split("\n").filter((s) => s.trim()).length < 4) {
      row = row.parentElement;
    }
    const [, , slug, token] = new URL(a.href).pathname.split("/");
    const [symbol, price, amount, usd] = row.innerText
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
    const contract = /^0x[0-9a-f]{40}$/i.test(token) ? token : "native";
    return { chainId: chainIds[slug] ?? null, slug, contract, symbol, price, amount, usd };
  });
}
```

6. DeFi protocol sections follow the wallet table, each with a protocol name, USD value, and position type. Read them
   from a fresh snapshot when the user requests positions.

## Chain Mapping

- DeBank names chains by slug (`eth`, `scrl`, `xdai`, `era`). Map slugs to chain IDs only through the `network_id` field
  of the keyless `https://api.debank.com/chain/list`, then match target rows by exact chain ID, never by display name.
- A token link `/token/<slug>/<0x-address>` carries the ERC-20 contract; a non-hex token segment (`/token/eth/eth`,
  `/token/matic/matic`) is the chain's native asset.
- Slug `undefined` (for example Hyperliquid spot and perps balances) is off-EVM: report it with DeFi positions, never as
  a target chain.

## Coverage and Scope

- Derive coverage at runtime: target chains whose ID appears in `chain/list` are DeBank-supported. A supported target
  chain absent from the profile is DeBank's indexed zero, not an RPC-confirmed zero.
- Ignore non-target chains. Sum target-chain rows for any target-only total; never report the page-wide total as one.
- Treat amounts, prices, and USD values as DeBank's formatted display data, not raw-unit balances. Amounts are rounded,
  and subscript notation compresses leading zeros (`0.0₅3` = `0.000003`).
- DeBank's spam filtering and pricing are its own. Apply the Pricing Hygiene in `address-usd-value.md` before using
  DeBank values in totals.
- Keep DeFi positions separate from wallet balances.

## Fallbacks

- Target chains missing from `chain/list`: use `blockscan-balances.md` when Blockscan lists the chain ID, otherwise
  `provider-routing.md`.
- Navigation fails, an error or challenge persists, the page rate-limits, or the wallet table is absent: use
  `blockscan-balances.md`, then `address-sweeps.md` for remaining target chains.
- Chrome DevTools MCP or Chromium is unavailable: use `address-sweeps.md` (or the API passes in `address-usd-value.md`).
- Exact or raw precision required: confirm with RPC `balanceOf` and `eth_getBalance` as `address-usd-value.md` does.

Report which condition caused each fallback.

## Output

Return the profile URL, `Data updated` age, one row per non-empty target chain (name, ID, native and token amounts,
DeBank USD), the target-only sum, and DeFi positions as a separate labeled list (protocol, chain, position type, USD).
Add counts of excluded non-target chains and coverage gaps with their cause. Separate fallback-derived facts by
provider.
