---
argument-hint: <chain-name-or-id>
disable-model-invocation: false
name: evm-chains
user-invocable: true
description: 'Use for EVM chain resolution and on-chain account data: chain name/ID, RPC/explorer/native symbol, RouteMesh, balances, token/NFT holdings/transfers, tx history, funding origin via Etherscan/Blockscout/Chainscout; bridge/cross-chain Bungee/Socket enrichment.'
---

# EVM Chains

Local EVM chain dataset (chain name, chain ID, public RPCs, native currency symbol, default block explorer URL, RouteMesh support) **and** a router for reading on-chain data: resolve the chain, then dispatch balance, token, transfer, transaction, and first-funding queries to Etherscan (preferred) or Blockscout (fallback). For bridge transactions and cross-chain swaps, enrich explorer/RPC verification with Bungee status data.

Use this skill to resolve chain metadata before reading from an RPC, sending transactions, calling contracts, constructing chain-specific RPC URLs, or building explorer links to addresses, transactions, or blocks — and to query on-chain account data once the chain is resolved (see [Querying On-Chain Data (Routing)](#querying-on-chain-data-routing)). Also use it when the user mentions bridging, bridge tx, cross-chain swap, Bungee, or Socket, or when a transaction is inferred to be bridge-related.

Match chains by displayed name or numeric chain ID. Treat any chain missing from the tables as outside this skill's local dataset. If the requested chain is not listed, use the web search tool to find authoritative metadata from the chain's official documentation or Chainlist before proceeding.

Also normalize common code aliases to their table rows, such as `mainnet` → Ethereum (`1`), `bsc` → BNB Chain (`56`), `coreDao` → Core Dao (`1116`), `hyperevm` → HyperEVM (`999`), and `zksyncSepolia` → ZKsync Sepolia Testnet (`300`).

## Querying On-Chain Data (Routing)

To read account data — native balance, token holdings, ERC-20/721/1155 transfers, transaction history, or first-funding — resolve the chain, then dispatch to the right explorer API. Do not default to Ethereum; infer the chain from the prompt (explicit chain mention, chain-specific tokens like POL→137 / ARB→42161, testnet keywords). If ambiguous, ask.

1. **Resolve the chain** — map the name → chain ID via the tables below (Mainnets, Testnets).
2. **Etherscan (preferred)** — if the chain ID is listed in `./references/etherscan-chains.md`, follow `./references/etherscan-api.md` (unified API V2, needs `$ETHERSCAN_API_KEY`).
3. **Blockscout (fallback)** — otherwise follow `./references/blockscout-api.md`. Blockscout/Chainscout indexes 1000+ chains Etherscan doesn't serve, with full token holdings on the free tier.
4. **Neither** — if the chain is in neither registry, query the public RPCs below directly over JSON-RPC (`cast` from the `cli-cast` skill, or `curl`). If the chain is non-EVM (Solana, Bitcoin, Cosmos, …), report that it is unsupported.

**Paid-chain auto-fallback.** Base (`8453`), OP (`10`), Avalanche (`43114`), and BNB (`56`) — plus their testnets `84532`, `11155420`, `43113`, `97` — are Etherscan-listed, but their data endpoints require a paid Etherscan plan. If the target is one of these **and** `./scripts/etherscan-detect-plan.sh` reports `paid_chains=false` (free tier), route to Blockscout instead: it serves them free, no key, with full holdings. Etherscan stays the default for every other chain.

## Bridge Transaction Enrichment (Bungee)

When the user mentions bridging, bridge tx, cross-chain swap, Bungee, or Socket, or when a transaction looks bridge-related from logs, counterparties, calldata, or token movement, read `./references/bungee-api.md` before answering. Use Bungee to enrich the analysis with origin/destination transaction context, route/bridge name, status code, timestamps, and refunds.

Keep Bungee as an enrichment source alongside Etherscan, Blockscout, explorers, and RPC receipts. Do not treat Bungee as authoritative for on-chain execution by itself; verify submitted transactions and terminal outcomes with explorer/RPC data whenever possible. If Bungee has no record or the public sandbox is rate-limited/unavailable, say so and continue normal on-chain analysis.

## RouteMesh

Use RouteMesh only when the `RouteMesh` column is `Yes` and the `ROUTEMESH_API_KEY` environment variable is available.

To verify current RouteMesh support, call `GET https://lb.routeme.sh/chains`; use `https://lb2.routeme.sh/chains` as the backup endpoint. Do not use `https://rpc.routeme.sh/chains`; the hostname may not resolve even though it appears in RouteMesh's OpenAPI spec.

Construct the RouteMesh RPC URL as:

```text
https://lb.routeme.sh/rpc/CHAIN_ID/ROUTEMESH_API_KEY
```

Replace `CHAIN_ID` with the numeric chain ID and `ROUTEMESH_API_KEY` with the value of the `ROUTEMESH_API_KEY` environment variable. If `RouteMesh` is `No` or `ROUTEMESH_API_KEY` is not available, use the chain's primary public RPC first, then the listed fallback RPCs in order.

## Public RPCs

Public RPCs are best-effort. Before relying on one for data fetches or contract calls, verify it with `eth_chainId`. If the primary endpoint fails, try the fallback endpoints for that chain from the fallback table. If a chain has no fallback row, only the primary public RPC is listed.

Do not use RPCs for Form or Meld. They are intentionally omitted from the active mainnet table because those chains are now defunct and no longer operating. If the user asks about Form or Meld, say that the chain is defunct/no longer operating instead of returning an RPC URL.

## Explorer URLs

The `Explorer URL` column is the base URL of the chain's canonical block explorer. Append standard path segments to build links:

| Resource    | Path              | Example                                  |
| ----------- | ----------------- | ---------------------------------------- |
| Address     | `/address/<addr>` | `https://arbiscan.io/address/0xabc...`   |
| Transaction | `/tx/<hash>`      | `https://etherscan.io/tx/0x123...`       |
| Block       | `/block/<number>` | `https://basescan.org/block/12345678`    |
| Token       | `/token/<addr>`   | `https://polygonscan.com/token/0xdef...` |

Etherscan and Etherscan-stack explorers (Arbiscan, Basescan, BscScan, Polygonscan, Optimism Etherscan, Lineascan, Snowscan, Blastscan, Berascan, Uniscan, Gnosisscan, abscan.org) all follow this scheme. Most Blockscout-based and chain-native explorers accept the same segments, but conventions can drift — verify against the explorer UI when in doubt.

Explorer URL presence, Etherscan-style paths, or an Etherscan-stack explorer name do not imply Etherscan API V2 support. When API coverage matters, use `./references/etherscan-chains.md` or Etherscan's live `https://api.etherscan.io/v2/chainlist` endpoint. Treat this table as chain metadata for RPC and explorer-link construction only.

## Caveats

**OP Mainnet pre-regenesis history is not available through current explorer/RPC routes.** For OP Mainnet (`10`) queries before the final regenesis on `2021-11-11`, read `./references/optimism-pre-2021-11-11.md` before using Etherscan, Blockscout, or public RPC results.

**Ronin (`app.roninchain.com`) does not follow the Etherscan path scheme.** Verify against the explorer UI before constructing a Ronin link.

**PulseChain's explorer (`scan.pulsechain.com`) is a Blockscout frontend; its JSON API is on a separate host.** Address, tx, and block links under `scan.pulsechain.com` resolve, but `scan.pulsechain.com/api/...` returns a frontend 404, and the Blockscout API backend at `api.scan.pulsechain.com` has been observed down (HTTP 502). Prefer PulseChain's listed RPC endpoints for programmatic data.

## Mainnets

| Chain name    | Chain ID | Primary public RPC                              | Native currency symbol | Explorer URL                      | RouteMesh |
| ------------- | -------- | ----------------------------------------------- | ---------------------- | --------------------------------- | --------- |
| Abstract      | 2741     | https://api.mainnet.abs.xyz                     | ETH                    | https://abscan.org                | Yes       |
| Arbitrum      | 42161    | https://arb1.arbitrum.io/rpc                    | ETH                    | https://arbiscan.io               | Yes       |
| Avalanche     | 43114    | https://api.avax.network/ext/bc/C/rpc           | AVAX                   | https://snowscan.xyz              | Yes       |
| Base          | 8453     | https://mainnet.base.org                        | ETH                    | https://basescan.org              | Yes       |
| Berachain     | 80094    | https://rpc.berachain.com                       | BERA                   | https://berascan.com              | Yes       |
| Blast         | 81457    | https://rpc.blast.io                            | ETH                    | https://blastscan.io              | Yes       |
| BNB Chain     | 56       | https://bsc-dataseed1.bnbchain.org              | BNB                    | https://bscscan.com               | Yes       |
| Celo          | 42220    | https://forno.celo.org                          | CELO                   | https://celoscan.io               | Yes       |
| Chiliz        | 88888    | https://rpc.chiliz.com                          | CHZ                    | https://chiliscan.com             | Yes       |
| Core Dao      | 1116     | https://rpc.coredao.org                         | CORE                   | https://scan.coredao.org          | Yes       |
| Denergy       | 369369   | https://rpc.d.energy                            | WATT                   | https://explorer.denergychain.com | No        |
| Dymension     | 1100     | https://dymension.api.onfinality.io/public      | DYM                    | https://dym.fyi                   | Yes       |
| Ethereum      | 1        | https://ethereum-rpc.publicnode.com             | ETH                    | https://etherscan.io              | Yes       |
| Fantom        | 250      | https://rpc.fantom.network                      | FTM                    | https://ftmscan.com               | Yes       |
| Gnosis        | 100      | https://rpc.gnosischain.com                     | XDAI                   | https://gnosisscan.io             | Yes       |
| HyperEVM      | 999      | https://rpc.hyperliquid.xyz/evm                 | HYPE                   | https://hyperevmscan.io           | Yes       |
| IoTeX         | 4689     | https://babel-api.mainnet.iotex.io              | IOTX                   | https://iotexscan.io              | Yes       |
| Lightlink     | 1890     | https://replicator.phoenix.lightlink.io/rpc/v1  | ETH                    | https://phoenix.lightlink.io      | Yes       |
| Linea Mainnet | 59144    | https://rpc.linea.build                         | ETH                    | https://lineascan.build           | Yes       |
| Mode          | 34443    | https://mainnet.mode.network                    | ETH                    | https://modescan.io               | Yes       |
| Monad         | 143      | https://rpc.monad.xyz                           | MON                    | https://monadscan.com             | Yes       |
| Morph         | 2818     | https://rpc.morphl2.io                          | ETH                    | https://explorer.morphl2.io       | Yes       |
| OP Mainnet    | 10       | https://mainnet.optimism.io                     | ETH                    | https://optimistic.etherscan.io   | Yes       |
| Plasma        | 9745     | https://rpc.plasma.to                           | XPL                    | https://plasmascan.to             | Yes       |
| Polygon       | 137      | https://polygon-bor-rpc.publicnode.com          | POL                    | https://polygonscan.com           | Yes       |
| PulseChain    | 369      | https://rpc.pulsechain.com                      | PLS                    | https://scan.pulsechain.com       | Yes       |
| Ronin         | 2020     | https://api.roninchain.com/rpc                  | RON                    | https://app.roninchain.com        | Yes       |
| Scroll        | 534352   | https://rpc.scroll.io                           | ETH                    | https://scrollscan.com            | Yes       |
| Sei Network   | 1329     | https://evm-rpc.sei-apis.com                    | SEI                    | https://seiscan.io                | Yes       |
| Sonic         | 146      | https://rpc.soniclabs.com                       | S                      | https://sonicscan.org             | Yes       |
| Sophon        | 50104    | https://rpc.sophon.xyz                          | SOPH                   | https://sophscan.xyz              | Yes       |
| Superseed     | 5330     | https://mainnet.superseed.xyz                   | ETH                    | https://explorer.superseed.xyz    | Yes       |
| Taiko         | 167000   | https://rpc.mainnet.taiko.xyz                   | ETH                    | https://taikoscan.io              | Yes       |
| Tangle        | 5845     | https://rpc.tangle.tools                        | TNT                    | https://explorer.tangle.tools     | Yes       |
| Unichain      | 130      | https://mainnet.unichain.org                    | ETH                    | https://uniscan.xyz               | Yes       |
| World Chain   | 480      | https://worldchain-mainnet.g.alchemy.com/public | ETH                    | https://worldscan.org             | Yes       |
| XDC           | 50       | https://rpc.xdcrpc.com                          | XDC                    | https://xdcscan.com               | Yes       |
| ZKsync Era    | 324      | https://mainnet.era.zksync.io                   | ETH                    | https://explorer.zksync.io        | Yes       |
| Zora          | 7777777  | https://zora.drpc.org                           | ETH                    | https://explorer.zora.energy      | No        |

## Mainnet Fallback RPCs

| Chain name    | Chain ID | Fallback public RPCs                                                                                           |
| ------------- | -------- | -------------------------------------------------------------------------------------------------------------- |
| Abstract      | 2741     | https://abstract.drpc.org, https://2741.rpc.thirdweb.com                                                       |
| Arbitrum      | 42161    | https://arbitrum-one-rpc.publicnode.com, https://42161.rpc.thirdweb.com, https://rpcfree.com/arbitrum-rpc      |
| Avalanche     | 43114    | https://avalanche-c-chain-rpc.publicnode.com, https://43114.rpc.thirdweb.com                                   |
| Base          | 8453     | https://base-rpc.publicnode.com, https://base.gateway.tenderly.co, https://developer-access-mainnet.base.org   |
| Berachain     | 80094    | https://berachain-rpc.publicnode.com, https://80094.rpc.thirdweb.com, https://rpc.berachain-apis.com           |
| Blast         | 81457    | https://blast-rpc.publicnode.com, https://blast.drpc.org, https://81457.rpc.thirdweb.com                       |
| BNB Chain     | 56       | https://bsc-rpc.publicnode.com, https://bsc.drpc.org, https://56.rpc.thirdweb.com                              |
| Celo          | 42220    | https://celo.drpc.org, https://celo-rpc.publicnode.com, https://42220.rpc.thirdweb.com                         |
| Chiliz        | 88888    | https://chiliz.publicnode.com, https://88888.rpc.thirdweb.com, https://rpc.ankr.com/chiliz                     |
| Core Dao      | 1116     | https://core.drpc.org, https://1116.rpc.thirdweb.com, https://rpc-core.icecreamswap.com                        |
| Dymension     | 1100     | https://dymension.drpc.org, https://1100.rpc.thirdweb.com, https://dymension-evm.blockpi.network/v1/rpc/public |
| Ethereum      | 1        | https://eth.drpc.org, https://rpc.flashbots.net, https://1.rpc.thirdweb.com                                    |
| Fantom        | 250      | https://rpc2.fantom.network                                                                                    |
| Gnosis        | 100      | https://gnosis-rpc.publicnode.com, https://gnosis.drpc.org, https://100.rpc.thirdweb.com                       |
| HyperEVM      | 999      | https://hyperliquid.drpc.org, https://999.rpc.thirdweb.com, https://gwan-ssl.wandevs.org:46891                 |
| IoTeX         | 4689     | https://4689.rpc.thirdweb.com                                                                                  |
| Lightlink     | 1890     | https://1890.rpc.thirdweb.com                                                                                  |
| Linea Mainnet | 59144    | https://linea-rpc.publicnode.com, https://59144.rpc.thirdweb.com                                               |
| Mode          | 34443    | https://mode.drpc.org, https://34443.rpc.thirdweb.com                                                          |
| Monad         | 143      | https://monad.drpc.org, https://143.rpc.thirdweb.com                                                           |
| Morph         | 2818     | https://morph.drpc.org, https://2818.rpc.thirdweb.com, https://rpc-quicknode.morphl2.io                        |
| OP Mainnet    | 10       | https://optimism-rpc.publicnode.com, https://optimism.drpc.org, https://10.rpc.thirdweb.com                    |
| Plasma        | 9745     | https://9745.rpc.thirdweb.com, https://plasma.drpc.org, https://plasma.gateway.tenderly.co                     |
| Polygon       | 137      | https://polygon.drpc.org, https://137.rpc.thirdweb.com, https://rpc-mainnet.matic.quiknode.pro                 |
| PulseChain    | 369      | https://pulsechain-rpc.publicnode.com, https://rpc-pulsechain.g4mm4.io, https://369.rpc.thirdweb.com           |
| Ronin         | 2020     | https://ronin.drpc.org, https://2020.rpc.thirdweb.com                                                          |
| Scroll        | 534352   | https://scroll.drpc.org, https://534352.rpc.thirdweb.com                                                       |
| Sei Network   | 1329     | https://sei.drpc.org, https://1329.rpc.thirdweb.com                                                            |
| Sonic         | 146      | https://sonic-rpc.publicnode.com, https://sonic.drpc.org, https://146.rpc.thirdweb.com                         |
| Sophon        | 50104    | https://50104.rpc.thirdweb.com                                                                                 |
| Superseed     | 5330     | https://superseed.drpc.org, https://5330.rpc.thirdweb.com                                                      |
| Taiko         | 167000   | https://167000.rpc.thirdweb.com                                                                                |
| Unichain      | 130      | https://unichain-rpc.publicnode.com, https://130.rpc.thirdweb.com                                              |
| World Chain   | 480      | https://worldchain.drpc.org, https://480.rpc.thirdweb.com                                                      |
| XDC           | 50       | https://50.rpc.thirdweb.com, https://erpc.xdcrpc.com, https://rpc.xdc.org                                      |
| ZKsync Era    | 324      | https://zksync.drpc.org, https://324.rpc.thirdweb.com                                                          |
| Zora          | 7777777  | https://7777777.rpc.thirdweb.com, https://rpc.zora.energy                                                      |

## Testnets

| Chain name             | Chain ID | Primary public RPC                          | Native currency symbol | Explorer URL                             | RouteMesh |
| ---------------------- | -------- | ------------------------------------------- | ---------------------- | ---------------------------------------- | --------- |
| Arbitrum Sepolia       | 421614   | https://sepolia-rollup.arbitrum.io/rpc      | ETH                    | https://sepolia.arbiscan.io              | Yes       |
| Base Sepolia           | 84532    | https://sepolia.base.org                    | ETH                    | https://sepolia.basescan.org             | Yes       |
| BattleChain Testnet    | 627      | https://testnet.battlechain.com             | ETH                    | https://explorer.testnet.battlechain.com | No        |
| Ethereum Sepolia       | 11155111 | https://ethereum-sepolia-rpc.publicnode.com | ETH                    | https://sepolia.etherscan.io             | Yes       |
| Linea Sepolia          | 59141    | https://rpc.sepolia.linea.build             | ETH                    | https://sepolia.lineascan.build          | Yes       |
| OP Sepolia             | 11155420 | https://sepolia.optimism.io                 | ETH                    | https://optimism-sepolia.blockscout.com  | Yes       |
| Superseed Sepolia      | 53302    | https://sepolia.superseed.xyz               | ETH                    | https://sepolia-explorer.superseed.xyz   | No        |
| ZKsync Sepolia Testnet | 300      | https://sepolia.era.zksync.dev              | ETH                    | https://sepolia.explorer.zksync.io       | Yes       |

## Testnet Fallback RPCs

| Chain name             | Chain ID | Fallback public RPCs                                                                            |
| ---------------------- | -------- | ----------------------------------------------------------------------------------------------- |
| Arbitrum Sepolia       | 421614   | https://arbitrum-sepolia.drpc.org, https://421614.rpc.thirdweb.com                              |
| Base Sepolia           | 84532    | https://base-sepolia-rpc.publicnode.com, https://84532.rpc.thirdweb.com                         |
| Ethereum Sepolia       | 11155111 | https://11155111.rpc.thirdweb.com, https://sepolia.drpc.org, https://rpc.sepolia.ethpandaops.io |
| Linea Sepolia          | 59141    | https://59141.rpc.thirdweb.com                                                                  |
| OP Sepolia             | 11155420 | https://optimism-sepolia-rpc.publicnode.com, https://11155420.rpc.thirdweb.com                  |
| Superseed Sepolia      | 53302    | https://53302.rpc.thirdweb.com                                                                  |
| ZKsync Sepolia Testnet | 300      | https://300.rpc.thirdweb.com                                                                    |
