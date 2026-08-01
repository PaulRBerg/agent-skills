# DEX Transaction Evidence

Use this workflow for wallet-facing DEX history and for interpreting known transactions or orders. It covers swaps and
orders, liquidity and position actions, approvals and permits, rewards, migrations, native wrapping, refunds, and
cancellations. It is evidence-only: do not quote, construct, sign, submit, or administer anything.

## Resolve Before Interpretation

1. For a transaction hash with a named chain, resolve it against `references/generated/target-mainnets.json`, then use
   `references/workflows/provider-routing.md` directly for the raw transaction, receipt, logs, and decoded input. Stop
   on a non-target chain.
2. When the chain is unknown, use `references/workflows/blockscan-tx-lookup.md` once to resolve it, then continue
   through `references/workflows/provider-routing.md`. Apply the OP Mainnet pre-regenesis exception in `SKILL.md` before
   requiring a current-provider receipt.
3. Require exact provider evidence before describing an on-chain result. Except for the documented OP Mainnet legacy
   route, this means the exact receipt. A failed or pending receipt cannot prove a completed trade, liquidity change,
   claim, migration, wrap, refund, or cancellation.
4. For address history, use the resolved chain's provider route to identify candidate transactions, then inspect each
   candidate's receipt. A transaction-list label or method name is not enough.
5. For an off-chain order identifier, load only the relevant protocol reference, resolve its candidate fill,
   cancellation, or refund transactions, then verify those transactions on-chain. If the identifier or API namespace is
   not globally chain-scoped, ask for a chain. Never default to Ethereum or fan out across chains without approval.

An order may have no maker transaction. Distinguish order creation or signing, API acceptance, on-chain fill,
cancellation, expiry, and refund as separate lifecycle facts.

## Evidence Stack

Prefer evidence in this order and retain conflicts:

1. Raw receipt status and emitted logs from the resolved chain.
2. Transaction calldata decoded with the verified ABI for the exact target and implementation.
3. Runtime bytecode, proxy implementation, factory provenance, and current official deployment feeds.
4. Traces and internal calls for wrappers, routers, callbacks, native value, and underlying liquidity.
5. Protocol APIs for known identifiers and protocol-native status.
6. Explorer labels, decoded method names, token symbols, and UI descriptions as hints only.

If traces are unavailable, report every route or internal-call claim they would have established as a coverage gap.
Never promote an explorer label, a four-byte selector guess, or matching source text to deployment proof.

## Classify the Interaction

Choose one primary interaction class and list secondary legs:

| Class                 | Required evidence                                                                  |
| --------------------- | ---------------------------------------------------------------------------------- |
| Swap or order fill    | Successful receipt plus wallet asset deltas and protocol/router/pool evidence      |
| Order lifecycle       | Signed order/API record plus fill, invalidation, expiry, or refund evidence        |
| Liquidity or position | Pool/periphery events, position or LP identifiers, and wallet asset/receipt deltas |
| Approval or permit    | Exact spender, asset, amount/expiry/nonce, and emitted or consumed permit evidence |
| Reward or claim       | Verified distributor/staking contract plus claimed-asset transfer                  |
| Migration             | Verified migrator/wrapper plus linked source withdrawal and destination deposit    |
| Wrap or unwrap        | Native-value trace and wrapped-token deposit/withdrawal or transfer evidence       |
| Refund or sweep       | Proven return transfer/value trace and the component that returned it              |
| Cancellation          | Protocol-native invalidation plus any resulting on-chain return                    |

An approval-only transaction remains an approval even if its spender is a DEX router. A successful router call without
wallet sold/received deltas may be a cancellation, claim, refund, or no-op; do not force it into a swap class.

## Attribute Every Layer

Report these roles independently:

- **Execution protocol:** the user-facing settlement or aggregation protocol, such as 1inch Fusion or CoW Protocol.
- **Integration wrapper:** the root target or smart account that composed the action.
- **Router or periphery:** the contract that dispatched swaps or position commands.
- **Pool or settlement:** the core contract whose state changed.
- **Underlying liquidity:** each AMM, pool version, or direct counterflow proven by calls or events.

Do not relabel a 1inch or CoW execution as Uniswap merely because a solver or router used Uniswap liquidity. For a
wrapper-driven Uniswap zap, identify the wrapper as the entrypoint and Uniswap as underlying liquidity. If the root
target is not verified, report an unknown wrapper and continue only with the proven downstream calls.

## Compute Wallet-Level Asset Changes

1. Select the wallet and role being reported: trader/maker, receiver, LP or position owner, operator, approver, reward
   claimant, or refund recipient. Do not combine unrelated participants in a batched transaction.
2. Sum inbound and outbound ERC-20 `Transfer` logs for that wallet by token. Include ERC-721/ERC-1155 position transfers
   separately; never treat token IDs as fungible amounts.
3. Use traces and native value transfers for native-asset legs. Preserve internal wraps and unwraps instead of silently
   replacing WETH with ETH or vice versa.
4. Report the final wallet delta as sold, received, returned, or position assets. Keep temporary router/pool transfers
   as route evidence, not wallet proceeds.
5. Report transaction gas separately from sold assets. Use the receipt's effective gas price, gas used, and
   chain-specific L1/data fee fields when present; identify who paid it. A relayer or solver may pay gas for an order.
6. Preserve raw integer amounts and decimals evidence. If token metadata conflicts or is unavailable, do not invent a
   display amount.

Refunded unused input is not swap output. A router sweep or unwrap may explain the wallet's final receipt without being
a separate trade. Fee-on-transfer, rebasing, internal-balance, and hook-modified assets require trace or state evidence;
event arithmetic alone may be incomplete.

## Interpretation Limits

- Use exact-input or exact-output only when verified calldata or command encoding proves it.
- Use the successful receipt and asset deltas for execution. Calldata limits are intent, not actual output.
- Do not reconstruct a historical quote, slippage, price impact, or expected route from current data.
- Do not claim full route composition from endpoint transfers alone. List only pools and interactions proven by
  calldata, logs, or traces.
- Separate LP principal, accrued fees, rewards, and refunds only when contract accounting and event sequencing support
  the split. Otherwise report the combined collected amount and the missing evidence.
- Treat protocol APIs as optional enrichment. Missing credentials, rate limits, stale indexing, unsupported chains, or
  conflicting status must degrade to on-chain evidence with a visible coverage gap.
- Treat a verified fork as that fork. Matching Uniswap interfaces, bytecode, event signatures, or pool math does not
  make an unverified or independently deployed fork canonical Uniswap.

## Completion

Return the fields required by `SKILL.md`. Keep `Observed facts`, `Inference`, and non-empty `⚠️ Coverage gaps` as
separate sections. Name the evidence source for every material classification and include exact transaction, order,
pool, and position identifiers without truncation.

For a failed transaction, report attempted calldata only as attempted behavior and state that no receipt logs or state
changes survived. For an unsupported chain, unavailable trace, unknown deployment, or API-only status, stop the affected
claim at the strongest supported evidence.
