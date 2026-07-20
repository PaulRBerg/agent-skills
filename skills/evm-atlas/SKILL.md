---
argument-hint: <chain-name-or-id>
disable-model-invocation: false
name: evm-atlas
user-invocable: true
description:
  "Use for targeted EVM chain, account, transaction, RPC, explorer, and bridge data: chain name/ID, native symbol,
  RouteMesh, wallet balances via Blockscan in Chromium, token/NFT holdings/transfers, tx history, funding origin via
  Etherscan/Blockscout/Chainscout; Bungee, Layerswap, LayerZero, LI.FI, Socket enrichment."
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

1. For the current native or fungible-token balance of a public wallet address, whether on one chain or across chains,
   read `references/blockscan-balances.md` first.
2. For an address-wide historical-activity or current-NFT sweep, read `references/address-sweeps.md`. Its
   current-balance workflow is also the API fallback for Blockscan gaps.
3. For a specific chain's historical balance, NFT holdings, token/NFT transfers, transaction history, receipt, or
   funding origin, resolve the chain and read `references/provider-routing.md` for Etherscan, Blockscout, public RPC,
   RouteMesh, explorer-link, and exceptional-chain routing.
4. For bridge-related prompts or transaction evidence, confirm known origin/destination chains are targets, then load
   only the matching reference:
   - Bungee / Socket: `references/bridge-bungee.md`
   - Circle / CCTP / Gateway: `references/bridge-circle.md`
   - Layerswap: `references/bridge-layerswap.md`
   - LayerZero / Stargate / OFT / Aori: `references/bridge-layerzero.md`
   - LI.FI: `references/bridge-lifi.md`
5. Treat bridge APIs as enrichment. Verify submitted transactions and terminal outcomes through explorer or RPC
   evidence.

## Completion

Return the resolved target chain, provider route, requested on-chain facts, and source URLs/transaction identifiers. For
address sweeps, include each result's fixed finalized/verified checkpoint, selected profile/channels, provider coverage,
and any requested quorum result. Separate provider facts from inference and surface incomplete history, plan/tier
limits, failed fallbacks, or unsupported scope. Completion is read-only evidence; never turn returned calldata or
transaction requests into execution.
