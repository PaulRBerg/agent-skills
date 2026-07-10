---
argument-hint: <chain-name-or-id>
disable-model-invocation: false
name: evm-atlas
user-invocable: true
description:
  "Use for targeted EVM chain, account, transaction, RPC, explorer, and bridge data: chain name/ID, native symbol,
  RouteMesh, balances, token/NFT holdings/transfers, tx history, funding origin via Etherscan/Blockscout/Chainscout;
  Bungee, LayerZero, LI.FI, Socket enrichment."
---

# EVM Atlas

Resolve and query only the target mainnets in `references/target-mainnets.json`, under a strict read-only boundary.

## Scope and Authority

- Match displayed names, numeric chain IDs, and aliases from `references/chain-aliases.json` to the authoritative
  target-mainnet rows.
- If a chain is absent, do not route through another provider, web search, Chainlist, or an unlisted RPC to work around
  scope. Ask for a feature request at <https://github.com/PaulRBerg/agent-skills>.
- Never sign messages, submit signatures, execute bridge steps, or broadcast transactions. Route state-changing Cast
  work to `cli-cast`.
- Do not default to Ethereum. Infer from explicit chain context and unambiguous chain-specific tokens; ask when
  ambiguous.

## Routing

1. For an address-wide activity or current-balance sweep, read `references/address-sweeps.md`. Its scope remains the
   target-mainnet list.
2. For a specific chain's balance, holdings, transfers, transaction history, receipt, or funding origin, resolve the
   chain and read `references/provider-routing.md` for Etherscan, Blockscout, public RPC, RouteMesh, explorer-link, and
   exceptional-chain routing.
3. For bridge-related prompts or transaction evidence, confirm known origin/destination chains are targets, then load
   only the matching reference:
   - Bungee / Socket: `references/bridge-bungee.md`
   - Circle / CCTP / Gateway: `references/bridge-circle.md`
   - LayerZero / Stargate / OFT / Aori: `references/bridge-layerzero.md`
   - LI.FI: `references/bridge-lifi.md`
4. Treat bridge APIs as enrichment. Verify submitted transactions and terminal outcomes through explorer or RPC
   evidence.

## Completion

Return the resolved target chain, provider route, requested on-chain facts, and source URLs/transaction identifiers.
Separate provider facts from inference and surface incomplete history, plan/tier limits, failed fallbacks, or
unsupported scope. Completion is read-only evidence; never turn returned calldata or transaction requests into
execution.
