# Etherscan Supported Target Chains Reference

Source: <https://docs.etherscan.io/supported-chains>
Live list: <https://api.etherscan.io/v2/chainlist> (returns the provider's authoritative `chainid` set)

Verified against the live `chainlist` endpoint on 2026-06-14 (64 provider chains). This file intentionally lists only chains from the `SKILL.md` target list; provider-supported chains outside that list are out of scope for this skill.

## Target Mainnets (Free Tier Available)

| Chain     | Chain ID | Notes         |
| --------- | -------- | ------------- |
| Abstract  | `2741`   |               |
| Arbitrum  | `42161`  |               |
| Berachain | `80094`  |               |
| Blast     | `81457`  |               |
| Celo      | `42220`  |               |
| Ethereum  | `1`      | Default chain |
| Gnosis    | `100`    |               |
| HyperEVM  | `999`    |               |
| Linea     | `59144`  |               |
| Monad     | `143`    |               |
| Polygon   | `137`    |               |
| Sei       | `1329`   |               |
| Sonic     | `146`    |               |
| Unichain  | `130`    |               |
| XDC       | `50`     |               |

## Target Mainnets (Paid Plan Required)

The following target chains require a paid Etherscan plan for data endpoints (balances, transactions, logs, etc.). **Lite ($49/mo) is sufficient**; `module=contract` endpoints (`getsourcecode`, `getabi`, etc.) work on every plan.

| Chain     | Chain ID |
| --------- | -------- |
| Avalanche | `43114`  |
| Base      | `8453`   |
| BNB Chain | `56`     |
| Optimism  | `10`     |

## Target Mainnets Not on Etherscan V2

Route these target chains to Blockscout when available, then to the target table's public RPC if needed. Do not query Etherscan V2 for them.

| Chain      | Chain ID  | Notes                                   |
| ---------- | --------- | --------------------------------------- |
| Chiliz     | `88888`   | Not returned by the live chainlist      |
| Core Dao   | `1116`    | Not returned by the live chainlist      |
| Fantom     | `250`     | Not returned by the live chainlist      |
| IoTeX      | `4689`    | Not returned by the live chainlist      |
| Lightlink  | `1890`    | Not returned by the live chainlist      |
| Mode       | `34443`   | Not returned by the live chainlist      |
| Morph      | `2818`    | Not returned by the live chainlist      |
| Ronin      | `2020`    | Not returned by the live chainlist      |
| Scroll     | `534352`  | Removed from Etherscan V2 on 2026-04-16 |
| Sophon     | `50104`   | Not returned by the live chainlist      |
| Superseed  | `5330`    | Not returned by the live chainlist      |
| ZKsync Era | `324`     | Not currently supported by Etherscan V2 |
| Zora       | `7777777` | Not returned by the live chainlist      |

Testnets are outside this skill's target list. If the user asks about any non-target chain, ask them to file a feature request in <https://github.com/PaulRBerg/agent-skills>.
