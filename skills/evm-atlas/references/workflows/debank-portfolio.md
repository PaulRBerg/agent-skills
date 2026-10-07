# DeBank Portfolio

Use this reference for the wallet-wide current holdings of a public EVM address across target chains: native and
fungible-token balances plus DeFi protocol positions (liquidity, lending, staking, vesting). DeBank covers more target
chains than Blockscan, including non-Etherscan chains. For one named chain, use
`references/workflows/blockscan-balances.md`. Historical balances, NFT inventories, and transaction history stay on
`references/workflows/provider-routing.md`.

## Global Queue

DeBank's WAF limits the whole browser. Every agent on this host shares one allowance. One agent's burst blocks the rest.
Hold a lease from `scripts/debank-gate.py` for all debank.com work: navigating, pasting the collector, `start`, and DOM
reads. The gate grants leases one at a time in FIFO order. State lives under
`${XDG_STATE_HOME:-~/.local/state}/evm-atlas/debank-gate`.

<!-- prettier-ignore -->
```sh
uv run --no-project scripts/debank-gate.py acquire --label '<skill>: <purpose>' --profiles <n>
```

- Each command prints one JSON line. On exit 0 with `"status": "granted"`, proceed. Under that granted status, keep the
  `ticket`. Exit 3 with `"status": "queued"` (after `--wait`, default 240 s) reports `position`, `holder`, and
  `cooldownUntil`. Under that queued status, rerun `acquire` with `--ticket <ticket>` to keep the place. The gate drops
  a ticket if you do not poll it for 120 s.

  In Claude Code, run `acquire --wait 3600` in the background. When it exits, continue.

- `--profiles` is the number of addresses the lease covers, at most 25. Bulk work takes one lease per batch, so other
  agents' single-address checks get a turn between batches.
- A lease expires after `--ttl` (600 s). Run `renew --ticket <ticket>` before then for longer work. Exit 4 (`"lost"`)
  means it expired and another agent may hold the gate. Under that exit, stop touching DeBank. Then acquire again.
- When done, or on any failure, close the owned DeBank page. Then run `release --ticket <ticket>` under that condition.
- On a WAF block, run `block --ticket <ticket>`. It releases the lease and pauses the queue for every agent for 15
  minutes.
- `status` shows the holder, cooldown, and queue. Never touch debank.com without a lease, even when the queue is long. A
  caller that cannot wait uses Fallbacks and reports the queue wait as the cause.

## Chromium Workflow

1. With a lease held, validate each address (20-byte hex). Then open an owned page on
   `https://debank.com/profile/<addr>` with Chrome DevTools `new_page` under that lease. When the cookie dialog appears,
   choose `Reject`.
2. Never call `api.debank.com` balance endpoints yourself (DeBank's Cloud OpenAPI is paid). The profile page's calls are
   `fetch` GETs signed by the app (`x-api-*` headers): unsigned calls, even from inside the page, return
   `429 Request too fast`, and `credentials: 'include'` fails CORS. The collector captures the app's own responses
   instead.
3. **Token discovery**, for one address or many, uses the collector. Read `scripts/debank-collect.js`. Pass its whole
   contents verbatim as the `evaluate_script` `function`. The file intentionally has no trailing `;`, so it stays a
   pasteable arrow function. It returns `{ installed: true, reused: false }` (`reused: true` when the page already has
   it), installs `window.__debankCollect`, and wraps `window.fetch` to record the app's `used_chains` and `balance_list`
   responses.

   Per address it routes the page to the profile and succeeds once `used_chains` and a `balance_list` for every listed
   chain return 200 with `error_code` 0. DeBank lists the chains an address used, not the chains where it holds tokens,
   so an `ok` record with chains can hold zero tokens. The API includes small balances the UI folds, so no `Show all`
   click is needed. Start the run with a second `evaluate_script` call:

<!-- prettier-ignore -->
```js
async () => window.__debankCollect.start(["<addr>"])
```

`start` lowercases and de-duplicates the addresses. It throws for an invalid address, an active run, or a failed
`chain/list` call. Otherwise, it returns `{ queued }` without waiting for the run. Awaiting it surfaces those errors. An
optional second argument sets `{ timeoutMs: 30000, maxAttempts: 3, cooldownMs: 20000, haltAfter: 3 }` (the defaults).

4. Poll `window.__debankCollect.status()` with short `evaluate_script` calls until `running` is `false`. It returns
   `{ running, total, done, ok, failed, pending, rateLimited, blocked, startedAt, elapsedMs }`, with `startedAt` an ISO
   string. A profile takes about 2.5 s (1.2-4.6 s). A `429`, error, or timeout pauses the page for `cooldownMs` and
   requeues the address until `maxAttempts`, after which its record is `failed`. After `haltAfter` consecutive failed
   attempts that saw a `429`, the run stops with `blocked: true` and fails every queued address. See Rate Limits and WAF
   Blocks.
5. Save `window.__debankCollect.results()` by calling `evaluate_script` with
   `function: () => window.__debankCollect.results()` and a `filePath`. The path must be inside the MCP workspace roots
   (a git-ignored project directory). The MCP refuses other paths. `results()` returns the latest run's completed
   records in input order. A new `start` clears them. Save them before starting again.

   Each record has `address`, `status` (`ok` or `failed`), `attempts`, `error` (when `failed`), `chains` (the
   `used_chains` slugs), `observedAt` (ISO string), and `tokens`, each
   `{ chainId, chain, contract, symbol, decimals, rawAmount, amount, price }`. `contract` is the lowercased ERC-20
   address or `"native"`, `chainId` is the chain's numeric `network_id` (`null` when `chain/list` has none for the
   slug), and `rawAmount` is an exact decimal string in raw units. A `failed` record carries `chains: []` and
   `tokens: []`. Those arrays mean nothing. Only `status` counts.

