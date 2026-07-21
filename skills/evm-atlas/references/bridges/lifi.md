# LI.FI Bridging

## Overview

Use LI.FI as a read-only source for bridge and cross-chain swap quotes, route options, supported chains/tokens/tools,
and transfer status on target chains. Prefer the REST API for agent use. The SDK docs are useful for object names and
route semantics, but this skill must not execute SDK routes or returned transaction requests.

Default to the REST API base URL:

```text
https://li.quest/v1
```

Most LI.FI API calls work without an API key at lower rate limits. Add the key only when present:

```bash
-H "x-lifi-api-key: $LIFI_API_KEY"
```

Never execute bridge steps from this skill. Do not sign messages, submit transactions, execute SDK routes, call
`executeRoute`, call `getStepTransaction` for execution, or broadcast any returned `transactionRequest`. Returned
transaction data is for inspection only.

## Read-Only Router

Use this router after the known origin and destination chains are confirmed against
`references/generated/target-mainnets.json`.

1. **Simple quote:** call `GET /quote` when the user wants one best route or a specific bridge/swap estimate.
2. **Multiple routes:** call `POST /advanced/routes` when the user needs route comparison, multiple options, or complex
   multi-step paths.
3. **Known source tx hash:** call `GET /status?txHash=<hash>`. Add `fromChain`, `toChain`, and `bridge` when known for
   faster, more precise results.
4. **Supported chains:** call `GET /chains`; use `chainTypes=EVM` when only EVM chains are relevant.
5. **Supported tokens:** call `GET /tokens?chains=<chainIds>` for chain-specific token catalogs.
6. **Available bridges and exchanges:** call `GET /tools`.
7. **Possible token connections:** call `GET /connections` when validating whether a pair can be swapped or bridged.

Example quote inspection:

```bash
curl -sS "https://li.quest/v1/quote?fromChain=1&toChain=42161&fromToken=USDC&toToken=USDC&fromAmount=1000000&fromAddress=0xWALLET"
```

Example status lookup:

```bash
curl -sS "https://li.quest/v1/status?txHash=0xTX_HASH&fromChain=1&toChain=42161&bridge=stargateV2"
```

If `$LIFI_API_KEY` is set, add `-H "x-lifi-api-key: $LIFI_API_KEY"`.

## Request Fields

For quote or route inspection, use explicit chain IDs from `references/generated/target-mainnets.json`.

| Field                          | Use                                                                    |
| ------------------------------ | ---------------------------------------------------------------------- |
| `fromChain`                    | Source chain ID                                                        |
| `toChain`                      | Destination chain ID                                                   |
| `fromToken`                    | Source token symbol or token contract address                          |
| `toToken`                      | Destination token symbol or token contract address                     |
| `fromAmount`                   | Source amount in smallest units                                        |
| `toAmount`                     | Destination amount in smallest units; do not combine with `fromAmount` |
| `fromAddress`                  | Sender address; use only user-provided or placeholder addresses        |
| `toAddress`                    | Destination receiver; defaults may differ by endpoint                  |
| `slippage`                     | Optional slippage tolerance; default is API-defined                    |
| `allowBridges` / `denyBridges` | Restrict route bridge set when the user requests it                    |

Do not invent wallet addresses. Use placeholders for hypothetical quotes and user-provided addresses for real lookups.

## Report Fields

Extract and report these fields when present:

| Field                   | LI.FI path examples                                                |
| ----------------------- | ------------------------------------------------------------------ |
| Route or quote ID       | `id`, `routes[].id`                                                |
| Tool / bridge           | `tool`, `toolDetails`, `includedSteps[].tool`, `steps[].tool`      |
| Source chain/token      | `action.fromChainId`, `action.fromToken`, `estimate.fromAmount`    |
| Destination chain/token | `action.toChainId`, `action.toToken`, `estimate.toAmount`          |
| Amounts                 | `estimate.fromAmount`, `estimate.toAmount`, `estimate.toAmountMin` |
| Gas and fees            | `estimate.gasCosts[]`, `estimate.feeCosts[]`                       |
| Execution artifacts     | `transactionRequest`, step transaction fields for inspection only  |
| Status                  | `status`, `substatus`, `substatusMessage`                          |
| Source tx hash          | `sending.txHash`, `transactionHash`, request `txHash`              |
| Destination tx hash     | `receiving.txHash`                                                 |
| Explorer links          | `sending.txLink`, `receiving.txLink`                               |

Token amounts are raw integer units. Convert with token decimals when present, and preserve raw values when decimals are
absent.

## Status Values

Interpret LI.FI status as:

| Status      | Meaning                 |
| ----------- | ----------------------- |
| `NOT_FOUND` | LI.FI has no record yet |
| `PENDING`   | In progress             |
| `DONE`      | Terminal success        |
| `FAILED`    | Terminal failure        |

Important substatuses:

| Substatus   | Meaning                                        |
| ----------- | ---------------------------------------------- |
| `COMPLETED` | Transfer completed as expected                 |
| `PARTIAL`   | User received a different token; still success |
| `REFUNDED`  | Tokens were returned to the sender             |

For `DONE`, still verify terminal destination activity with explorer or RPC data when the destination chain is a target
chain.

## Failure Handling

- Missing `$LIFI_API_KEY`: use unauthenticated requests and respect lower public rate limits.
- Rate limit `429`: back off, mention the limit, and continue explorer/RPC analysis.
- No route found: report the API reason when present; try only user-approved alternative tokens, chains, amounts, or
  bridge filters.
- Insufficient balance or gas: report the quoted requirement; do not attempt remediation transactions.
- Slippage errors: report the issue; do not change slippage unless the user explicitly asks for a new quote.
- Non-target chains in LI.FI results: report that the leg is outside this skill and ask for a feature request rather
  than continuing analysis on that leg.

## Sources

- https://docs.li.fi/sdk/overview
- https://docs.li.fi/api-reference/introduction
- https://docs.li.fi/agents/overview
- https://docs.li.fi/sdk/request-routes
- https://docs.li.fi/sdk/execute-routes
- https://docs.li.fi/sdk/chains-tools
