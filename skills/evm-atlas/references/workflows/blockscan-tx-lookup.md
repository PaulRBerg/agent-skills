# Blockscan Transaction Lookup

Use this reference for a specific transaction hash: resolving which target chain it belongs to, or its status, parties,
value, fee, and timestamp. Prefer Blockscan in Chromium first, whether the chain is already known or unstated — a single
lookup beats probing each candidate chain's provider in turn. Full raw receipts, logs, internal transactions, or decoded
input on an already-resolved chain remain on the existing provider routes.

## Chromium Workflow

1. Validate the hash format (`0x` plus 64 hex chars). If the user also names a chain, resolve it against
   `references/generated/target-mainnets.json` first; still confirm it against the Blockscan result in step 4.
2. Open `https://blockscan.com/tx/<hash>` with `mcp__chrome_devtools.new_page`, using Chromium rather than a direct API
   or a general web search.
3. Wait for the transaction heading to replace any initial `Just a moment...` challenge, then take a current
   accessibility snapshot.
4. Identify the chain from the `<h1>` heading (`<Chain> Transaction Details`) and the `View on <Explorer>` link's
   `href`. Cross-check that `href`'s host against `explorerUrl` in `references/generated/target-mainnets.json` for the
   authoritative chain ID and name — the heading text and chain-logo image alone are display hints, not proof.
5. If the resolved chain has no matching target-mainnet row, stop: this is out of scope per `SKILL.md`. Do not serve
   data for it and do not fall back to another provider to work around scope.
6. Read status (success/failed/pending), timestamp, `from`/`to`, value, and fee directly from the page for the
   human-readable answer.

## Coverage and Scope

- A rendered `<Chain> Transaction Details` page proves Blockscan currently indexes that hash on that chain.
- `Error 404` (`Seems we got lost!`) is not proof the hash doesn't exist. Blockscan aggregates the Etherscan-family
  (`*scan`) explorers only, so a real transaction on a chain Blockscan doesn't cover, or one not yet indexed, also 404s.
  Never report "transaction not found" from a 404 alone.
- Blockscan's summary fields are formatted/display data, not a substitute for an exact raw receipt.

## Fallbacks

Use `references/workflows/provider-routing.md` for the resolved (or user-named) chain when:

- Chrome DevTools MCP or Chromium is unavailable;
- navigation fails, a challenge or error persists, or the page is rate limited;
- the page 404s and the user named a candidate chain — query that chain's provider directly; or
- the request needs the full raw receipt, logs, internal transactions, decoded input, or another detail beyond
  Blockscan's summary.

If the hash's chain is genuinely unknown, no candidate chain is named, and Blockscan 404s, say so explicitly. Do not
default to sweeping every target chain's provider one by one — that is the per-provider cost this workflow exists to
avoid. Ask the user for a candidate chain, or confirm before running that broader sweep.

## Output

Return the resolved target chain name and ID, transaction status, `from`/`to`, value, fee, timestamp, the transaction
hash, the Blockscan URL, and the native-explorer URL surfaced by `View on <Explorer>`. Identify fallback-derived facts
by provider and separate them from Blockscan results.