6. Read the `All Chain` summary and the DeFi protocol sections from the DOM only, on that address's own profile (after a
   run, the page shows the last address collected). Wait for `Data updated`. When present, click `Unfold <n> chains`.
   Then read per-chain USD values from the summary. DeFi sections follow the wallet table, each with a protocol name,
   USD value, and position type. When the user requests positions, read them from a fresh snapshot.

- `Data updated <age>` is the DeFi project-snapshot time, not wallet-token freshness. `20727 days ago` (the Unix epoch)
  means `portfolio/project_list` failed to load and says nothing about tokens. Token observation time is each record's
  `observedAt`.
- The DOM wallet table folds small balances behind a toggle reading "Tokens with small balances are not displayed. Show
  all". It appears only when the wallet has at least 15 tokens and at least 4 are under min(0.1% of wallet USD, $1000).
  Only when you must read the table itself, click `Show all`. The clickable is a `<span>` with an `<svg>` child. Thus, a
  leaf-element text matcher never finds it. Scraping the folded table misses those tokens.

## Many Addresses

One `start` call takes any number of addresses and routes a single owned page through their profiles. DeBank refuses to
render in an iframe. Thus, a loop never needs one page per address.

- Use one owned page, opened with `new_page` (it reports visible, so timers are not throttled). Each profile costs about
  `10 + <chains>` requests, and bulk runs were blocked after every 30-55 profiles.
- Split the addresses into batches of at most 25. Per batch, run `acquire --profiles <batch size>`. Then `start` the
  batch. Poll it. Save `results()`. Then `release`.

  Acquire again for the next batch.

- The loop runs without being awaited, so no DevTools protocol timeout applies. Poll `status()` with short calls.
- A new `start` throws while a run is active and clears the previous results once it begins. To abort an active run,
  reload the page. Then paste the script again.
- A `failed` record is a coverage gap, never an empty wallet. Handle it per Fallbacks. An `ok` record with no tokens is
  DeBank's indexed zero.
- Never read the page UI to decide that a wallet is empty. During rate limiting, a `429` on `used_chains` makes the page
  show "No assets yet" with a "Request too fast" toast, `429`s on the balance endpoints remove the wallet table, and DOM
  rows can linger from the previous profile after a route change.

## Rate Limits and WAF Blocks

`Request too fast` (HTTP `429`, body `error_code: 429`, or the page toast) is DeBank's WAF rejecting the request. It is
a coverage gap, never evidence about the wallet, and never a reason to call `api.debank.com` another way. Classify it by
how it arrived:

- **Unsigned call.** A direct `fetch`, `curl`, or WebFetch of `api.debank.com` balance endpoints always gets `429`, so
  retrying it never works. Switch to the collector. Never present the `429` as DeBank being down.
- **Burst.** `rateLimited > 0` while records still finish `ok` means the collector is absorbing short bursts. Take no
  action.
- **Block.** `status().blocked` is `true`, `start` throws `chain/list failed: HTTP 429`, or the toast persists. DeBank
  is rejecting the whole browser. Blocks lasted 5-13 minutes. Do not reload, re-paste, open more pages, or restart in a
  loop. Each request prolongs the block.

  Close the page. Then run `debank-gate.py block --ticket <ticket>` so every queued agent waits out the cooldown instead
  of extending it. Then acquire again for the `failed` addresses only. The queue grants the lease after the cooldown. If
  that retry is also blocked, take the remaining addresses through Fallbacks.

Before reporting a block, confirm it in Chromium through the `status()` output, record `error` strings, or `429`
responses in `list_network_requests`. Report it as `DeBank WAF rate-limit block ("Request too fast")` with the
verification method, the retry made, and the affected addresses.

## Chain Mapping

- DeBank names chains by slug (`eth`, `scrl`, `xdai`, `era`). The collector maps slugs to chain IDs through the
  `network_id` field of the keyless `https://api.debank.com/chain/list`. Match target chains by exact `chainId`, never
  by slug or display name.
- A token whose `chainId` is `null` (`chain/list` has no numeric `network_id` for its slug, as for non-EVM balances such
  as Hyperliquid spot and perps) is not a target chain. Report it with DeFi positions, never as a target chain.

## Coverage and Scope

- Derive coverage at runtime: target chains whose ID appears in `chain/list` are DeBank-supported. `used_chains` bounds
  the chains the collector queries, so a supported target chain absent from a record is DeBank's indexed zero, not an
  RPC-confirmed zero.
- Ignore non-target chains. Sum target-chain tokens for any target-only total. Never report the page-wide total as one.
- DeBank drops spam server-side (every returned token had `is_scam` and `is_suspicious` false), so it never reports spam
  tokens, and absence from a record does not show that a token is not held.
- `rawAmount` is exact in raw units (with `decimals`). `amount` and `price` are DeBank's. They are still indexer data,
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

Return the profile URL and the collector `observedAt` per address. Only when DeFi positions were read, add the
`Data updated` age. Label that age as the DeFi snapshot time. Then return one row per non-empty target chain (name, ID,
native and token amounts, DeBank USD), the target-only sum, and DeFi positions as a separate labeled list (protocol,
chain, position type, USD).

For many addresses, also give the saved results path and the `ok` and `failed` counts. Add counts of excluded non-target
chains and coverage gaps (including each `failed` address) with their cause. Separate fallback-derived facts by
provider.
