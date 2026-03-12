---
argument-hint: <chain-name-or-id>
disable-model-invocation: false
name: evm-chains
user-invocable: true
description: This skill should be used when the user asks to resolve an EVM chain name or chain ID, find the default public RPC or native currency symbol for a Sablier SDK chain, or check whether a Sablier SDK chain is supported by RouteMesh.
---

# EVM Chains

Authoritative Sablier-SDK-backed EVM chain reference for chain name, chain ID, default public RPC, native currency symbol, and RouteMesh support lookups.

Match chains by displayed name or numeric chain ID. Treat any chain missing from the table as outside this skill's supported dataset. RouteMesh support is derived from the built Sablier SDK chain object's `rpc.routemesh` availability, not from `cli-cast`'s shorter fallback reference.

| Chain name | Chain ID | Default public RPC | Native currency symbol | RouteMesh |
| ---------- | -------- | ------------------ | ---------------------- | --------- |
| Abstract | 2741 | https://api.mainnet.abs.xyz | ETH | Yes |
| Arbitrum | 42161 | https://arb1.arbitrum.io/rpc | ETH | Yes |
| Arbitrum Sepolia | 421614 | https://sepolia-rollup.arbitrum.io/rpc | ETH | Yes |
| Avalanche | 43114 | https://api.avax.network/ext/bc/C/rpc | AVAX | Yes |
| Base | 8453 | https://mainnet.base.org | ETH | Yes |
| Berachain | 80094 | https://rpc.berachain.com | BERA | Yes |
| Blast | 81457 | https://rpc.blast.io | ETH | Yes |
| Blast Sepolia | 168587773 | https://sepolia.blast.io | ETH | Yes |
| BNB Chain | 56 | https://56.rpc.thirdweb.com | BNB | Yes |
| Chiliz | 88888 | https://rpc.chiliz.com | CHZ | Yes |
| Core Dao | 1116 | https://rpc.coredao.org | CORE | Yes |
| Ethereum | 1 | https://eth.merkle.io | ETH | Yes |
| Form | 478 | https://rpc.form.network/http | ETH | No |
| Gnosis | 100 | https://rpc.gnosischain.com | XDAI | Yes |
| HyperEVM | 999 | https://rpc.hyperliquid.xyz/evm | HYPE | Yes |
| IoTeX | 4689 | https://babel-api.mainnet.iotex.io | IOTX | Yes |
| Lightlink | 1890 | https://replicator.phoenix.lightlink.io/rpc/v1 | ETH | Yes |
| Linea Mainnet | 59144 | https://rpc.linea.build | ETH | Yes |
| Linea Sepolia | 59141 | https://rpc.sepolia.linea.build | ETH | Yes |
| Meld | 333000333 | https://rpc-1.meld.com | MELD | No |
| Mode | 34443 | https://mainnet.mode.network | ETH | Yes |
| Mode Testnet | 919 | https://sepolia.mode.network | ETH | Yes |
| Monad | 143 | https://rpc.monad.xyz | MON | Yes |
| Morph | 2818 | https://rpc.morphl2.io | ETH | Yes |
| OP Mainnet | 10 | https://mainnet.optimism.io | ETH | Yes |
| OP Sepolia | 11155420 | https://sepolia.optimism.io | ETH | Yes |
| Polygon | 137 | https://polygon-rpc.com | POL | Yes |
| Ronin | 2020 | https://api.roninchain.com/rpc | RON | Yes |
| Scroll | 534352 | https://rpc.scroll.io | ETH | Yes |
| Sei Network | 1329 | https://evm-rpc.sei-apis.com | SEI | Yes |
| Sepolia | 11155111 | https://11155111.rpc.thirdweb.com | ETH | Yes |
| Sonic | 146 | https://rpc.soniclabs.com | S | Yes |
| Sophon | 50104 | https://rpc.sophon.xyz | SOPH | Yes |
| Superseed | 5330 | https://mainnet.superseed.xyz | ETH | Yes |
| Superseed Sepolia | 53302 | https://sepolia.superseed.xyz | ETH | Yes |
| Tangle | 5845 | https://rpc.tangle.tools | TNT | Yes |
| Unichain | 130 | https://mainnet.unichain.org/ | ETH | Yes |
| XDC | 50 | https://rpc.xdcrpc.com | XDC | Yes |
| ZKsync Era | 324 | https://mainnet.era.zksync.io | ETH | Yes |
| ZKsync Sepolia Testnet | 300 | https://sepolia.era.zksync.dev | ETH | Yes |
