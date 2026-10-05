# Uniswap Evidence

Use this reference after `references/workflows/dex-transactions.md` for canonical Uniswap v1, v2, v3, or v4 evidence,
including Universal Router, Permit2, position management, wrapper-driven liquidity, and v1-to-v2 migration. This
reference does not add live quoting/execution or UniswapX order interpretation.

## Establish Canonical Deployment and Version

Resolve the chain first. At observation time, use Uniswap's current deployment pages or official repository deployment
artifacts. Then compare the target's runtime bytecode, verified source, proxy implementation where applicable, factory,
and ABI. Do not keep or infer a local address encyclopedia.

If official sources omit the chain or conflict with the explorer, report the conflict. Treat an independently deployed
fork as non-canonical even when it preserves Uniswap selectors, events, or bytecode.

Use the architecture, entrypoint, and logs together:

| Family | Core identity and high-signal evidence                                                                     |
| ------ | ---------------------------------------------------------------------------------------------------------- |
| v1     | Factory-created exchange per ERC-20. ETH/token exchange, LP-token transfers, and exchange events           |
| v2     | Factory-created pair per token pair. Pair `Mint`, `Burn`, `Swap`, `Sync`, and ERC-20 LP-token transfers    |
| v3     | Factory-created pool per token pair/fee. Pool events plus router or `NonfungiblePositionManager` evidence  |
| v4     | Singleton `PoolManager`. `PoolKey`/`PoolId`, core events, periphery commands, flash-accounting settlements |

The transaction's root target may instead be an integration wrapper, smart account, migrator, position manager, or
Universal Router. Report that entrypoint separately.

## Uniswap v1

- Resolve the token's exchange through the canonical v1 factory or prove the historical factory provenance. v1 exchange
  contracts are also the fungible LP tokens.
- Decode the exact exchange method to distinguish ETH-to-token, token-to-ETH, and token-to-token swaps and exact-input
  from exact-output intent. Confirm execution with `TokenPurchase`/`EthPurchase`, transfers, native value, and receipt
  status.
- For token-to-token swaps, preserve both exchange legs. If the available trace lacks the intermediate ETH leg, do not
  report a single-pool route.
- Use `AddLiquidity`/`RemoveLiquidity`, LP-token mint/burn transfers, and wallet deltas for liquidity actions.
- Only after verifying its deployment and code, identify the official v1-to-v2 migrator. A migration removes the
  wallet's v1 liquidity, adds the resulting token/ETH to v2, mints v2 LP tokens to the recipient, and may refund unused
  token or ETH. Report the two liquidity legs and refund separately. A coincidental remove-plus-add sequence is not
  enough.

## Uniswap v2

- Prove the pair through its canonical factory and token ordering. Pair events establish pool activity. Router calldata
  establishes user intent and recipient.
- LP positions are fungible pair tokens. Use pair `Mint`/`Burn` events together with LP-token `Transfer` mint/burn
  evidence and underlying token transfers.
- Distinguish standalone LP-token `approve`/`permit`, transfer, add/remove liquidity, and swap actions. A permit can be
  consumed inside a remove-liquidity call and need not be a separate user transaction.
- For fee-on-transfer router methods, wallet and pair deltas outrank nominal calldata amounts.
- Treat zaps, vaults, farms, and third-party migrators as integration wrappers. If a wrapper receives one asset, swaps
  part, and adds liquidity, report one wrapper-driven zap with its swap and LP legs. Under that condition, do not claim
  that the root transaction was sent to Uniswap.
- Treat staking/mining deposits, withdrawals, and reward claims as separate incentive-contract actions. Verify the
  incentive contract and reward transfer. Pair LP-token provenance alone does not make a farm canonical Uniswap.

## Uniswap v3

- Identify a pool by canonical factory provenance, token pair, and fee tier. Pool `Swap`, `Mint`, `Burn`, and `Collect`
  events describe core activity. Periphery events identify the wallet-facing action.
- Decode router `exactInput*`/`exactOutput*` calls and multicalls in order. A multicall is a container, not an
  interaction class.
- Track the complete position-NFT lifecycle through `NonfungiblePositionManager`: ERC-721 mint/transfer/approval,
  `IncreaseLiquidity`, `DecreaseLiquidity`, `Collect`, and burn. When evidenced, report the token ID, owner,
  operator/recipient, pool, ticks, liquidity change, and collected assets.
