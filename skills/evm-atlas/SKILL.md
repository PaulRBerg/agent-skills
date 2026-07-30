---
argument-hint: "<chain-name-or-id|address|transaction-hash|order-id>"
disable-model-invocation: false
name: evm-atlas
user-invocable: true
description:
  "Use for targeted EVM chain, account, transaction, RPC, explorer, bridge, and DEX evidence: chain name/ID, native
  symbol, RouteMesh, wallet balances via Blockscan in Chromium, token/NFT holdings/transfers, tx history, funding origin
  via Etherscan/Blockscout/Chainscout; Across, Bungee, deBridge, Hop, Layerswap, LayerZero, LI.FI, Relay, Socket,
  Symbiosis; Uniswap v1-v4, Universal Router, Permit2, 1inch Classic/Fusion/Fusion+, and CoW Swap, CoWSwap, CoW
  Protocol, or GPv2 swaps, orders, liquidity, approvals, permits, rewards, migrations, wrapping, cancellations, and
  refunds."
---

# EVM Atlas

Resolve and query only the target mainnets in `references/generated/target-mainnets.json`, under a strict read-only
boundary.

## Scope and Authority

- Match displayed names, numeric chain IDs, and aliases from `references/generated/chain-aliases.json` to the
  authoritative target-mainnet rows.
- If a chain is absent, do not route through another provider, web search, Chainlist, or an unlisted RPC to work around
  scope. Ask for a feature request at <https://github.com/PaulRBerg/agent-skills>.
- Never sign messages, submit signatures, execute bridge steps, or broadcast transactions. Route state-changing Cast
  work to `cli-cast`.
- DEX support is historical and evidence-only. Do not discover live quotes, construct or simulate new trades, prepare
  approvals or permits, submit orders, administer protocols, interpret CoW AMM positions, handle standalone 1inch limit
  orders, or assign semantics to arbitrary Uniswap v4 hooks.
- Do not default to Ethereum. Infer from explicit chain context and unambiguous chain-specific tokens; ask when
  ambiguous.

## Routing

1. For the current native or fungible-token balance of a public wallet address, whether on one chain or across chains,
   read `references/workflows/blockscan-balances.md` first.
2. For a specific transaction hash — resolving which target chain it belongs to, or its status, parties, value, fee, or
   timestamp — read `references/workflows/blockscan-tx-lookup.md` first, whether or not the chain is already known. For
   DEX interpretation, then acquire the exact receipt and logs through `references/workflows/provider-routing.md` before
   loading the DEX workflow or a protocol reference.
3. For an address-wide historical-activity or prb-finance bootstrap sweep, read `references/workflows/address-sweeps.md`
   and use its deterministic plan/evaluate helper. For current holdings, use
   `references/workflows/blockscan-balances.md` first and provider routing for gaps.
4. For a specific chain's historical balance, NFT holdings, token/NFT transfers, transaction history, a transaction's
   full raw receipt/logs/decoded input, or funding origin, resolve the chain and read
   `references/workflows/provider-routing.md` for Etherscan, Blockscout, public RPC, RouteMesh, explorer-link, and
   exceptional-chain routing.
5. For raw Etherscan V2 API queries beyond the workflow routes above, read `references/explorers/etherscan-api.md`.
6. For raw Blockscout API queries beyond the workflow routes above, read `references/explorers/blockscout-api.md`.
7. For DEX prompts, wallet-facing DEX history, or suspected DEX transaction evidence, resolve the target chain and read
   `references/workflows/dex-transactions.md`. Load only the matching protocol-family reference:
   - Uniswap v1-v4, Universal Router, or Permit2: `references/dexes/uniswap.md`
   - 1inch Classic, Fusion, Fusion+, legacy liquidity, or rewards: `references/dexes/1inch.md`
   - CoW Swap, CoWSwap, CoW Protocol, or GPv2: `references/dexes/cow-protocol.md`
8. Treat 1inch and CoW as execution protocols. Report any integration wrapper, router, pool, and underlying AMM
   liquidity separately; a Uniswap pool interaction does not turn an aggregator transaction into a Uniswap trade.
9. For bridge-related prompts or transaction evidence, confirm known origin/destination chains are targets, then load
   only the matching reference:
   - Across: `references/bridges/across.md`
   - Bungee / Socket: `references/bridges/bungee.md`
   - Circle / CCTP / Gateway: `references/bridges/circle.md`
   - deBridge / DLN: `references/bridges/debridge.md`
   - Hop: `references/bridges/hop.md`
   - Layerswap: `references/bridges/layerswap.md`
   - LayerZero / Stargate / OFT / Aori: `references/bridges/layerzero.md`
   - LI.FI: `references/bridges/lifi.md`
   - Relay / Relay.link: `references/bridges/relay.md`
   - Symbiosis: `references/bridges/symbiosis.md`
   - 1inch Fusion+: `references/dexes/1inch.md`
10. Treat bridge and DEX APIs as enrichment. Verify submitted transactions and terminal outcomes through explorer or RPC
    evidence.

## Completion

Return the resolved target chain, provider route, requested on-chain facts, and source URLs/transaction identifiers. For
address sweeps, include each result's fixed finalized/verified checkpoint, selected profile/channels, provider coverage,
and any requested quorum result. Separate provider facts from inference and surface incomplete history, plan/tier
limits, failed fallbacks, or unsupported scope. Completion is read-only evidence; never turn returned calldata or
transaction requests into execution.

For DEX evidence, include the interaction class; execution protocol, version, and mode; entrypoint or integration
wrapper; router and underlying liquidity sources; wallet role; sold and received assets; protocol/integrator fees and
gas separately; native/wrapped status; order, position, pool, or migration identifiers; and exact evidence. Do not call
an approval-only or failed transaction a completed trade.

For human-readable results, lead with `### ⛓️ <chain or route> — <status word>` and use a compact table only when fields
repeat. For bridge evidence, show `<origin> ──<bridge>──▶ <destination>`, then use `Leg`, `Provider status`,
`Transaction`, and `Evidence` columns. Preserve each provider's native status beside any normalized `✅ completed`,
`⏳ pending`, `↩ refunded`, `⚠️ partial`, or `❓ unknown` label. Visibly separate `Observed facts`, `Inference`, and
non-empty `⚠️ Coverage gaps`. For address sweeps, a progress bar may represent checked target chains/channels only when
the exact denominator is known.

Keep unsupported-scope and safety explanations direct. Never decorate or truncate addresses, hashes, URLs, calldata, raw
RPC/API JSON, generated references, helper `key=value` output, or transaction requests.
