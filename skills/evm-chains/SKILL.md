---
argument-hint: <chain-name-or-id>
disable-model-invocation: false
name: evm-chains
user-invocable: true
description: This skill should be used when the user asks to resolve an EVM chain name or chain ID, find chain metadata such as a default public RPC, native currency symbol, or block explorer URL, determine whether a chain is supported by RouteMesh, or needs chain resolution before fetching data from or interacting with an EVM chain.
---

# EVM Chains

Local EVM chain reference for chain name, chain ID, public RPCs, native currency symbol, default block explorer URL, and RouteMesh support lookups.

Use this skill to resolve chain metadata before reading from an RPC, sending transactions, calling contracts, constructing chain-specific RPC URLs, or building explorer links to addresses, transactions, or blocks.

Match chains by displayed name or numeric chain ID. Treat any chain missing from the tables as outside this skill's local dataset. If the requested chain is not listed, use the web search tool to find authoritative metadata from the chain's official documentation or Chainlist before proceeding.

## RouteMesh

Use RouteMesh only when the `RouteMesh` column is `Yes` and the `ROUTEMESH_API_KEY` environment variable is available.

Construct the RouteMesh RPC URL as:

```text
https://lb.routeme.sh/rpc/CHAIN_ID/ROUTEMESH_API_KEY
```

Replace `CHAIN_ID` with the numeric chain ID and `ROUTEMESH_API_KEY` with the value of the `ROUTEMESH_API_KEY` environment variable. If `RouteMesh` is `No` or `ROUTEMESH_API_KEY` is not available, use the chain's primary public RPC first, then the listed fallback RPCs in order.

## Public RPCs

Public RPCs are best-effort. Before relying on one for data fetches or contract calls, verify it with `eth_chainId`. If the primary endpoint fails, try the fallback endpoints for that chain from the fallback table.

Do not use RPCs for Form, Meld, or Tangle. They are intentionally omitted from the active mainnet table because those chains are now defunct and no longer operating. If the user asks about Form, Meld, or Tangle, say that the chain is defunct/no longer operating instead of returning an RPC URL.

## Explorer URLs

The `Explorer URL` column is the base URL of the chain's canonical block explorer. Append standard path segments to build links:

| Resource    | Path              | Example                                  |
| ----------- | ----------------- | ---------------------------------------- |
| Address     | `/address/<addr>` | `https://arbiscan.io/address/0xabc...`   |
| Transaction | `/tx/<hash>`      | `https://etherscan.io/tx/0x123...`       |
| Block       | `/block/<number>` | `https://basescan.org/block/12345678`    |
| Token       | `/token/<addr>`   | `https://polygonscan.com/token/0xdef...` |

Etherscan and Etherscan-stack explorers (Arbiscan, Basescan, BscScan, Polygonscan, Optimism Etherscan, Lineascan, Snowscan, Blastscan, Scrollscan, Berascan, Uniscan, Gnosisscan, abscan.org) all follow this scheme. Most Blockscout-based and chain-native explorers accept the same segments, but conventions can drift — verify against the explorer UI when in doubt.

**Ronin (`app.roninchain.com`) does not follow the Etherscan path scheme.** Verify against the explorer UI before constructing a Ronin link.

## Mainnets

