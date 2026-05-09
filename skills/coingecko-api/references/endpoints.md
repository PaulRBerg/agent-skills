# CoinGecko API Endpoint Reference

Curated endpoint reference for the CoinGecko API V3. All endpoints use GET method.

## Base URLs

| Tier | Base URL | Auth Header | Auth Query Param |
| ---- | -------- | ----------- | ---------------- |
| Demo | `https://api.coingecko.com/api/v3` | `x-cg-demo-api-key` | `x_cg_demo_api_key` |
| Pro | `https://pro-api.coingecko.com/api/v3` | `x-cg-pro-api-key` | `x_cg_pro_api_key` |

## Coins

### GET /coins/list

List all supported coins with IDs, names, and symbols.

| Parameter | Required | Default | Description |
| --------- | -------- | ------- | ----------- |
| `include_platform` | No | `false` | Include platform contract addresses |
| `status` | No | `active` | Filter by state: `active` or `inactive` |

Response:

```json
[
  {
    "id": "bitcoin",
    "symbol": "btc",
    "name": "Bitcoin",
    "platforms": {}
  }
]
```

### GET /coins/markets

List coins with market data, sorted and paginated.

| Parameter | Required | Default | Description |
| --------- | -------- | ------- | ----------- |
| `vs_currency` | Yes | - | Target currency (e.g., `usd`) |
| `ids` | No | - | Comma-separated coin IDs |
| `names` | No | - | Comma-separated coin names |
| `symbols` | No | - | Comma-separated symbols |
| `category` | No | - | Filter by category |
| `order` | No | `market_cap_desc` | Sort: `market_cap_desc`, `market_cap_asc`, `volume_desc`, `volume_asc`, `id_asc`, `id_desc` |
| `per_page` | No | `100` | Results per page (1-250) |
| `page` | No | `1` | Page number |
| `sparkline` | No | `false` | Include 7-day sparkline |
| `price_change_percentage` | No | - | Comma-separated: `1h,24h,7d,14d,30d,200d,1y` |
| `precision` | No | - | Decimal places (0-18 or `full`) |

Response fields per coin: `id`, `symbol`, `name`, `current_price`, `market_cap`, `market_cap_rank`, `total_volume`, `price_change_24h`, `price_change_percentage_24h`, `circulating_supply`, `max_supply`, `ath`, `atl`, `last_updated`.

### GET /coins/{id}

Detailed coin data including market data, tickers, community, and developer stats. Also available as `/coins/{platform_id}/contract/{address}` to resolve by contract address instead of coin ID (same parameters and response).

| Parameter | Required | Default | Description |
| --------- | -------- | ------- | ----------- |
| `id` | Yes (path) | - | Coin ID (e.g., `bitcoin`) |
| `localization` | No | `true` | Include localized data |
| `tickers` | No | `true` | Include exchange tickers |
| `market_data` | No | `true` | Include market data |
| `community_data` | No | `true` | Include community stats |
| `developer_data` | No | `true` | Include developer stats |
| `sparkline` | No | `false` | Include 7-day sparkline |

### GET /coins/{id}/market_chart

Historical market data (price, market cap, volume) as time series.

| Parameter | Required | Default | Description |
| --------- | -------- | ------- | ----------- |
| `id` | Yes (path) | - | Coin ID |
| `vs_currency` | Yes | - | Target currency |
| `days` | Yes | - | Data range: integer or `max` |
| `interval` | No | auto | `5m` (paid only), `hourly`, or `daily` |
| `precision` | No | - | Decimal places (0-18 or `full`) |

Auto-granularity: 1 day = 5-min, 2-90 days = hourly, >90 days = daily (00:00 UTC).

Response:

```json
{
  "prices": [[1711843200000, 69702.31], ...],
  "market_caps": [[1711843200000, 1370247487960.09], ...],
  "total_volumes": [[1711843200000, 16408802301.84], ...]
}
```

### GET /coins/{id}/ohlc

OHLC candlestick data.

| Parameter | Required | Default | Description |
| --------- | -------- | ------- | ----------- |
| `id` | Yes (path) | - | Coin ID |
| `vs_currency` | Yes | - | Target currency |
| `days` | Yes | - | `1`, `7`, `14`, `30`, `90`, `180`, `365`, or `max` |
| `precision` | No | - | Decimal places |