- `DecreaseLiquidity` records newly owed principal from the liquidity reduction. `Collect` can transfer that principal
  together with previously or newly accrued fees. In a decrease-plus-collect multicall, treat the decrease amounts as
  principal. Only when pre-transaction owed amounts, fee-growth accounting, command order, and full collection prove the
  allocation, compute fees as collected minus included principal. If the evidence does not prove that allocation, report
  collected total and the unresolved principal/fee split.
- Burning the position NFT is distinct from decreasing liquidity and collecting. Require its ERC-721 burn evidence.

## Uniswap v4

- Identify the canonical singleton `PoolManager` and relevant periphery deployment. A v4 pool is not a contract address.
  Derive or decode its `PoolKey` (`currency0`, `currency1`, fee, tick spacing, hooks) and corresponding `PoolId`.
- Use `Initialize`, `Swap`, and `ModifyLiquidity` events with the exact `PoolId`. Resolve currencies. Preserve native
  ETH, which v4 can use without WETH.
- Decode Universal Router v4 commands or `PositionManager` action sequences. Position actions include mint, increase,
  decrease, and burn. Fee collection is a zero-liquidity decrease paired with `TAKE_PAIR`. There is no `COLLECT` action.
  Preserve the settle, take, and close actions that resolve deltas.
- Flash accounting nets intermediate balance changes inside `PoolManager`. Do not require an ERC-20 transfer per hop. Do
  not infer a missing hop from transfer logs. Use commands, pool events, balance deltas, and traces.
- Record the hook address and verified permissions. Interpret custom fees, curves, deltas, rewards, or access rules only
  from verified hook source/ABI and observed evidence. If either verified hook source/ABI or observed evidence is
  missing, label the hook opaque. Under that condition, keep its semantic effect as a coverage gap.
- A `ModifyLiquidity` event proves core liquidity changed, not who ultimately owned a wrapper-managed position. Use the
  periphery command, token/position transfer, and recipient evidence for wallet attribution.

## Universal Router and Permit2

- Decode `execute` command bytes and their paired inputs in sequence. Attribute each v2, v3, or v4 swap command to its
  version. List wrap, unwrap, sweep/refund, transfer, and balance-check commands as secondary legs.
- When commands cross versions, do not label the entire Universal Router transaction with one Uniswap version.
- Distinguish an ERC-20 allowance granted to Permit2, a Permit2 allowance or signature-transfer authorization, and the
  later router spend. When evidenced, report owner, token, spender, amount, nonce, and expiration.
- A Permit2 or token approval without a successful consuming swap is approval-only. A failed consuming transaction does
  not undo an approval from an earlier transaction.

## Fees and Asset Results

Use wallet net changes from the shared workflow. Report pool fee tier or dynamic-fee evidence separately from gas,
wrapper/integrator fees, and explicit rewards. Do not calculate historical LP fees, price impact, or slippage from
current reserves or a present-day quote.

For v4, hook accounting can alter pool and wallet deltas. For v2/v3, protocol or wrapper transfers may also make simple
`amountIn - amountOut` fee arithmetic wrong. State only the fee components directly supported by calldata, events,
state, or traces.

## Sources

- https://developers.uniswap.org/docs/protocols/overview
- https://developers.uniswap.org/docs/protocols/v2/deployments
- https://developers.uniswap.org/docs/protocols/v3/deployments
- https://developers.uniswap.org/docs/protocols/v4/deployments
- https://developers.uniswap.org/docs/protocols/v4/concepts/architecture
- https://developers.uniswap.org/docs/protocols/v4/guides/position-manager
- https://developers.uniswap.org/docs/protocols/v4/guides/managing-liquidity/collect-fees
- https://developers.uniswap.org/docs/protocols/universal-router/overview
- https://developers.uniswap.org/docs/protocols/permit2/overview
- https://github.com/Uniswap/v1-contracts
- https://github.com/Uniswap/v2-periphery/blob/master/contracts/UniswapV2Migrator.sol
- https://github.com/Uniswap/v3-periphery
- https://github.com/Uniswap/v4-core
- https://github.com/Uniswap/v4-periphery
