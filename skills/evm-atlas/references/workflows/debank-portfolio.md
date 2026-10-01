# DeBank Portfolio

Use this reference for the wallet-wide current holdings of a public EVM address across target chains: native and
fungible-token balances plus DeFi protocol positions (liquidity, lending, staking, vesting). DeBank covers more target
chains than Blockscan, including non-Etherscan chains. For one named chain, use
`references/workflows/blockscan-balances.md`. Historical balances, NFT inventories, and transaction history stay on
`references/workflows/provider-routing.md`.

## Chromium Workflow

1. Validate each address (20-byte hex), then open an owned page on `https://debank.com/profile/<addr>` with Chrome
   DevTools `new_page`. When the cookie dialog appears, choose `Reject`.
2. Never call `api.debank.com` balance endpoints yourself (DeBank's Cloud OpenAPI is paid). The profile page's calls are
   `fetch` GETs signed by the app (`x-api-*` headers): unsigned calls, even from inside the page, return
   `429 Request too fast`, and `credentials: 'include'` fails CORS. The collector captures the app's own responses
   instead.
3. **Token discovery**, for one address or many, uses the collector. Read `scripts/debank-collect.js` and pass its whole
   contents verbatim as the `evaluate_script` `function`; the file has no trailing `;` on purpose, so it stays a
   pasteable arrow function. It returns `{ installed: true, reused: false }` (`reused: true` when the page already has
   it), installs `window.__debankCollect`, and wraps `window.fetch` to record the app's `used_chains` and `balance_list`
   responses. Per address it routes the page to the profile and succeeds once `used_chains` and a `balance_list` for
   every listed chain return 200 with `error_code` 0. DeBank lists the chains an address used, not the chains where it
   holds tokens, so an `ok` record with chains can hold zero tokens. The API includes small balances the UI folds, so no
   `Show all` click is needed. Start the run with a second `evaluate_script` call:

<!-- prettier-ignore -->
```js
async () => window.__debankCollect.start(["<addr>"])
```

`start` lowercases and de-duplicates the addresses, throws for an invalid address, an active run, or a failed
`chain/list` call, and otherwise returns `{ queued }` without waiting for the run, so awaiting it surfaces those errors.
An optional second argument sets `{ timeoutMs: 30000, maxAttempts: 3, cooldownMs: 20000, haltAfter: 3 }` (the defaults).

4. Poll `window.__debankCollect.status()` with short `evaluate_script` calls until `running` is `false`. It returns
   `{ running, total, done, ok, failed, pending, rateLimited, blocked, startedAt, elapsedMs }`, with `startedAt` an ISO
   string. A profile takes about 2.5 s (1.2-4.6 s). A `429`, error, or timeout pauses the page for `cooldownMs` and
   requeues the address until `maxAttempts`, after which its record is `failed`. After `haltAfter` consecutive failed
   attempts that saw a `429`, the run stops with `blocked: true` and fails every queued address; see Rate Limits and WAF
   Blocks.
5. Save `window.__debankCollect.results()` by calling `evaluate_script` with
   `function: () => window.__debankCollect.results()` and a `filePath`. The path must be inside the MCP workspace roots
   (a git-ignored project directory); other paths are refused. `results()` returns the latest run's completed records in
   input order, and a new `start` clears them, so save first. Each record has `address`, `status` (`ok` or `failed`),
   `attempts`, `error` (when `failed`), `chains` (the `used_chains` slugs), `observedAt` (ISO string), and `tokens`,
   each `{ chainId, chain, contract, symbol, decimals, rawAmount, amount, price }`. `contract` is the lowercased ERC-20
   address or `"native"`, `chainId` is the chain's numeric `network_id` (`null` when `chain/list` has none for the
   slug), and `rawAmount` is an exact decimal string in raw units. A `failed` record carries `chains: []` and
   `tokens: []`; those arrays mean nothing, and only `status` counts.
6. Read the `All Chain` summary and the DeFi protocol sections from the DOM only, on that address's own profile (after a
   run, the page shows the last address collected). Wait for `Data updated`; click `Unfold <n> chains` when present,
   then read per-chain USD values from the summary. DeFi sections follow the wallet table, each with a protocol name,
   USD value, and position type; read them from a fresh snapshot when the user requests positions.

- `Data updated <age>` is the DeFi project-snapshot time, not wallet-token freshness. `20727 days ago` (the Unix epoch)
  means `portfolio/project_list` failed to load and says nothing about tokens. Token observation time is each record's
  `observedAt`.
- The DOM wallet table folds small balances behind a toggle reading "Tokens with small balances are not displayed. Show
  all". It appears only when the wallet has at least 15 tokens and at least 4 are under min(0.1% of wallet USD, $1000).
  Click `Show all` only when you must read the table itself; the clickable is a `<span>` with an `<svg>` child, so a
  leaf-element text matcher never finds it. Scraping the folded table misses those tokens.

## Many Addresses

