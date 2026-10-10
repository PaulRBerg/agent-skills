---
argument-hint: "<chain-name-or-id|address|transaction-hash|order-id>"
compatibility: RouteMesh requests require the `routemesh` CLI initialized on macOS with `routemesh init`.
coordination: exempt
name: evm-atlas
skill-dependencies:
  - chromium-browser
  - cli-cast
description:
  "Use for targeted EVM chain, account, transaction, RPC, explorer, bridge, and DEX evidence: chain name/ID, native
  symbol, RouteMesh, wallet balances and DeFi positions via DeBank/Blockscan in Chromium, cross-chain USD portfolio
  value or net worth, token/NFT holdings/transfers, tx history, funding origin via Etherscan/Blockscout/Chainscout.
  Covers Arbitrum canonical withdrawals, Across, Bungee, deBridge, Gas.zip, Hop, Layerswap, LayerZero, LI.FI, Relay,
  Socket, Symbiosis. Covers Uniswap v1-v4, Universal Router, Permit2, 1inch Classic/Fusion/Fusion+, and CoW Swap,
  CoWSwap, CoW Protocol, or GPv2 swaps, orders, liquidity, approvals, permits, rewards, migrations, wrapping,
  cancellations, and refunds."
---

# EVM Atlas

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Resolve and query only the target mainnets in `references/generated/target-mainnets.json`, under a strict read-only
boundary.

Before you collect browser UI evidence, load `chromium-browser`. Follow its page-ownership, live-tool, and privacy
contract. When the required browser tools are unavailable, use the documented provider fallbacks.

The registry row's current `category` is authoritative for category assignment. For an exact-zero native sweep or a
question about category-specific fee behavior, resolve the target before reading
[chain categories](references/chain-categories.md). Do not infer a historical category or maintain a prose roster of
target chains.

## Scope and Authority

- Match displayed names, numeric chain IDs, and aliases from `references/generated/chain-aliases.json` to the
  authoritative target-mainnet rows.
- Resolve a chain with `scripts/chain-lookup.sh <name|alias|slug|chain_id>` first. It prints `key=value` lines, or one
  compact row with `--json`. Never `cat` a generated JSON file or print the full `routemesh chains` list. For other
  queries, use a `jq -c` selector with named fields, such as `jq -c '.chains[] | {chainId, category}'`.
- If a chain is absent, do not route through another provider, web search, Chainlist, or an unlisted RPC to work around
  scope. Ask for a feature request at <https://github.com/PaulRBerg/agent-skills>.
- A target row with `defunct` identifies a chain its operator shut down. Its history is final through
  `defunct.finalStateBlock`. Later empty blocks or missing provider history are not coverage gaps. Without
  `finalStateBlock`, the chain may still accept exit transactions such as withdrawals. In that case, treat it as live.
  State the shutdown in the result.
- Own every discrete read and bounded live subscription handed off by `cli-cast`, including chain, block, fee, nonce,
  `eth_call`, `eth_estimateGas`, transaction, receipt, log, balance, code, storage, proof, and ENS queries. Complete the
  read here even when its result will prepare, simulate, or verify later state-changing work.
- Never sign messages, submit signatures, execute bridge steps, or broadcast transactions. Route state-changing Cast
  work to `cli-cast`.
- DEX support is historical and evidence-only. Do not discover live quotes, construct or simulate new trades, prepare
  approvals or permits, submit orders, administer protocols, interpret CoW AMM positions, handle standalone 1inch limit
  orders, or assign semantics to arbitrary Uniswap v4 hooks.
- Do not default to Ethereum. Infer from explicit chain context and unambiguous chain-specific tokens. If the chain is
  ambiguous, ask.
- Never echo, interpolate, or log API-key values (`ETHERSCAN_API_KEY`, `BLOCKSCOUT_API_KEY`, RPC keys). Check presence
  without printing values: `[ -n "$ETHERSCAN_API_KEY" ] && echo set || echo unset`. Never put `${VAR:-...}` or
  `${VAR:+...}` expansions in printed output.
- For Etherscan reads, use the official `etherscan` CLI with the resolved numeric `--chain` and `--output json`.
  Existing CLI authentication can use an environment key or saved login. Do not inspect credentials, pass `--api-key`,
  or run `whoami`, `config`, or `login`. CLI availability does not change provider priority or plan gates. Never call
  `proxy eth_sendRawTransaction` or submit contract verification through this read-only skill.
- Keyless Blockscout is sunset (July 2026). Hosted `*.blockscout.com` instance subdomains also rate-limit keyless
  traffic. Route every Blockscout-hosted chain through the keyed `https://api.blockscout.com/{chain_id}` gateway. See
  `references/explorers/blockscout-endpoints.md`.
- Every agent on the host shares DeBank's rate limit. Hold a `scripts/debank-gate.py` lease for any debank.com access,
  including a quick profile look. See the Global Queue in `references/workflows/debank-portfolio.md`.
- An unreachable or erroring indexer is a coverage gap, never evidence of zero activity. Confirm in Chromium before you
  record an endpoint as down or blocked. State the verification method in results.

## Routing

1. For a discrete JSON-RPC read, batch, or bounded live subscription, including one handed off by `cli-cast`, resolve
   the chain with `scripts/chain-lookup.sh`. Then read `references/workflows/provider-routing.md`. Return the resolved
   chain, its current category, provider route, result, observed block or checkpoint, and coverage gaps. Do not route
   the read back to `cli-cast`.
2. For the current native or fungible-token balances or DeFi positions of a public wallet address across chains, read
   `references/workflows/debank-portfolio.md` first. For one named chain, read
   `references/workflows/blockscan-balances.md` first.
