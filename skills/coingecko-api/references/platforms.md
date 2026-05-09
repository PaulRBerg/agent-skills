# CoinGecko Asset Platforms Reference

## Dynamic Platform List Endpoint

CoinGecko provides an endpoint to fetch all supported asset platforms:

```bash
curl -s "$CG_BASE/asset_platforms" "${CG_AUTH[@]}"
```

Use this endpoint to get the most up-to-date platform support information.

## Major EVM Chains

| Chain             | Platform ID           | Chain ID |
| ----------------- | --------------------- | -------- |
| Ethereum          | `ethereum`            | `1`      |
| Polygon           | `polygon-pos`         | `137`    |
| Arbitrum One      | `arbitrum-one`        | `42161`  |
| Optimism          | `optimistic-ethereum` | `10`     |
| Base              | `base`                | `8453`   |
| BNB Smart Chain   | `binance-smart-chain` | `56`     |
| Avalanche C-Chain | `avalanche`           | `43114`  |
| Gnosis            | `xdai`                | `100`    |
| Fantom            | `fantom`              | `250`    |
| Cronos            | `cronos`              | `25`     |
| Celo              | `celo`                | `42220`  |
| Moonbeam          | `moonbeam`            | `1284`   |
| Moonriver         | `moonriver`           | `1285`   |

## Layer 2 & Rollups

| Chain      | Platform ID       | Chain ID |
| ---------- | ----------------- | -------- |
| zkSync Era | `zksync`          | `324`    |
| Linea      | `linea`           | `59144`  |
| Scroll     | `scroll`          | `534352` |
| Blast      | `blast`           | `81457`  |
| Mantle     | `mantle`          | `5000`   |
| Mode       | `mode`            | `34443`  |
| Metis      | `metis-andromeda` | `1088`   |

## Other Chains

| Chain    | Platform ID     | Chain ID |
| -------- | --------------- | -------- |
| Solana   | `solana`        | N/A      |
| Tron     | `tron`          | N/A      |
| Near     | `near-protocol` | N/A      |
| Sui      | `sui`           | N/A      |
| Aptos    | `aptos`         | N/A      |
| Sei      | `sei-v2`        | `1329`   |
| Starknet | `starknet`      | N/A      |

## Notes

- Platform IDs are case-sensitive (always lowercase)
- Not all tokens on a chain are indexed by CoinGecko
- New tokens may take time to appear in the database
- Testnets are not supported
