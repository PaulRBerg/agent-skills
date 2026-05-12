---
argument-hint: <coin-id> <date>
disable-model-invocation: false
name: coingecko-historical
user-invocable: true
description: This skill should be used when the user asks to "open CoinGecko historical data", "show historical price on date X", "open coingecko historical page", or wants to view the CoinGecko historical-data page for a coin around a given date in their default browser.
---

# CoinGecko Historical

Open the CoinGecko historical-data page for a coin in the user's default browser, centered on a one-day window around a target date.

## Arguments

- **coin-id** (required): CoinGecko coin slug (e.g., `bitcoin`, `ethereum`, `solana`). This is the same ID used by the CoinGecko API and the `cg` CLI — *not* the ticker symbol. If the user provides only a symbol or name, resolve it first (e.g., `cg search <term> -o json` or `https://api.coingecko.com/api/v3/search?query=<term>`).
- **date** (required): Target date in `YYYY-MM-DD` format.

## Behavior

Compute `DATE-1` (the day before `date`) and `DATE+1` (the day after `date`), then open:

```
https://coingecko.com/en/coins/<coin-id>/historical_data?start=<DATE-1>&end=<DATE+1>
```

Use the macOS `open` command:

```bash
open "https://coingecko.com/en/coins/<coin-id>/historical_data?start=<DATE-1>&end=<DATE+1>"
```

### Date arithmetic

Use GNU/BSD `date` to compute the surrounding days. BSD `date` (default on macOS) syntax:

```bash
start=$(date -j -v-1d -f "%Y-%m-%d" "$DATE" +%Y-%m-%d)
end=$(date -j -v+1d -f "%Y-%m-%d" "$DATE" +%Y-%m-%d)
```

## Example

User invokes: `/coingecko-historical bitcoin 2024-03-14`

Run:

```bash
open "https://coingecko.com/en/coins/bitcoin/historical_data?start=2024-03-13&end=2024-03-15"
```

## Notes

- `open` is macOS-only. On Linux substitute `xdg-open`; on Windows use `start`.
- Validate the date format before computing offsets; stop and ask the user if `date` is missing or malformed.
