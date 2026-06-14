# Target Chains & Chainscout

Blockscout and Chainscout index many EVM networks, but this skill only uses them for chains in `SKILL.md` [Target Mainnets](../SKILL.md#target-mainnets). Do not use Chainscout to expand scope. If a requested chain is not in the target table, ask the user to file a feature request in <https://github.com/PaulRBerg/agent-skills>.

## Chainscout API

No API key required. Live UI: <https://chains.blockscout.com/>

| Endpoint                                            | Returns                                               |
| --------------------------------------------------- | ----------------------------------------------------- |
| `GET https://chains.blockscout.com/api/chains`      | Object keyed by `chain_id` -> metadata (all networks) |
| `GET https://chains.blockscout.com/api/chains/{id}` | Metadata for one chain                                |

Single-chain response shape:

```json
{
  "name": "Gnosis",
  "native_currency": "XDAI",
  "isTestnet": false,
  "layer": 1,
  "rollupType": null,
  "ecosystem": "Ethereum",
  "explorers": [{ "url": "https://gnosis.blockscout.com/", "hostedBy": "blockscout" }]
}
```

`explorers[].hostedBy`:

- `blockscout` — Blockscout-operated; candidate for the unified PRO host (`api.blockscout.com/{chain_id}/...`). Still confirm with a live call — not every hosted target chain is fronted by the PRO host.
- anything else — community-operated. Per-instance only; use `explorers[].url` directly (no key).

Use `./scripts/resolve-chain.sh <chain_id>` to extract these fields as `key=value` lines. The helper refuses non-target chain IDs and rejects Chainscout responses whose name does not match the target chain. For **name -> `chain_id`**, use the target table in `SKILL.md` first.

## Target Chains Observed in Chainscout

Observed on 2026-06-14. Presence here does not override the target table's canonical explorer/RPC metadata.

| Chain      | `chain_id` | Native | Hosted by  | Instance URL                     | Notes                             |
| ---------- | ---------- | ------ | ---------- | -------------------------------- | --------------------------------- |
| Arbitrum   | `42161`    | ETH    | blockscout | https://arbitrum.blockscout.com/ |                                   |
| Base       | `8453`     | ETH    | blockscout | https://base.blockscout.com/     |                                   |
| Celo       | `42220`    | CELO   | blockscout | https://celo.blockscout.com/     |                                   |
| Ethereum   | `1`        | ETH    | blockscout | https://eth.blockscout.com/      |                                   |
| Fantom     | `250`      | FTM    | self       | https://ftmscout.com/            |                                   |
| Gnosis     | `100`      | XDAI   | blockscout | https://gnosis.blockscout.com/   |                                   |
| HyperEVM   | `999`      | HYPE   | self       | https://www.hyperscan.com/       | Chainscout marks `isTestnet=true` |
| Lightlink  | `1890`     | ETH    | blockscout | https://phoenix.lightlink.io/    |                                   |
| Linea      | `59144`    | ETH    | self       | https://explorer.linea.build/    |                                   |
| Mode       | `34443`    | ETH    | blockscout | https://explorer.mode.network/   |                                   |
| Morph      | `2818`     | ETH    | self       | https://explorer.morphl2.io/     |                                   |
| Optimism   | `10`       | ETH    | blockscout | https://explorer.optimism.io/    |                                   |
| Polygon    | `137`      | POL    | blockscout | https://polygon.blockscout.com/  |                                   |
| Scroll     | `534352`   | ETH    | blockscout | https://scroll.blockscout.com    |                                   |
| Superseed  | `5330`     | ETH    | self       | https://explorer.superseed.xyz/  |                                   |
| Unichain   | `130`      | ETH    | blockscout | https://unichain.blockscout.com  |                                   |
| ZKsync Era | `324`      | ETH    | blockscout | https://zksync.blockscout.com/   |                                   |

## Target Chains Absent or Unsafe in Chainscout

Use Etherscan or the target table's public RPC/explorer for these.

| Chain     | `chain_id` | Notes                                             |
| --------- | ---------- | ------------------------------------------------- |
| Abstract  | `2741`     | Not returned by Chainscout                        |
| Avalanche | `43114`    | Not returned by Chainscout                        |
| Berachain | `80094`    | Not returned by Chainscout                        |
| Blast     | `81457`    | Not returned by Chainscout                        |
| BNB Chain | `56`       | Not returned by Chainscout                        |
| Chiliz    | `88888`    | Not returned by Chainscout                        |
| Core Dao  | `1116`     | Not returned by Chainscout                        |
| IoTeX     | `4689`     | Not returned by Chainscout                        |
| Monad     | `143`      | Not returned by Chainscout                        |
| Ronin     | `2020`     | Chainscout returns a different network for `2020` |
| Sei       | `1329`     | Not returned by Chainscout                        |
| Sonic     | `146`      | Not returned by Chainscout                        |
| Sophon    | `50104`    | Not returned by Chainscout                        |
| XDC       | `50`       | Not returned by Chainscout                        |
| Zora      | `7777777`  | Not returned by Chainscout                        |

## Contributing

Missing or wrong Blockscout registry data is fixed via PR to <https://github.com/blockscout/chainscout>. Requests to add non-target chains to this skill belong in <https://github.com/PaulRBerg/agent-skills>.
