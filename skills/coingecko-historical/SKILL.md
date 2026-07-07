---
argument-hint: <coin-id> <date>
disable-model-invocation: true
model: opus
name: coingecko-historical
user-invocable: true
description: Open the CoinGecko historical-data page for a coin/date in Chromium via Chrome DevTools MCP.
---

# CoinGecko Historical

Open the CoinGecko historical-data page for a coin in Chromium via Chrome DevTools MCP, centered on a one-day window around a target date.

## Arguments

- **coin-id** (required): CoinGecko coin slug (e.g., `bitcoin`, `ethereum`, `solana`). This is the same ID used by the CoinGecko API and the `cg` CLI — *not* the ticker symbol. If the user provides only a symbol or name, resolve it first (e.g., `cg search <term> -o json` or `https://api.coingecko.com/api/v3/search?query=<term>`).
- **date** (required): Target date in `YYYY-MM-DD` format.

## Behavior

Compute `DATE-1` (the day before `date`) and `DATE+1` (the day after `date`), then open:

```
https://coingecko.com/en/coins/<coin-id>/historical_data?start=<DATE-1>&end=<DATE+1>
```

Call the `mcp__chrome_devtools.new_page` tool with:

```json
{
  "url": "https://coingecko.com/en/coins/<coin-id>/historical_data?start=<DATE-1>&end=<DATE+1>",
  "background": false
}
```

### Date arithmetic

Use BSD `date` (default on macOS) to compute the surrounding days:

```bash
start=$(date -j -v-1d -f "%Y-%m-%d" "$DATE" +%Y-%m-%d)
end=$(date -j -v+1d -f "%Y-%m-%d" "$DATE" +%Y-%m-%d)
```

## Example

User invokes: `/coingecko-historical bitcoin 2024-03-14`

Call `mcp__chrome_devtools.new_page` with:

```json
{
  "url": "https://coingecko.com/en/coins/bitcoin/historical_data?start=2024-03-13&end=2024-03-15",
  "background": false
}
```

## Notes

- Validate the date format before computing offsets; stop and ask the user if `date` is missing or malformed.
- Do not use macOS `open`; this skill specifically opens Chromium through Chrome DevTools MCP.
