---
argument-hint: <chain-name-or-id>
disable-model-invocation: false
name: evm-chains
user-invocable: true
description: 'Use for targeted EVM chain resolution and on-chain account data: target chain name/ID, RPC/explorer/native symbol, RouteMesh, balances, token/NFT holdings/transfers, tx history, funding origin via Etherscan/Blockscout/Chainscout; bridge/cross-chain Bungee/Socket enrichment.'
---

# EVM Chains

Targeted EVM chain dataset (chain name, chain ID, public RPCs, native currency symbol, default block explorer URL, RouteMesh support) **and** a router for reading on-chain data: resolve the chain, then dispatch balance, token, transfer, transaction, and first-funding queries to Etherscan (preferred) or Blockscout (fallback). For bridge transactions and cross-chain swaps, enrich explorer/RPC verification with Bungee status data only when the relevant chains are target chains.

Use this skill to resolve chain metadata before reading from an RPC, sending transactions, calling contracts, constructing chain-specific RPC URLs, or building explorer links to addresses, transactions, or blocks — and to query on-chain account data once the chain is resolved (see [Querying On-Chain Data (Routing)](#querying-on-chain-data-routing)). Also use it when the user mentions bridging, bridge tx, cross-chain swap, Bungee, or Socket, or when a transaction is inferred to be bridge-related.

Match chains by displayed name or numeric chain ID. The [Target Mainnets](#target-mainnets) table is the skill's authoritative scope. If the requested chain is not listed there, do not use Etherscan, Blockscout, Bungee, RouteMesh, Chainlist, web search, or public RPCs to work around the scope. Ask the user to file a feature request in <https://github.com/PaulRBerg/agent-skills>.

Also normalize common code aliases to their table rows, such as `mainnet` → Ethereum (`1`), `bsc` → BNB Chain (`56`), `coreDao` → Core Dao (`1116`), `hyperevm` → HyperEVM (`999`), `op` → Optimism (`10`), and `zksync` → ZKsync Era (`324`).

## Querying On-Chain Data (Routing)

To read account data — native balance, token holdings, ERC-20/721/1155 transfers, transaction history, or first-funding — resolve the chain, then dispatch to the right explorer API. Do not default to Ethereum; infer the chain from the prompt (explicit chain mention, chain-specific tokens like POL→137 / ARB→42161, target-chain aliases). If ambiguous, ask. If the resolved chain is not a target chain, ask for a feature request instead of querying.

1. **Resolve the chain** — map the name → chain ID via [Target Mainnets](#target-mainnets).
2. **Etherscan (preferred)** — if the chain ID is listed in `./references/etherscan-chains.md`, follow `./references/etherscan-api.md` (unified API V2, needs `$ETHERSCAN_API_KEY`).
3. **Blockscout (fallback)** — for target chains not served by Etherscan, or paid Etherscan chains on a free Etherscan plan, follow `./references/blockscout-api.md`.
4. **Neither** — if the target chain is in neither registry, query the target chain's public RPC directly over JSON-RPC (`cast` from the `cli-cast` skill, or `curl`). If the chain is not a target chain, ask for a feature request instead of querying.

**Paid-chain auto-fallback.** Base (`8453`), Optimism (`10`), Avalanche (`43114`), and BNB Chain (`56`) are Etherscan-listed target chains, but their data endpoints require a paid Etherscan plan. If the target is one of these **and** `./scripts/etherscan-detect-plan.sh` reports `paid_chains=false` (free tier), route to Blockscout instead. Etherscan stays the default for every other Etherscan-listed target chain.

## Bridge Transaction Enrichment (Bungee)

When the user mentions bridging, bridge tx, cross-chain swap, Bungee, or Socket, or when a transaction looks bridge-related from logs, counterparties, calldata, or token movement, first confirm the known origin and destination chains are target chains. If either known chain is outside [Target Mainnets](#target-mainnets), ask for a feature request instead of continuing. Otherwise, read `./references/bungee-api.md` before answering. Use Bungee to enrich the analysis with origin/destination transaction context, route/bridge name, status code, timestamps, and refunds.

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

Only use RPCs for target chains. For any chain not in [Target Mainnets](#target-mainnets), ask the user to file a feature request in <https://github.com/PaulRBerg/agent-skills>.

## Explorer URLs

The `Explorer URL` column is the base URL of the chain's canonical block explorer. Append standard path segments to build links:

| Resource    | Path              | Example                                  |
| ----------- | ----------------- | ---------------------------------------- |
| Address     | `/address/<addr>` | `https://arbiscan.io/address/0xabc...`   |
| Transaction | `/tx/<hash>`      | `https://etherscan.io/tx/0x123...`       |
| Block       | `/block/<number>` | `https://basescan.org/block/12345678`    |
| Token       | `/token/<addr>`   | `https://polygonscan.com/token/0xdef...` |

Etherscan and Etherscan-stack explorers (Arbiscan, Basescan, BscScan, Polygonscan, Optimism Etherscan, Lineascan, Snowscan, Blastscan, Berascan, Uniscan, Gnosisscan, abscan.org) all follow this scheme. Most Blockscout-based and chain-native target explorers accept the same segments, but conventions can drift — verify against the explorer UI when in doubt.

Explorer URL presence, Etherscan-style paths, or an Etherscan-stack explorer name do not imply Etherscan API V2 support. When API coverage matters, use `./references/etherscan-chains.md`. Etherscan's live `https://api.etherscan.io/v2/chainlist` endpoint may contain additional provider-supported chains, but those are outside this skill unless they appear in [Target Mainnets](#target-mainnets). Treat this table as chain metadata for RPC and explorer-link construction only.

## Caveats

**OP Mainnet pre-regenesis history is not available through current explorer/RPC routes.** For OP Mainnet (`10`) queries before the final regenesis on `2021-11-11`, read `./references/optimism-pre-2021-11-11.md` before using Etherscan, Blockscout, or public RPC results.

**Ronin (`app.roninchain.com`) does not follow the Etherscan path scheme.** Verify against the explorer UI before constructing a Ronin link.

**Ronin (`2020`) collides with a non-target Chainscout registry entry.** Do not use Chainscout metadata for Ronin; use Ronin's listed explorer/RPC or another target-aware source.

## Target Mainnets

The `Slug` column is the Sablier SDK chain slug (`sablier` package `~/sablier/sdk`, `src/evm/chains/specs.ts`, resolved as `meta.slug ?? key`). Arbitrum Nova, Celo, Fantom, IoTeX, and Zora are not defined in the SDK; their slugs follow the same convention (lowercase, hyphenated name).

| Chain name    | Chain ID | Slug          | Primary public RPC                             | Native currency symbol | Explorer URL                         | RouteMesh |
| ------------- | -------- | ------------- | ---------------------------------------------- | ---------------------- | ------------------------------------ | --------- |
| Abstract      | 2741     | abstract      | https://api.mainnet.abs.xyz                    | ETH                    | https://abscan.org                   | Yes       |
| Arbitrum      | 42161    | arbitrum      | https://arb1.arbitrum.io/rpc                   | ETH                    | https://arbiscan.io                  | Yes       |
| Arbitrum Nova | 42170    | arbitrum-nova | https://nova.arbitrum.io/rpc                   | ETH                    | https://arbitrum-nova.blockscout.com | Yes       |
| Avalanche     | 43114    | avalanche     | https://api.avax.network/ext/bc/C/rpc          | AVAX                   | https://snowscan.xyz                 | Yes       |
| Base          | 8453     | base          | https://mainnet.base.org                       | ETH                    | https://basescan.org                 | Yes       |
| Berachain     | 80094    | berachain     | https://rpc.berachain.com                      | BERA                   | https://berascan.com                 | Yes       |
| Blast         | 81457    | blast         | https://rpc.blast.io                           | ETH                    | https://blastscan.io                 | Yes       |
| BNB Chain     | 56       | bsc           | https://bsc-dataseed1.bnbchain.org             | BNB                    | https://bscscan.com                  | Yes       |
| Celo          | 42220    | celo          | https://forno.celo.org                         | CELO                   | https://celoscan.io                  | Yes       |
| Chiliz        | 88888    | chiliz        | https://rpc.chiliz.com                         | CHZ                    | https://chiliscan.com                | Yes       |
| Core Dao      | 1116     | core-dao      | https://rpc.coredao.org                        | CORE                   | https://scan.coredao.org             | Yes       |
| Ethereum      | 1        | mainnet       | https://ethereum-rpc.publicnode.com            | ETH                    | https://etherscan.io                 | Yes       |
| Fantom        | 250      | fantom        | https://rpc.fantom.network                     | FTM                    | https://ftmscan.com                  | Yes       |
| Gnosis        | 100      | gnosis        | https://rpc.gnosischain.com                    | XDAI                   | https://gnosisscan.io                | Yes       |
| HyperEVM      | 999      | hyperevm      | https://rpc.hyperliquid.xyz/evm                | HYPE                   | https://hyperevmscan.io              | Yes       |
| IoTeX         | 4689     | iotex         | https://babel-api.mainnet.iotex.io             | IOTX                   | https://iotexscan.io                 | Yes       |
| Lightlink     | 1890     | lightlink     | https://replicator.phoenix.lightlink.io/rpc/v1 | ETH                    | https://phoenix.lightlink.io         | Yes       |
| Linea         | 59144    | linea         | https://rpc.linea.build                        | ETH                    | https://lineascan.build              | Yes       |
| Mode          | 34443    | mode          | https://mainnet.mode.network                   | ETH                    | https://modescan.io                  | Yes       |
| Monad         | 143      | monad         | https://rpc.monad.xyz                          | MON                    | https://monadscan.com                | Yes       |
| Morph         | 2818     | morph         | https://rpc.morphl2.io                         | ETH                    | https://explorer.morphl2.io          | Yes       |
| Optimism      | 10       | optimism      | https://mainnet.optimism.io                    | ETH                    | https://optimistic.etherscan.io      | Yes       |
| Polygon       | 137      | polygon       | https://polygon-bor-rpc.publicnode.com         | POL                    | https://polygonscan.com              | Yes       |
| Ronin         | 2020     | ronin         | https://api.roninchain.com/rpc                 | RON                    | https://app.roninchain.com           | Yes       |
| Scroll        | 534352   | scroll        | https://rpc.scroll.io                          | ETH                    | https://scrollscan.com               | Yes       |
| Sei           | 1329     | sei           | https://evm-rpc.sei-apis.com                   | SEI                    | https://seiscan.io                   | Yes       |
| Sonic         | 146      | sonic         | https://rpc.soniclabs.com                      | S                      | https://sonicscan.org                | Yes       |
| Sophon        | 50104    | sophon        | https://rpc.sophon.xyz                         | SOPH                   | https://sophscan.xyz                 | Yes       |
| Superseed     | 5330     | superseed     | https://mainnet.superseed.xyz                  | ETH                    | https://explorer.superseed.xyz       | Yes       |
| Unichain      | 130      | unichain      | https://mainnet.unichain.org                   | ETH                    | https://uniscan.xyz                  | Yes       |
| XDC           | 50       | xdc           | https://rpc.xdcrpc.com                         | XDC                    | https://xdcscan.com                  | Yes       |
| ZKsync Era    | 324      | zksync        | https://mainnet.era.zksync.io                  | ETH                    | https://explorer.zksync.io           | Yes       |
| Zora          | 7777777  | zora          | https://zora.drpc.org                          | ETH                    | https://explorer.zora.energy         | No        |

## Target Fallback RPCs

| Chain name    | Chain ID | Fallback public RPCs                                                                                         |
| ------------- | -------- | ------------------------------------------------------------------------------------------------------------ |
| Abstract      | 2741     | https://abstract.drpc.org, https://2741.rpc.thirdweb.com                                                     |
| Arbitrum      | 42161    | https://arbitrum-one-rpc.publicnode.com, https://42161.rpc.thirdweb.com, https://rpcfree.com/arbitrum-rpc    |
| Arbitrum Nova | 42170    | https://arbitrum-nova-rpc.publicnode.com, https://42170.rpc.thirdweb.com, https://arbitrum-nova.drpc.org     |
| Avalanche     | 43114    | https://avalanche-c-chain-rpc.publicnode.com, https://43114.rpc.thirdweb.com                                 |
| Base          | 8453     | https://base-rpc.publicnode.com, https://base.gateway.tenderly.co, https://developer-access-mainnet.base.org |
| Berachain     | 80094    | https://berachain-rpc.publicnode.com, https://80094.rpc.thirdweb.com, https://rpc.berachain-apis.com         |
| Blast         | 81457    | https://blast-rpc.publicnode.com, https://blast.drpc.org, https://81457.rpc.thirdweb.com                     |
| BNB Chain     | 56       | https://bsc-rpc.publicnode.com, https://bsc.drpc.org, https://56.rpc.thirdweb.com                            |
| Celo          | 42220    | https://celo.drpc.org, https://celo-rpc.publicnode.com, https://42220.rpc.thirdweb.com                       |
| Chiliz        | 88888    | https://chiliz.publicnode.com, https://88888.rpc.thirdweb.com, https://rpc.ankr.com/chiliz                   |
| Core Dao      | 1116     | https://core.drpc.org, https://1116.rpc.thirdweb.com, https://rpc-core.icecreamswap.com                      |
| Ethereum      | 1        | https://eth.drpc.org, https://rpc.flashbots.net, https://1.rpc.thirdweb.com                                  |
| Fantom        | 250      | https://rpc2.fantom.network                                                                                  |
| Gnosis        | 100      | https://gnosis-rpc.publicnode.com, https://gnosis.drpc.org, https://100.rpc.thirdweb.com                     |
| HyperEVM      | 999      | https://hyperliquid.drpc.org, https://999.rpc.thirdweb.com, https://gwan-ssl.wandevs.org:46891               |
| IoTeX         | 4689     | https://4689.rpc.thirdweb.com                                                                                |
| Lightlink     | 1890     | https://1890.rpc.thirdweb.com                                                                                |
| Linea         | 59144    | https://linea-rpc.publicnode.com, https://59144.rpc.thirdweb.com                                             |
| Mode          | 34443    | https://mode.drpc.org, https://34443.rpc.thirdweb.com                                                        |
| Monad         | 143      | https://monad.drpc.org, https://143.rpc.thirdweb.com                                                         |
| Morph         | 2818     | https://morph.drpc.org, https://2818.rpc.thirdweb.com, https://rpc-quicknode.morphl2.io                      |
| Optimism      | 10       | https://optimism-rpc.publicnode.com, https://optimism.drpc.org, https://10.rpc.thirdweb.com                  |
| Polygon       | 137      | https://polygon.drpc.org, https://137.rpc.thirdweb.com, https://rpc-mainnet.matic.quiknode.pro               |
| Ronin         | 2020     | https://ronin.drpc.org, https://2020.rpc.thirdweb.com                                                        |
| Scroll        | 534352   | https://scroll.drpc.org, https://534352.rpc.thirdweb.com                                                     |
| Sei           | 1329     | https://sei.drpc.org, https://1329.rpc.thirdweb.com                                                          |
| Sonic         | 146      | https://sonic-rpc.publicnode.com, https://sonic.drpc.org, https://146.rpc.thirdweb.com                       |
| Sophon        | 50104    | https://50104.rpc.thirdweb.com                                                                               |
| Superseed     | 5330     | https://superseed.drpc.org, https://5330.rpc.thirdweb.com                                                    |
| Unichain      | 130      | https://unichain-rpc.publicnode.com, https://130.rpc.thirdweb.com                                            |
| XDC           | 50       | https://50.rpc.thirdweb.com, https://erpc.xdcrpc.com, https://rpc.xdc.org                                    |
| ZKsync Era    | 324      | https://zksync.drpc.org, https://324.rpc.thirdweb.com                                                        |
| Zora          | 7777777  | https://7777777.rpc.thirdweb.com, https://rpc.zora.energy                                                    |
