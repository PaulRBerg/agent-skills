---
argument-hint: <coin-id> <date>
disable-model-invocation: true
effort: low
model: sonnet
name: coingecko-open-page
user-invocable: true
description: Open the CoinGecko historical-data page for a coin/date in Chromium via Chrome DevTools MCP.
---

# CoinGecko Historical

Open a validated ±1-day window around the requested date in Chromium.

## Workflow

1. Require a CoinGecko coin ID and an ISO date (`YYYY-MM-DD`). Resolve a supplied name or symbol with
   `cg search <term> -o json` before continuing.

2. Build the URL with the portable helper:

   ```sh
   uv run <skill-dir>/scripts/build-url.py <coin-id> <date>
   ```

   Invalid IDs or calendar dates exit nonzero without opening a page.

3. Pass the returned URL to `mcp__chrome-devtools__new_page` with `background: false`.

4. Completion is `### 🌐 CoinGecko history opened — <coin-id> · <date> (±1 day)` followed by the linked page URL. Do not
   use the macOS `open` command. Keep the helper's bare-URL stdout and validation errors undecorated.