3. For the current USD value of one or more addresses across target chains (portfolio value, net worth, drained or dust
   checks), read `references/workflows/address-usd-value.md`.
4. For a specific transaction hash on a named chain, resolve the chain with `scripts/chain-lookup.sh`. Then read
   `references/workflows/provider-routing.md` directly for the transaction facts. Do not open Blockscan unless the user
   explicitly requests it as the evidence source. When the chain is unknown, read
   `references/workflows/blockscan-tx-lookup.md` once to resolve it.

   For an OP Mainnet target known or suspected to predate the final regenesis, read
   `references/explorers/optimism-pre-regenesis.md`. In that case, return its legacy execution packet or
   component-specific coverage outcome instead of requiring a current-provider receipt. Otherwise, acquire the exact
   provider receipt and logs before DEX or bridge outcome interpretation.

5. For an address-wide historical-activity or `bootstrap-discovery` sweep, read
   `references/workflows/address-sweeps.md`. Use its deterministic plan/evaluate helper. For current holdings, use
   `references/workflows/debank-portfolio.md` first and provider routing for gaps.
6. For a specific chain's historical balance, NFT holdings, token/NFT transfers, transaction history, a transaction's
   full raw receipt/logs/decoded input, or funding origin, resolve the chain with `scripts/chain-lookup.sh`. Then read
   `references/workflows/provider-routing.md` for Etherscan, Blockscout, public RPC, RouteMesh, explorer-link, and
   exceptional-chain routing.
7. For Etherscan CLI reads beyond the workflow routes above, read `references/explorers/etherscan-api.md`. It defines
   CLI 1.1.1 output, manual paging, plan gates, and direct API exceptions. ENS forward resolution remains a direct API
   exception for Ethereum mainnet onchain names. Apply its cache and resolver limits.
8. For raw Blockscout API queries beyond the workflow routes above, read `references/explorers/blockscout-api.md`.
9. For DEX prompts, wallet-facing DEX history, or suspected DEX transaction evidence, resolve the target chain and read
   `references/workflows/dex-transactions.md`. Load only the matching protocol-family reference:
   - Uniswap v1-v4, Universal Router, or Permit2: `references/dexes/uniswap.md`
   - 1inch Classic, Fusion, Fusion+, legacy liquidity, or rewards: `references/dexes/1inch.md`
   - CoW Swap, CoWSwap, CoW Protocol, or GPv2: `references/dexes/cow-protocol.md`
10. Treat 1inch and CoW as execution protocols. Report any integration wrapper, router, pool, and underlying AMM
    liquidity separately. A Uniswap pool interaction does not turn an aggregator transaction into a Uniswap trade.
11. For bridge-related prompts or transaction evidence, confirm that known origin/destination chains are targets. Then
    load only the matching reference:

    - Arbitrum One/Nova canonical bridge withdrawals: `references/bridges/arbitrum.md`
    - Across: `references/bridges/across.md`
    - Bungee / Socket: `references/bridges/bungee.md`
    - Circle / CCTP / Gateway: `references/bridges/circle.md`
    - deBridge / DLN: `references/bridges/debridge.md`
    - Gas.zip: `references/bridges/gaszip.md`
    - Hop: `references/bridges/hop.md`
    - Layerswap: `references/bridges/layerswap.md`
    - LayerZero / Stargate / OFT / Aori: `references/bridges/layerzero.md`
    - LI.FI: `references/bridges/lifi.md`
    - Relay / Relay.link: `references/bridges/relay.md`
    - Symbiosis: `references/bridges/symbiosis.md`
    - 1inch Fusion+: `references/dexes/1inch.md`

12. Treat bridge and DEX APIs as enrichment. Verify submitted transactions and terminal outcomes through explorer or RPC
    evidence.

## Completion

Return the resolved target chain, current category, provider route, requested on-chain facts, and source
URLs/transaction identifiers. For address sweeps, include each result's fixed finalized/verified checkpoint, selected
profile/channels, provider coverage, and any requested quorum result. Separate provider facts from inference. Report
incomplete history, plan/tier limits, failed fallbacks, or unsupported scope. Completion provides read-only evidence.
Never turn returned calldata or transaction requests into execution.

For a `cli-cast` handoff, return one read packet with these fields:

- Resolved chain name, ID, and current category.
- Exact provider route.
- Result.
- Observed block or checkpoint.
- Coverage gaps, which may be empty when none are observed.

Do not include a signing or broadcast command.

For DEX evidence, include these fields:

- Interaction class.
- Execution protocol, version, and mode.
- Entrypoint or integration wrapper.
- Router and underlying liquidity sources.
- Wallet role.
- Sold and received assets.
- Protocol/integrator fees and gas separately.
- Native/wrapped status.
- Order, position, pool, or migration identifiers.
- Exact evidence.

Do not call an approval-only or failed transaction a completed trade.

For human-readable results, lead with `### ⛓️ <chain or route> — <status word>`. Use a compact table only when fields
repeat.

For bridge evidence, show `<origin> ──<bridge>──▶ <destination>`. Then use `Leg`, `Provider status`, `Transaction`, and
`Evidence` columns. Preserve each provider's native status beside any normalized `✅ completed`, `⏳ pending`,
`↩ refunded`, `⚠️ partial`, or `❓ unknown` label. Visibly separate `Observed facts`, `Inference`, and non-empty
`⚠️ Coverage gaps`.

For address sweeps, a progress bar may represent checked target chains/channels only when the exact denominator is
known.

Keep unsupported-scope and safety explanations direct. Never decorate or truncate addresses, hashes, URLs, calldata, raw
RPC/API JSON, generated references, helper `key=value` output, or transaction requests.
