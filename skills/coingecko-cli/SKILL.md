---
disable-model-invocation: true
name: coingecko-cli
user-invocable: true
description: 'Use for CoinGecko/cg CLI crypto market data: prices, market cap, trending coins, top gainers/losers, coin search, historical data, or OHLC.'
---

# CoinGecko CLI

## Overview

Query cryptocurrency market data using the official CoinGecko CLI (`cg`). The CLI wraps the CoinGecko V3 API with structured JSON output, a dry-run mode, and a self-describing command catalog — designed to be driven by agents.

**Key capabilities:**

- Current prices by coin ID or symbol (`cg price`)
- Top coins by market cap with filters and CSV export (`cg markets`)
- Coin search with fuzzy matching (`cg search`)
- Trending coins, NFTs, and categories (`cg trending`)
- Historical price snapshots, time series, and OHLC (`cg history`)
- Top gainers/losers over configurable durations (`cg top-gainers-losers`, paid)

**Out of scope** — `cg` does not support contract-address price lookups, global market stats (total market cap, BTC/ETH dominance), NFT detail endpoints, GeckoTerminal on-chain DEX queries, or token logo metadata. Fall back to `WebFetch` against [docs.coingecko.com/llms.txt](https://docs.coingecko.com/llms.txt) for those.

## Prerequisites

Verify the CLI is installed:

```bash
command -v cg || { echo "cg not on PATH; install via: brew install coingecko/coingecko-cli/cg"; exit 1; }
```

Check authentication and tier without running interactive setup:

```bash
cg status -o json
```

The CLI reads `CG_API_KEY` and `CG_API_TIER` (`demo` or `paid`) from the environment. If `cg status` shows no key, **ask the user** before invoking `cg auth` — that command is interactive and writes config to disk.

### Tier Matrix

| Tier        | Rate Limit       | Monthly Cap | History | Paid-Only Commands   |
| ----------- | ---------------- | ----------- | ------- | -------------------- |
| Demo (free) | ~30 req/min      | ~10,000     | 1 year  | —                    |
| Analyst     | ~500 req/min     | ~500,000    | Deeper  | `top-gainers-losers` |
| Lite / Pro  | 500–1000 req/min | 1M–3M       | Deeper  | `top-gainers-losers` |

Detect feature gating up front by reading the `paid_only` field in `cg commands -o json`.

## Output Conventions

- **Always pass `-o json`** for any output that needs to be parsed by the agent. The default `table` format is for humans.
- **Use `--dry-run`** to preview the underlying API request as JSON without executing it. Useful when validating an unfamiliar flag combination before spending a quota.
- **Use `--export <file.csv>`** only when the user explicitly asks for CSV (`cg markets`, `cg history`, `cg top-gainers-losers` support this).

## Coin ID Resolution

CoinGecko identifies coins by string ID (e.g. `bitcoin`, `ethereum`), not symbol. Many IDs diverge from the coin's common name.

**Resolution order:**

1. **Known coin** — use the table below.
2. **Symbol** — `cg price --symbols btc,eth ...` works but symbols are non-unique. Only the top-market-cap coin per symbol is returned.
3. **Unknown / ambiguous** — `cg search <term> -o json` to resolve the ID first, then re-issue the real query.

### Common Coin IDs

| Coin        | ID                        | Symbol |
| ----------- | ------------------------- | ------ |
| Bitcoin     | `bitcoin`                 | BTC    |
| Ethereum    | `ethereum`                | ETH    |
| Solana      | `solana`                  | SOL    |
| BNB         | `binancecoin`             | BNB    |
| XRP         | `ripple`                  | XRP    |
| Cardano     | `cardano`                 | ADA    |
| Dogecoin    | `dogecoin`                | DOGE   |
| Chainlink   | `chainlink`               | LINK   |
| Avalanche   | `avalanche-2`             | AVAX   |
| Polygon     | `polygon-ecosystem-token` | POL    |
| Uniswap     | `uniswap`                 | UNI    |
| Aave        | `aave`                    | AAVE   |
| Maker       | `maker`                   | MKR    |
| USDC        | `usd-coin`                | USDC   |
| USDT        | `tether`                  | USDT   |
| DAI         | `dai`                     | DAI    |
| Wrapped BTC | `wrapped-bitcoin`         | WBTC   |

## Core Commands

### Price

```bash
cg price --ids bitcoin,ethereum --vs usd -o json
cg price --symbols btc,eth --vs eur -o json
```

Returns `{ "<id>": { "<currency>": <price>, "<currency>_24h_change": <pct> } }`.

### Markets

```bash
# Top 50 by market cap in USD
cg markets --total 50 --vs usd -o json

# Layer-1 category, sorted by volume, exported to CSV
cg markets --category layer-1 --order volume_desc --export l1.csv
```

`--order` enum: `market_cap_desc` (default), `market_cap_asc`, `volume_desc`, `volume_asc`, `id_asc`, `id_desc`. Category IDs come from `cg commands -o json` or [/coins/categories/list](https://docs.coingecko.com/reference/coins-categories-list).

### Search

```bash
cg search ethereum --limit 5 -o json
```

Returns an array of `{ id, name, symbol, market_cap_rank }`. Use this first whenever the user gives only a name or symbol.

### Trending

```bash
cg trending -o json
```

Returns trending `coins`, `nfts`, and `categories` from the last 24h. `--show-max coins,nfts,categories` increases the per-section limit (paid only).

### History

`cg history <coin>` has three mutually exclusive query modes:

```bash
# Snapshot on a single date
cg history bitcoin --date 2024-01-01 -o json

# Last N days, auto-granularity
cg history ethereum --days 30 -o json

# Explicit date range; --to covers the full day up to 23:59:59 UTC
cg history solana --from 2024-01-01 --to 2024-03-01 -o json
```

Modifiers:

- `--ohlc` — switch from price series to candlesticks. In `--days` mode, only `1, 7, 14, 30, 90, 180, 365, max` are accepted.
- `--interval daily|hourly` — override auto-granularity for `--days` or `--from/--to`. Large ranges auto-batch into multiple API requests.
- `--vs <currency>` — quote currency (default `usd`).
- `--export <file.csv>` — write to CSV.

Returned JSON in price mode: `{ "prices": [[ts_ms, price], ...], "market_caps": [...], "total_volumes": [...] }`. In OHLC mode the response is a bare array of `[ts_ms, open, high, low, close]` tuples.

### Top Gainers / Losers (paid)

```bash
cg top-gainers-losers --duration 24h --top-coins 1000 -o json
cg top-gainers-losers --losers --duration 7d --price-change-percentage 1h,7d,30d -o json
```

`--duration` enum: `1h, 24h, 7d, 14d, 30d, 60d, 1y`. `--top-coins` enum: `300, 500, 1000, all`. Returns 401 on demo tier — check `paid_only` via `cg commands` before calling.

## Output Formatting

Default to a Markdown table when presenting results to the user. Honour explicit format requests (JSON, CSV, plain text).

**Number formatting:**

- Prices > $1 → 2 decimals (`$67,187.34`)
- Prices $0.01 – $1 → 4 decimals (`$0.4523`)
- Prices < $0.01 → 6+ decimals (`$0.000001234`)
- Market caps → abbreviated (`$1.32T`, `$415.2B`, `$8.5M`)
- Percentages → signed, 2 decimals (`+3.64%`, `-1.23%`)

## Self-Discovery

When the inline examples don't cover what's needed, query the CLI's own machine-readable catalog:

```bash
cg commands -o json
```

Each entry includes `name`, `description`, `examples`, `flags` (with `type`, `default`, `enum`), `output_formats`, `api_endpoint`, `oas_operation_id`, `requires_auth`, and `paid_only`. Use it to detect tier gating before issuing a command, or to find the exact enum values for a flag.

For one-off flag lookups:

```bash
cg <subcommand> --help
```

## Rate Limits & Errors

- **HTTP 429** — back off briefly and retry; batch coin IDs into single calls (`--ids a,b,c`) wherever the command allows.
- **Missing / invalid key** — `cg status -o json` surfaces it. Prompt the user; do not run `cg auth` non-interactively.
- **Paid-only command on demo tier** — detect via the `paid_only` field in `cg commands -o json` before invoking. The CLI will otherwise return an API error.

## Additional Resources

- **CLI repo:** <https://github.com/coingecko/coingecko-cli>
- **AI agent guide:** <https://docs.coingecko.com/docs/ai-agent-hub/cli>
- **AI-friendly API docs:** <https://docs.coingecko.com/llms.txt> — fetch via `WebFetch` for endpoints `cg` does not expose (contract-address prices, NFT details, GeckoTerminal on-chain DEX data).
