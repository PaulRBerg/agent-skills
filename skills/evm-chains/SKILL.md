---
argument-hint: <chain-name-or-id>
disable-model-invocation: false
name: evm-chains
user-invocable: true
description: This skill should be used when the user asks to resolve an EVM chain name or chain ID, find chain metadata such as a default public RPC, native currency symbol, or block explorer URL, determine whether a chain is supported by RouteMesh, or needs chain resolution before fetching data from or interacting with an EVM chain.
---

# EVM Chains

Local EVM chain reference for chain name, chain ID, default public RPC, native currency symbol, default block explorer URL, and RouteMesh support lookups.

Use this skill to resolve chain metadata before reading from an RPC, sending transactions, calling contracts, constructing chain-specific RPC URLs, or building explorer links to addresses, transactions, or blocks.

Match chains by displayed name or numeric chain ID. Treat any chain missing from the tables as outside this skill's local dataset. If the requested chain is not listed, use the web search tool to find authoritative metadata from the chain's official documentation or Chainlist before proceeding.

## RouteMesh

Use RouteMesh only when the `RouteMesh` column is `Yes` and the `ROUTEMESH_API_KEY` environment variable is available.

Construct the RouteMesh RPC URL as:

```text
https://lb.routeme.sh/rpc/CHAIN_ID/ROUTEMESH_API_KEY
```

Replace `CHAIN_ID` with the numeric chain ID and `ROUTEMESH_API_KEY` with the value of the `ROUTEMESH_API_KEY` environment variable. If `RouteMesh` is `No` or `ROUTEMESH_API_KEY` is not available, use the chain's default public RPC instead.

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

| Chain name    | Chain ID  | Default public RPC                             | Native currency symbol | Explorer URL                    | RouteMesh |
| ------------- | --------- | ---------------------------------------------- | ---------------------- | ------------------------------- | --------- |
| Abstract      | 2741      | https://api.mainnet.abs.xyz                    | ETH                    | https://abscan.org              | Yes       |
| Arbitrum      | 42161     | https://arb1.arbitrum.io/rpc                   | ETH                    | https://arbiscan.io             | Yes       |
| Avalanche     | 43114     | https://api.avax.network/ext/bc/C/rpc          | AVAX                   | https://snowscan.xyz            | Yes       |
| Base          | 8453      | https://mainnet.base.org                       | ETH                    | https://basescan.org            | Yes       |
| Berachain     | 80094     | https://rpc.berachain.com                      | BERA                   | https://berascan.com            | Yes       |
| Blast         | 81457     | https://rpc.blast.io                           | ETH                    | https://blastscan.io            | Yes       |
| BNB Chain     | 56        | https://56.rpc.thirdweb.com                    | BNB                    | https://bscscan.com             | Yes       |
| Chiliz        | 88888     | https://rpc.chiliz.com                         | CHZ                    | https://chiliscan.com           | Yes       |
| Core Dao      | 1116      | https://rpc.coredao.org                        | CORE                   | https://scan.coredao.org        | Yes       |
| Ethereum      | 1         | https://eth.merkle.io                          | ETH                    | https://etherscan.io            | Yes       |
| Form          | 478       | https://rpc.form.network/http                  | ETH                    | https://explorer.form.network   | No        |
| Gnosis        | 100       | https://rpc.gnosischain.com                    | XDAI                   | https://gnosisscan.io           | Yes       |
| HyperEVM      | 999       | https://rpc.hyperliquid.xyz/evm                | HYPE                   | https://hyperevmscan.io         | Yes       |
| IoTeX         | 4689      | https://babel-api.mainnet.iotex.io             | IOTX                   | https://iotexscan.io            | Yes       |
| Lightlink     | 1890      | https://replicator.phoenix.lightlink.io/rpc/v1 | ETH                    | https://phoenix.lightlink.io    | Yes       |
| Linea Mainnet | 59144     | https://rpc.linea.build                        | ETH                    | https://lineascan.build         | Yes       |
| Meld          | 333000333 | https://rpc-1.meld.com                         | MELD                   | https://meldscan.io             | No        |
| Mode          | 34443     | https://mainnet.mode.network                   | ETH                    | https://modescan.io             | Yes       |
| Monad         | 143       | https://rpc.monad.xyz                          | MON                    | https://monadscan.com           | Yes       |
| Morph         | 2818      | https://rpc.morphl2.io                         | ETH                    | https://explorer.morphl2.io     | Yes       |
| OP Mainnet    | 10        | https://mainnet.optimism.io                    | ETH                    | https://optimistic.etherscan.io | Yes       |
| Polygon       | 137       | https://polygon-rpc.com                        | POL                    | https://polygonscan.com         | Yes       |
| Ronin         | 2020      | https://api.roninchain.com/rpc                 | RON                    | https://app.roninchain.com      | Yes       |
| Scroll        | 534352    | https://rpc.scroll.io                          | ETH                    | https://scrollscan.com          | Yes       |
| Sei Network   | 1329      | https://evm-rpc.sei-apis.com                   | SEI                    | https://seiscan.io              | Yes       |
| Sonic         | 146       | https://rpc.soniclabs.com                      | S                      | https://sonicscan.org           | Yes       |
| Sophon        | 50104     | https://rpc.sophon.xyz                         | SOPH                   | https://sophscan.xyz            | Yes       |
| Superseed     | 5330      | https://mainnet.superseed.xyz                  | ETH                    | https://explorer.superseed.xyz  | Yes       |
| Tangle        | 5845      | https://rpc.tangle.tools                       | TNT                    | https://explorer.tangle.tools   | Yes       |
| Unichain      | 130       | https://mainnet.unichain.org/                  | ETH                    | https://uniscan.xyz             | Yes       |
| XDC           | 50        | https://rpc.xdcrpc.com                         | XDC                    | https://xdcscan.com             | Yes       |
| ZKsync Era    | 324       | https://mainnet.era.zksync.io                  | ETH                    | https://explorer.zksync.io      | Yes       |
| Zora          | 7777777   | https://zora.drpc.org                          | ETH                    | https://explorer.zora.energy    | No        |

## Testnets

| Chain name       | Chain ID | Default public RPC                | Native currency symbol | Explorer URL                 | RouteMesh |
| ---------------- | -------- | --------------------------------- | ---------------------- | ---------------------------- | --------- |
| Ethereum Sepolia | 11155111 | https://11155111.rpc.thirdweb.com | ETH                    | https://sepolia.etherscan.io | Yes       |
