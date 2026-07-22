---
disable-model-invocation: false
name: coingecko-cli
user-invocable: false
description:
  "Use for CoinGecko/cg CLI crypto market data: prices, market cap, trending coins, top gainers/losers, coin search, or
  historical/OHLC data."
---

# CoinGecko CLI

Use the installed CLI's machine-readable command catalog as the source of truth for supported market-data operations.

## Workflow

1. Verify `cg` exists and inspect non-interactive auth/tier state:

   ```sh
   command -v cg
   cg status -o json
   cg commands -o json
   ```

   Do not run interactive `cg auth` or write config without the user's approval.

2. Select the command, flags, enum values, output formats, endpoint, auth requirement, and `paid_only` status from
   `cg commands -o json`. Use `cg <command> --help` only when the catalog lacks a needed detail.

3. Resolve CoinGecko IDs with `cg search <term> -o json` when the user supplied a name or ambiguous symbol. Do not
   silently treat symbols as unique.

4. Preview unfamiliar or quota-sensitive requests with `--dry-run`. Execute parseable queries with `-o json`; use
   `--export` only when the user requested a CSV artifact.

5. Present the requested result in the user's format. For ordinary human-readable output, use a compact table and
   preserve enough precision for the asset's magnitude.

## Boundaries and Defaults

- Batch IDs in one request when the command supports it. On 429, respect the reported reset/backoff rather than retrying
  aggressively.
- Detect paid-only commands before execution. If the current tier cannot serve the request, say so and offer a supported
  route.
- `cg` does not cover every CoinGecko endpoint. For unsupported contract-address prices, global stats, NFT detail,
  GeckoTerminal, or logo metadata, fetch the relevant current API documentation from
  <https://docs.coingecko.com/llms.txt> and state that the CLI route is unavailable.
- Never expose API keys or send private wallet/account data to market-data endpoints.

Completion requires the resolved coin/command, successful JSON or requested export evidence, and explicit handling of
tier, ambiguity, or rate-limit constraints.

## User-Facing Output

For ordinary human output, lead with `### 🪙 <coin> (<id>)`, show only the requested metrics in a compact table, and add
one source line with the endpoint plus timestamp, tier, or window when material. For exports, use
`### ✅ Exported <row count> rows`, link the artifact, and state the query/window without reproducing the CSV. Use
`### ⚠️ Rate limited — retry after <time>` when returning control, `### ⏳ Rate limited — retrying after <time>` only
while actively waiting, or `### ⚠️ Paid endpoint — current tier: <tier>` with one supported route. Return requested
JSON/CSV, exact IDs, URLs, values, commands, and diagnostics undecorated.
