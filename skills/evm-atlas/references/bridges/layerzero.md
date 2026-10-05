# LayerZero Bridging

## Overview

Use LayerZero as a read-only source for bridge route discovery, supported-token discovery, and deployment metadata on
target chains. This reference covers unauthenticated LayerZero Value Transfer API (VTA) discovery endpoints only.

Default to the VTA base URL:

```text
https://transfer.layerzero-api.com/v1
```

Do not use authenticated VTA transfer endpoints from this skill. Do not request API keys, build user steps, submit
signatures, approve spenders, or broadcast transactions. If the user provides transaction hashes or LayerZeroScan links,
verify them with explorer/RPC data instead.

## Legacy Stargate API

The Stargate API at `https://stargate.finance/api/v1` is deprecated in favor of LayerZero VTA. Use Stargate docs only as
legacy context when analyzing older integrations or URLs.

Legacy discovery endpoints:

| Endpoint      | Use                                           |
| ------------- | --------------------------------------------- |
| `GET /chains` | List legacy Stargate chain keys and chain IDs |
| `GET /tokens` | List legacy Stargate bridgeable tokens        |

Do not prefer legacy Stargate endpoints for new data unless the user specifically asks about Stargate API behavior.

## Read-Only Router

Use this router after confirming the known origin and destination chains against
`references/generated/target-mainnets.json`.

1. **Supported chains:** call `GET /chains`.
2. **Supported tokens or destinations:** call `GET /tokens`. Use `transferrableFromChainKey` and
   `transferrableFromTokenAddress` together to find valid destination tokens from a source token.
3. **Deployment metadata:** call `GET /metadata` when contract addresses or deployment details matter.
4. **Known transaction hash or LayerZeroScan link:** verify the referenced transaction on-chain with explorer/RPC data.

Example discovery:

```bash
curl -sS "https://transfer.layerzero-api.com/v1/chains"
curl -sS "https://transfer.layerzero-api.com/v1/tokens?transferrableFromChainKey=base&transferrableFromTokenAddress=0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"
curl -sS "https://transfer.layerzero-api.com/v1/metadata?chainKey=base"
```

## Report Fields

When present, extract these fields. Report the extracted fields:

| Field          | VTA path examples                                             |
| -------------- | ------------------------------------------------------------- |
| Chain identity | `chains[].chainKey`, `chains[].chainId`, `chains[].chainType` |
| Token identity | `tokens[].chainKey`, `tokens[].address`, `symbol`, `decimals` |
| Token routes   | `tokens[].transferableTo[]`, token route metadata             |
| Deployments    | metadata contract addresses and deployment labels             |

Token amounts, if present in supplied external data, are raw integer units. When token decimals are present, convert
with them. When decimals are absent, preserve raw values.

## Route Types

Common LayerZero route types and integrations to recognize in logs, explorer labels, or supplied payloads:

| Type               | Meaning                         |
| ------------------ | ------------------------------- |
| `OFT`              | Omnichain Fungible Token route  |
| `STARGATE_V2_TAXI` | Stargate V2 instant route       |
| `STARGATE_V2_BUS`  | Stargate V2 batched route       |
| `CCTP`             | Circle native USDC burn/mint    |
| `AORI`             | Intent-based route through Aori |

## Failure Handling

- Empty discovery results: report the missing chain, token, or deployment metadata. Do not infer bridge support from
  token symbols alone.
- Rate limits and 5xx responses: report the API limitation. Continue normal explorer/RPC analysis.
- Non-target chains in VTA results: report that the leg is outside this skill. For that leg, ask for a feature request
  instead of continuing analysis.

## Sources

- https://docs.stargate.finance/developers/api-docs/overview
- https://docs.layerzero.network/v2/developers/value-transfer-api/start
- https://docs.layerzero.network/v2/developers/value-transfer-api/overview
- https://docs.layerzero.network/v2/developers/value-transfer-api/api-reference/overview
- https://docs.layerzero.network/v2/developers/value-transfer-api/api-reference/chains
- https://docs.layerzero.network/v2/developers/value-transfer-api/api-reference/tokens
- https://docs.layerzero.network/v2/developers/value-transfer-api/api-reference/metadata
