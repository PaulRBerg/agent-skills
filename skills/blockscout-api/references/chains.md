# Chains & Chainscout

Blockscout indexes 1000+ EVM networks. There is no static master list to hardcode — **Chainscout** is the live registry that maps `chain_id` → instance URL and metadata.

## Chainscout API

No API key required. Live UI: <https://chains.blockscout.com/>

| Endpoint                                            | Returns                                              |
| --------------------------------------------------- | ---------------------------------------------------- |
| `GET https://chains.blockscout.com/api/chains`      | Object keyed by `chain_id` → metadata (all networks) |
| `GET https://chains.blockscout.com/api/chains/{id}` | Metadata for one chain                               |

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

- `blockscout` — Blockscout-operated; candidate for the unified PRO host (`api.blockscout.com/{chain_id}/…`). Still confirm with a live call — not every hosted chain is fronted by the PRO host.
- anything else — community-operated. Per-instance only; use `explorers[].url` directly (no key).

Use `./scripts/resolve-chain.sh <chain_id>` to extract these fields as `key=value` lines. For **name → `chain_id`**, use the `evm-chains` skill first.

## Common Chains

`PRO host` column: ✓ = verified `200` on `api.blockscout.com/{id}/api/v2/...`; ✗ = `404` on the PRO host but present in Chainscout (per-instance only).

| Chain        | `chain_id` | Native | PRO host | Notes                           |
| ------------ | ---------- | ------ | -------- | ------------------------------- |
| Ethereum     | `1`        | ETH    | ✓        | `hostedBy: blockscout`          |
| OP Mainnet   | `10`       | ETH    | ✓        |                                 |
| Polygon      | `137`      | POL    | ✓        |                                 |
| Base         | `8453`     | ETH    | ✓        |                                 |
| Arbitrum One | `42161`    | ETH    | ✓        |                                 |
| Gnosis       | `100`      | XDAI   | ✓        |                                 |
| Flare        | `14`       | FLR    | ✗        | in Chainscout, `hostedBy: self` |

For any chain not listed, resolve it through Chainscout rather than guessing the instance hostname. When the PRO host returns `404` but Chainscout has the chain, use its per-instance URL.

Chains absent from the PRO host **and** Chainscout (e.g., BNB `56`, Kaia `8217`, Abstract `2741` at time of writing) are not in the Blockscout registry — use another explorer such as the `etherscan-api` skill for those.

## Contributing

Missing or wrong chain data is fixed via PR to <https://github.com/blockscout/chainscout>.
