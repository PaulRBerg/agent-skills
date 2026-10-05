---
coordination: exempt
name: cli-coingecko
user-invocable: false
description:
  "Use for CoinGecko/cg CLI crypto market data: prices, market cap, trending coins, top gainers/losers, coin search, or
  historical/OHLC data."
---

# CoinGecko CLI

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Use the installed CLI's machine-readable command catalog as the authority for supported market-data operations.

## Workflow

1. Verify `cg` exists. Inspect non-interactive auth/tier state:

   ```sh
   command -v cg
   cg status -o json
   cg commands -o json
   ```

   Do not run interactive `cg auth` or write config without the user's approval.

2. On an explicitly requested CLI update, run `cg update --dry-run`. Then run `cg update` within that authorization. Ask
   only if the preview reveals an action beyond the requested update. If automatic detection is wrong, pass `--method`
   with `homebrew`, `npm`, `go`, or `script`. Recheck `cg version` and `cg commands -o json`. Then, when maintaining
   this skill, record the verified version in `references/version.txt`.

3. Select the command, flags, enum values, output formats, endpoint, auth requirement, and `paid_only` status from
   `cg commands -o json`. Use `cg <command> --help` only when the catalog lacks a needed detail.

4. Resolve CoinGecko IDs with `cg search <term> -o json` when the user supplied a name or ambiguous symbol. Do not
   silently treat symbols as unique.

5. Preview unfamiliar or quota-sensitive requests with `--dry-run`. Execute parseable queries with `-o json`. Use
   `--export` only when the user requested a CSV artifact.

6. For a historical-data request, these CLI failures can trigger the browser fallback:

   - `cg` is unavailable.
   - `cg history` fails due to a CLI defect.
   - Its response is demonstrably malformed.

   If one prevents completion and a visual result can satisfy the request, immediately read
   [references/open-historical-page.md](references/open-historical-page.md). Under those conditions, use its browser
   fallback.

7. Present the requested result in the user's format. For ordinary human-readable output, use a compact table. Preserve
   enough precision for the asset's magnitude.

## Boundaries and Defaults

- Batch IDs in one request when the command supports it. On 429, respect the reported reset/backoff instead of retrying
  aggressively.
- Detect paid-only commands before execution. If the current tier cannot serve the request, report that limitation.
  Under that condition, offer a supported route.
- Do not use the browser fallback for authentication, tier, rate-limit, invalid-input, or ambiguity errors. It does not
  satisfy requests for JSON, CSV, or other machine-readable evidence.
- `cg` does not cover every CoinGecko endpoint. For unsupported contract-address prices, global stats, NFT detail,
  GeckoTerminal, or logo metadata, fetch the relevant current API documentation from
  <https://docs.coingecko.com/llms.txt>. For those requests, state that the CLI route is unavailable.
- Never expose API keys or send private wallet/account data to market-data endpoints.

Completion requires the resolved coin/command, successful JSON or requested export evidence, and explicit handling of
tier, ambiguity, or rate-limit constraints. A historical-page fallback instead requires a validated URL opened in
Chromium plus disclosure of the CLI failure that triggered it.

## User-Facing Output

For ordinary human output, lead with `### 🪙 <coin> (<id>)`. Show only the requested metrics in a compact table. Add one
source line with the endpoint plus timestamp, tier, or window when material.

For exports, use `### ✅ Exported <row count> rows`. Link the artifact. State the query/window without reproducing the
CSV.

When returning control, use `### ⚠️ Rate limited — retry after <time>`. Only while actively waiting, use
`### ⏳ Rate limited — retrying after <time>`. For a paid endpoint, use `### ⚠️ Paid endpoint — current tier: <tier>`
with one supported route. Return requested JSON/CSV, exact IDs, URLs, values, commands, and diagnostics undecorated.