Response: `[[timestamp, open, high, low, close], ...]`

## Simple Price Queries

### GET /simple/price

Quick price lookup by coin ID, name, or symbol.

| Parameter | Required | Default | Description |
| --------- | -------- | ------- | ----------- |
| `vs_currencies` | Yes | - | Target currencies, comma-separated |
| `ids` | No* | - | Coin IDs, comma-separated |
| `names` | No* | - | Coin names, comma-separated |
| `symbols` | No* | - | Coin symbols, comma-separated |
| `include_market_cap` | No | `false` | Include market cap |
| `include_24hr_vol` | No | `false` | Include 24h volume |
| `include_24hr_change` | No | `false` | Include 24h change |
| `include_last_updated_at` | No | `false` | Include last update timestamp |
| `precision` | No | - | Decimal places (0-18 or `full`) |

*At least one of `ids`, `names`, or `symbols` required. Priority: `ids` > `names` > `symbols`.

Response:

```json
{
  "bitcoin": {
    "usd": 67187.34,
    "usd_market_cap": 1317802988326.25,
    "usd_24h_vol": 31260929299.52,
    "usd_24h_change": 3.64
  }
}
```

### GET /simple/token_price/{id}

Price lookup by contract address on a specific platform.

| Parameter | Required | Default | Description |
| --------- | -------- | ------- | ----------- |
| `id` | Yes (path) | - | Asset platform ID (e.g., `ethereum`) |
| `contract_addresses` | Yes | - | Token contract addresses, comma-separated |
| `vs_currencies` | Yes | - | Target currencies, comma-separated |
| `include_market_cap` | No | `false` | Include market cap |
| `include_24hr_vol` | No | `false` | Include 24h volume |
| `include_24hr_change` | No | `false` | Include 24h change |
| `include_last_updated_at` | No | `false` | Include last update timestamp |
| `precision` | No | - | Decimal places |

Response keyed by contract address:

```json
{
  "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599": {
    "usd": 67187.34,
    "usd_market_cap": 1317802988326.25
  }
}
```

## Search & Trending

### GET /search

Search for coins, exchanges, categories, and NFTs.

| Parameter | Required | Description |
| --------- | -------- | ----------- |
| `query` | Yes | Search term |

Response: `{ "coins": [...], "exchanges": [...], "categories": [...], "nfts": [...] }`

### GET /search/trending

Top trending coins, NFTs, and categories (based on search activity).

| Parameter | Required | Description |
| --------- | -------- | ----------- |
| `show_max` | No (paid plans only) | Show max results for types: `coins`, `nfts`, `categories` (comma-separated) |

Returns up to 15 coins, 7 NFTs, 6 categories (defaults). Paid plans get higher limits.

## Asset Platforms

### GET /asset_platforms

List all supported asset platforms with chain IDs.

| Parameter | Required | Description |
| --------- | -------- | ----------- |
| `filter` | No | Filter type (e.g., `nft`) |

Response:

```json
[
  {
    "id": "ethereum",
    "chain_identifier": 1,
    "name": "Ethereum",
    "shortname": "ETH",
    "native_coin_id": "ethereum"
  }
]
```

## Global Data

### GET /global

Global cryptocurrency market data (total market cap, volume, dominance).

No parameters required.

### GET /global/decentralized_finance_defi

Global DeFi market data.

No parameters required.

## Exchanges

### GET /exchanges

List all exchanges with trading volume.

| Parameter | Required | Default | Description |
| --------- | -------- | ------- | ----------- |
| `per_page` | No | `100` | Results per page |
| `page` | No | `1` | Page number |

### GET /exchanges/{id}

Exchange details by ID.

### GET /exchanges/{id}/tickers

Exchange tickers with pagination.

## API Key Info

### GET /key

**Pro/Analyst/Lite plans only** (requires Pro base URL). Returns 401 (10005) on Demo base URL.

No parameters required (authenticated via header).

Response: `plan`, `rate_limit_request_per_minute`, `monthly_call_credit`, `current_remaining_monthly_calls`.

## Asset Platform IDs

For a curated list of common platform IDs, see `PLATFORMS.md`.