| Chain name    | Chain ID | Primary public RPC                             | Native currency symbol | Explorer URL                    | RouteMesh |
| ------------- | -------- | ---------------------------------------------- | ---------------------- | ------------------------------- | --------- |
| Abstract      | 2741     | https://api.mainnet.abs.xyz                    | ETH                    | https://abscan.org              | Yes       |
| Arbitrum      | 42161    | https://arb1.arbitrum.io/rpc                   | ETH                    | https://arbiscan.io             | Yes       |
| Avalanche     | 43114    | https://api.avax.network/ext/bc/C/rpc          | AVAX                   | https://snowscan.xyz            | Yes       |
| Base          | 8453     | https://mainnet.base.org                       | ETH                    | https://basescan.org            | Yes       |
| Berachain     | 80094    | https://rpc.berachain.com                      | BERA                   | https://berascan.com            | Yes       |
| Blast         | 81457    | https://rpc.blast.io                           | ETH                    | https://blastscan.io            | Yes       |
| BNB Chain     | 56       | https://bsc-dataseed1.bnbchain.org             | BNB                    | https://bscscan.com             | Yes       |
| Celo          | 42220    | https://forno.celo.org                         | CELO                   | https://celoscan.io             | Yes       |
| Chiliz        | 88888    | https://rpc.chiliz.com                         | CHZ                    | https://chiliscan.com           | Yes       |
| Core Dao      | 1116     | https://rpc.coredao.org                        | CORE                   | https://scan.coredao.org        | Yes       |
| Ethereum      | 1        | https://ethereum-rpc.publicnode.com            | ETH                    | https://etherscan.io            | Yes       |
| Gnosis        | 100      | https://rpc.gnosischain.com                    | XDAI                   | https://gnosisscan.io           | Yes       |
| HyperEVM      | 999      | https://rpc.hyperliquid.xyz/evm                | HYPE                   | https://hyperevmscan.io         | Yes       |
| IoTeX         | 4689     | https://babel-api.mainnet.iotex.io             | IOTX                   | https://iotexscan.io            | Yes       |
| Lightlink     | 1890     | https://replicator.phoenix.lightlink.io/rpc/v1 | ETH                    | https://phoenix.lightlink.io    | Yes       |
| Linea Mainnet | 59144    | https://rpc.linea.build                        | ETH                    | https://lineascan.build         | Yes       |
| Mode          | 34443    | https://mainnet.mode.network                   | ETH                    | https://modescan.io             | Yes       |
| Monad         | 143      | https://rpc.monad.xyz                          | MON                    | https://monadscan.com           | Yes       |
| Morph         | 2818     | https://rpc.morphl2.io                         | ETH                    | https://explorer.morphl2.io     | Yes       |
| OP Mainnet    | 10       | https://mainnet.optimism.io                    | ETH                    | https://optimistic.etherscan.io | Yes       |
| Polygon       | 137      | https://polygon-bor-rpc.publicnode.com         | POL                    | https://polygonscan.com         | Yes       |
| Ronin         | 2020     | https://api.roninchain.com/rpc                 | RON                    | https://app.roninchain.com      | Yes       |
| Scroll        | 534352   | https://rpc.scroll.io                          | ETH                    | https://scrollscan.com          | Yes       |
| Sei Network   | 1329     | https://evm-rpc.sei-apis.com                   | SEI                    | https://seiscan.io              | Yes       |
| Sonic         | 146      | https://rpc.soniclabs.com                      | S                      | https://sonicscan.org           | Yes       |
| Sophon        | 50104    | https://rpc.sophon.xyz                         | SOPH                   | https://sophscan.xyz            | Yes       |
| Superseed     | 5330     | https://mainnet.superseed.xyz                  | ETH                    | https://explorer.superseed.xyz  | Yes       |
| Unichain      | 130      | https://mainnet.unichain.org                   | ETH                    | https://uniscan.xyz             | Yes       |
| XDC           | 50       | https://rpc.xdcrpc.com                         | XDC                    | https://xdcscan.com             | Yes       |
| ZKsync Era    | 324      | https://mainnet.era.zksync.io                  | ETH                    | https://explorer.zksync.io      | Yes       |
| Zora          | 7777777  | https://zora.drpc.org                          | ETH                    | https://explorer.zora.energy    | No        |

## Mainnet Fallback RPCs

