# Blockscan Transaction Lookup

Use this reference primarily to resolve an exact transaction hash whose chain is unknown, and return Blockscan's
formatted status, parties, value, fee, and timestamp when available. A single unknown-chain lookup avoids probing each
candidate chain's provider in turn. Blockscan's formatted summary is not raw transaction evidence.

## Route by Chain Context

1. Validate the hash format (`0x` plus 64 hex chars).
2. If the user supplied a chain, resolve it against `references/generated/target-mainnets.json`. Unless the user
   explicitly requested Blockscan as the evidence source, do not navigate to Blockscan; immediately use
   `references/workflows/provider-routing.md` for the transaction facts.
3. For a known or suspected OP Mainnet transaction from before the final regenesis, apply
   `references/explorers/optimism-pre-regenesis.md` before requiring evidence from a current provider.
4. Continue to the Chromium workflow only when the chain remains unknown or the user explicitly requested Blockscan.

## Chromium Workflow

1. Open `https://blockscan.com/tx/<hash>` with `mcp__chrome_devtools.new_page`, using Chromium rather than a direct API
   or a general web search.
2. Wait for the transaction heading to replace any initial `Just a moment...` challenge, then take a current
   accessibility snapshot.
3. Identify the chain from the `<h1>` heading (`<Chain> Transaction Details`) and the `View on <Explorer>` link's
   `href`. Cross-check that `href`'s host against `explorerUrl` in `references/generated/target-mainnets.json` for the
   authoritative chain ID and name — the heading text and chain-logo image alone are display hints, not proof.
4. If the resolved chain has no matching target-mainnet row, stop: this is out of scope per `SKILL.md`. Do not serve
   data for it and do not fall back to another provider to work around scope.
5. Read status (success/failed/pending), timestamp, `from`/`to`, value, and fee directly from the page for the
   human-readable answer.

## Coverage and Scope

- A rendered `<Chain> Transaction Details` page proves Blockscan currently indexes that hash on that chain.
- `Error 404` (`Seems we got lost!`) is not proof the hash doesn't exist. Blockscan aggregates the Etherscan-family
  (`*scan`) explorers only, so a real transaction on a chain Blockscan doesn't cover, or one not yet indexed, also 404s.
  Never report "transaction not found" from a 404 alone.
- Blockscan's summary fields are formatted/display data, not a substitute for an exact raw receipt.

## Fallbacks

After Blockscan resolves the chain, use `references/workflows/provider-routing.md` when:

- the request needs the full raw receipt, logs, internal transactions, decoded input, or another detail beyond
  Blockscan's summary.

If the chain is unknown and Blockscan 404s or is unavailable, report the chain as unresolved and stop. Do not infer that
the transaction does not exist or sweep every target chain's provider. Ask for a candidate chain or approval for that
broader provider sweep.

If the user explicitly requested Blockscan for a known chain, attempt it and report its coverage result, including when
Chromium is unavailable, navigation fails, a challenge persists, the page is rate limited, or it 404s. Do not silently
substitute another evidence source; identify any separately requested fallback facts by provider.

## Output

Always return the transaction hash, resolved target chain name and ID when known, selected provider route, requested
facts, and evidence source. Include the Blockscan URL and native-explorer URL surfaced by `View on <Explorer>` only when
Blockscan was queried. Otherwise, report the provider route and its evidence without implying Blockscan confirmation.
