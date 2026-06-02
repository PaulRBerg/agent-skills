# Etherscan Supported Chains Reference

Source: <https://docs.etherscan.io/supported-chains>
Live list: <https://api.etherscan.io/v2/chainlist> (returns the authoritative `chainid` set)

Verified against the live `chainlist` endpoint (64 chains).

## Major Mainnets (Free Tier Available)

| Chain            | Chain ID | Notes         |
| ---------------- | -------- | ------------- |
| Ethereum Mainnet | `1`      | Default chain |
| Polygon Mainnet  | `137`    |               |
| Arbitrum One     | `42161`  |               |
| Linea Mainnet    | `59144`  |               |
| Blast Mainnet    | `81457`  |               |
| Mantle Mainnet   | `5000`   |               |
| Unichain Mainnet | `130`    |               |
| Moonbeam         | `1284`   |               |
| Moonriver        | `1285`   |               |
| Gnosis           | `100`    |               |
| Celo Mainnet     | `42220`  |               |
| Fraxtal Mainnet  | `252`    |               |

## Free Tier NOT Available

The following chains require a paid Etherscan plan for data endpoints (balances, transactions, logs, etc.). **Lite ($49/mo) is sufficient** — it grants access to every supported chain:

| Chain                | Chain ID   |
| -------------------- | ---------- |
| Base Mainnet         | `8453`     |
| Base Sepolia Testnet | `84532`    |
| OP Mainnet           | `10`       |
| OP Sepolia Testnet   | `11155420` |
| Avalanche C-Chain    | `43114`    |
| Avalanche Fuji       | `43113`    |
| BNB Smart Chain      | `56`       |
| BNB Testnet          | `97`       |

**Exception:** `module=contract` endpoints (`getsourcecode`, `getabi`, etc.) work on **all** chains for **every** plan, including the free tier. The paid-plan requirement above applies only to data endpoints.

## Other Mainnets (Free Tier Available)

| Chain            | Chain ID |
| ---------------- | -------- |
| Abstract         | `2741`   |
| ApeChain         | `33139`  |
| Berachain        | `80094`  |
| BitTorrent Chain | `199`    |
| HyperEVM         | `999`    |
| Katana           | `747474` |
| MegaETH          | `4326`   |
| Memecore         | `4352`   |
| Monad            | `143`    |
| opBNB            | `204`    |
| Plasma           | `9745`   |
| Sei              | `1329`   |
| Sonic            | `146`    |
| Stable           | `988`    |
| Taiko            | `167000` |
| World            | `480`    |
| XDC              | `50`     |

## Testnets (Free Tier Available)

| Chain                        | Chain ID    |
| ---------------------------- | ----------- |
| Sepolia                      | `11155111`  |
| Hoodi                        | `560048`    |
| Abstract Sepolia             | `11124`     |
| ApeChain Curtis              | `33111`     |
| Arbitrum Sepolia             | `421614`    |
| Berachain Bepolia            | `80069`     |
| BitTorrent Testnet           | `1029`      |
| Blast Sepolia                | `168587773` |
| Celo Sepolia                 | `11142220`  |
| Fraxtal Hoodi                | `2523`      |
| Katana Bokuto                | `737373`    |
| Linea Sepolia                | `59141`     |
| Mantle Sepolia               | `5003`      |
| MegaETH Testnet              | `6343`      |
| Memecore Insectarium Testnet | `43522`     |
| Monad Testnet                | `10143`     |
| Moonbase Alpha               | `1287`      |
| opBNB Testnet                | `5611`      |
| Plasma Testnet               | `9746`      |
| Polygon Amoy                 | `80002`     |
| Sei Testnet                  | `1328`      |
| Sonic Testnet                | `14601`     |
| Stable Testnet               | `2201`      |
| Taiko Hoodi                  | `167013`    |
| Unichain Sepolia             | `1301`      |
| World Sepolia                | `4801`      |
| XDC Apothem                  | `51`        |

## Recently Removed / Deprecated

These chains were previously supported but **API requests will fail** on the V2 endpoint as of the dates noted:

| Chain                              | Chain ID | Deprecated                         |
| ---------------------------------- | -------- | ---------------------------------- |
| Scroll Mainnet                     | `534352` | Apr 16, 2026                       |
| Scroll Sepolia                     | `534351` | Apr 16, 2026                       |
| Swell Mainnet                      | `1923`   | Feb 25, 2026                       |
| Swell Testnet                      | `1924`   | Feb 25, 2026                       |
| MemeCore Formicarium Testnet (old) | `43521`  | Feb 25, 2026 (migrated to `43522`) |

The following chains are **not currently supported** on Etherscan V2 and were never part of the V2 unified API: Holesky, zkSync Era (`324`), zkSync Sepolia (`300`), Arbitrum Nova (`42170`). If a user references these, route them to a chain-native explorer or try Blockscout (`./blockscout-api.md`) instead.