One `start` call takes any number of addresses and routes a single owned page through their profiles (DeBank refuses to
render in an iframe), so a loop never needs one page per address.

- Use at most two owned pages, opened with `new_page` (they report visible, so timers are not throttled). Two concurrent
  pages ran without errors; three started at once triggered a burst `429`, and each profile costs about `10 + <chains>`
  requests. Split the addresses round-robin into two slices, paste the script and `start` the first slice, then start
  the second page a few seconds later.
- The loop runs without being awaited, so no DevTools protocol timeout applies; poll `status()` with short calls and
  save `results()` per page.
- A new `start` throws while a run is active and clears the previous results once it begins. To abort an active run,
  reload the page and paste the script again.
- A `failed` record is a coverage gap, never an empty wallet; handle it per Fallbacks. An `ok` record with no tokens is
  DeBank's indexed zero.
- Never read the page UI to decide that a wallet is empty. During rate limiting, a `429` on `used_chains` makes the page
  show "No assets yet" with a "Request too fast" toast, `429`s on the balance endpoints remove the wallet table, and DOM
  rows can linger from the previous profile after a route change.

## Rate Limits and WAF Blocks

`Request too fast` (HTTP `429`, body `error_code: 429`, or the page toast) is DeBank's WAF rejecting the request. It is
a coverage gap, never evidence about the wallet, and never a reason to call `api.debank.com` another way. Classify it by
how it arrived:

- **Unsigned call.** A direct `fetch`, `curl`, or WebFetch of `api.debank.com` balance endpoints always gets `429`, so
  retrying it never works. Switch to the collector; never present the `429` as DeBank being down.
- **Burst.** `rateLimited > 0` while records still finish `ok` is the collector absorbing short bursts; no action.
- **Block.** `status().blocked` is `true`, or `start` throws `chain/list failed: HTTP 429`. DeBank is rejecting this
  client. Do not reload, re-paste, open more pages, or restart in a loop: each request prolongs it. Close any second
  page, wait at least two minutes, then run one retry on a single page with only the `failed` addresses and
  `{ cooldownMs: 60000 }`. If that run is also `blocked` or `start` still throws, stop using DeBank for this task and
  take the remaining addresses through Fallbacks.

Confirm a block in Chromium before reporting it: the `status()` output, the record `error` strings, or the `429`
responses in `list_network_requests`. Report it as `DeBank WAF rate-limit block ("Request too fast")` with the
verification method, the retry made, and the affected addresses.

## Chain Mapping

- DeBank names chains by slug (`eth`, `scrl`, `xdai`, `era`). The collector maps slugs to chain IDs through the
  `network_id` field of the keyless `https://api.debank.com/chain/list`. Match target chains by exact `chainId`, never
  by slug or display name.
- A token whose `chainId` is `null` (`chain/list` has no numeric `network_id` for its slug, as for non-EVM balances such
  as Hyperliquid spot and perps) is not a target chain: report it with DeFi positions, never as a target chain.

## Coverage and Scope

- Derive coverage at runtime: target chains whose ID appears in `chain/list` are DeBank-supported. `used_chains` bounds
  the chains the collector queries, so a supported target chain absent from a record is DeBank's indexed zero, not an
  RPC-confirmed zero.
- Ignore non-target chains. Sum target-chain tokens for any target-only total; never report the page-wide total as one.
- DeBank drops spam server-side (every returned token had `is_scam` and `is_suspicious` false), so it never reports spam
  tokens, and absence from a record does not show that a token is not held.
- `rawAmount` is exact in raw units (with `decimals`); `amount` and `price` are DeBank's. They are still indexer data,
  and `address-usd-value.md` confirms amounts by RPC before using them. Apply the Pricing Hygiene there before using
  DeBank prices in totals.
- Keep DeFi positions separate from wallet balances.

## Fallbacks

- Target chains missing from `chain/list`: use `blockscan-balances.md` when Blockscan lists the chain ID, otherwise
  `provider-routing.md`.
- Navigation fails, an error or challenge persists, an address's record is `failed`, or a WAF block survives the one
  retry in Rate Limits and WAF Blocks: use `blockscan-balances.md`, then `address-sweeps.md` for remaining target
  chains.
- Chrome DevTools MCP or Chromium is unavailable: use `address-sweeps.md` (or the API passes in `address-usd-value.md`).
- On-chain precision required: confirm with RPC `balanceOf` and `eth_getBalance` as `address-usd-value.md` does.

Report which condition caused each fallback.

## Output

Return the profile URL and the collector `observedAt` per address; add the `Data updated` age only when DeFi positions
were read, labeled as the DeFi snapshot time. Then one row per non-empty target chain (name, ID, native and token
amounts, DeBank USD), the target-only sum, and DeFi positions as a separate labeled list (protocol, chain, position
type, USD). For many addresses, also give the saved results path and the `ok` and `failed` counts. Add counts of
excluded non-target chains and coverage gaps (including each `failed` address) with their cause. Separate
fallback-derived facts by provider.