| Chain name    | Chain ID | Fallback public RPCs                                                                                         |
| ------------- | -------- | ------------------------------------------------------------------------------------------------------------ |
| Abstract      | 2741     | https://abstract.drpc.org, https://2741.rpc.thirdweb.com                                                     |
| Arbitrum      | 42161    | https://arbitrum-one-rpc.publicnode.com, https://42161.rpc.thirdweb.com, https://rpcfree.com/arbitrum-rpc    |
| Avalanche     | 43114    | https://avalanche-c-chain-rpc.publicnode.com, https://43114.rpc.thirdweb.com                                 |
| Base          | 8453     | https://base-rpc.publicnode.com, https://base.gateway.tenderly.co, https://developer-access-mainnet.base.org |
| Berachain     | 80094    | https://berachain-rpc.publicnode.com, https://80094.rpc.thirdweb.com, https://rpc.berachain-apis.com         |
| Blast         | 81457    | https://blast-rpc.publicnode.com, https://blast.drpc.org, https://81457.rpc.thirdweb.com                     |
| BNB Chain     | 56       | https://bsc-rpc.publicnode.com, https://bsc.drpc.org, https://56.rpc.thirdweb.com                            |
| Celo          | 42220    | https://celo.drpc.org, https://celo-rpc.publicnode.com, https://42220.rpc.thirdweb.com                       |
| Chiliz        | 88888    | https://chiliz.publicnode.com, https://88888.rpc.thirdweb.com, https://rpc.ankr.com/chiliz                   |
| Core Dao      | 1116     | https://core.drpc.org, https://1116.rpc.thirdweb.com, https://rpc-core.icecreamswap.com                      |
| Ethereum      | 1        | https://eth.drpc.org, https://rpc.flashbots.net, https://1.rpc.thirdweb.com                                  |
| Gnosis        | 100      | https://gnosis-rpc.publicnode.com, https://gnosis.drpc.org, https://100.rpc.thirdweb.com                     |
| HyperEVM      | 999      | https://hyperliquid.drpc.org, https://999.rpc.thirdweb.com, https://gwan-ssl.wandevs.org:46891               |
| IoTeX         | 4689     | https://4689.rpc.thirdweb.com                                                                                |
| Lightlink     | 1890     | https://1890.rpc.thirdweb.com                                                                                |
| Linea Mainnet | 59144    | https://linea-rpc.publicnode.com, https://59144.rpc.thirdweb.com                                             |
| Mode          | 34443    | https://mode.drpc.org, https://34443.rpc.thirdweb.com                                                        |
| Monad         | 143      | https://monad.drpc.org, https://143.rpc.thirdweb.com                                                         |
| Morph         | 2818     | https://morph.drpc.org, https://2818.rpc.thirdweb.com, https://rpc-quicknode.morphl2.io                      |
| OP Mainnet    | 10       | https://optimism-rpc.publicnode.com, https://optimism.drpc.org, https://10.rpc.thirdweb.com                  |
| Polygon       | 137      | https://polygon.drpc.org, https://137.rpc.thirdweb.com, https://rpc-mainnet.matic.quiknode.pro               |
| Ronin         | 2020     | https://ronin.drpc.org, https://2020.rpc.thirdweb.com                                                        |
| Scroll        | 534352   | https://scroll-rpc.publicnode.com, https://scroll.drpc.org, https://534352.rpc.thirdweb.com                  |
| Sei Network   | 1329     | https://sei.drpc.org, https://1329.rpc.thirdweb.com                                                          |
| Sonic         | 146      | https://sonic-rpc.publicnode.com, https://sonic.drpc.org, https://146.rpc.thirdweb.com                       |
| Sophon        | 50104    | https://50104.rpc.thirdweb.com                                                                               |
| Superseed     | 5330     | https://superseed.drpc.org, https://5330.rpc.thirdweb.com                                                    |
| Unichain      | 130      | https://unichain-rpc.publicnode.com, https://130.rpc.thirdweb.com                                            |
| XDC           | 50       | https://50.rpc.thirdweb.com, https://erpc.xdcrpc.com, https://rpc.xdc.org                                    |
| ZKsync Era    | 324      | https://zksync.drpc.org, https://324.rpc.thirdweb.com                                                        |
| Zora          | 7777777  | https://7777777.rpc.thirdweb.com, https://rpc.zora.energy                                                    |

## Testnets

| Chain name       | Chain ID | Primary public RPC                          | Native currency symbol | Explorer URL                 | RouteMesh |
| ---------------- | -------- | ------------------------------------------- | ---------------------- | ---------------------------- | --------- |
| Ethereum Sepolia | 11155111 | https://ethereum-sepolia-rpc.publicnode.com | ETH                    | https://sepolia.etherscan.io | Yes       |

## Testnet Fallback RPCs

| Chain name       | Chain ID | Fallback public RPCs                                                                            |
| ---------------- | -------- | ----------------------------------------------------------------------------------------------- |
| Ethereum Sepolia | 11155111 | https://11155111.rpc.thirdweb.com, https://sepolia.drpc.org, https://rpc.sepolia.ethpandaops.io |
