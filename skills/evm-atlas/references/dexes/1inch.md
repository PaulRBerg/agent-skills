# 1inch Evidence

Use this reference after `references/workflows/dex-transactions.md` for 1inch Classic swaps, same-chain Fusion orders,
Fusion+ cross-chain orders, and historical 1inch liquidity/reward interactions. It is evidence-only.

Do not call live Classic `/quote` or `/swap` endpoints, request a Fusion/Fusion+ quote, construct or submit an order, or
cover standalone Orderbook/Limit Order workflows. Fusion's use of the Limit Order Protocol does not expand that scope.

## Attribution and Deployment

Treat 1inch as the execution or aggregation protocol. Report the integration wrapper, 1inch router/settlement, and each
underlying liquidity source separately. A Classic or Fusion execution that calls a Uniswap pool remains a 1inch
execution using Uniswap liquidity.

For historical and current routers, settlement contracts, escrow factories, and legacy pools:

1. Resolve the target chain.
2. Consult the current 1inch API deployment endpoint where documented, official tagged contract repositories and their
   `deployments/` artifacts, then verified explorer source.
3. Compare runtime bytecode, proxy implementation, and the ABI for the transaction's block-era contract.
4. Report the observed router/contract version. Do not infer that a historical transaction used today's API or router
   version, and do not maintain a copied address list.

## Classic Swaps

- Recognize historical aggregation contracts and current Aggregation Router variants from verified code, not address
  shape or explorer labels.
- Decode the exact router ABI and calldata. Historical `swap`, split-route, direct-pool, and optimized selector families
  differ; a decoded selector name from an unrelated ABI is not evidence.
- Use traces, pool events, and token/native transfers to attribute underlying liquidity. Router calldata or an API route
  from a different time cannot reconstruct an opaque executor's historical path.
- Compute the wallet's sold, received, refunded, and wrapped/native legs from the receipt and trace. Report the wallet
  gas payer separately.
- Classify ERC-20 approvals and permit calls separately from the swap. Verify the spender against the block-era router
  or settlement deployment; an approval to an obsolete router is still an approval, not evidence of a later trade.
- List integrator or infrastructure fees only when calldata, transfers, or verified contract accounting proves them. Do
  not back-calculate a fee from a live quote or current documentation.

## Same-Chain Fusion

Fusion is an intent order filled by a resolver, not a maker-submitted swap transaction.

- Require a resolved chain before calling the chain-scoped Fusion Orders API for a known order hash or maker address.
  Use the current documented API version and configured credential. Missing credentials are a coverage gap, not a reason
  to call quote, relayer, or submission endpoints.
- Preserve the API-native status and fill array. Sum only fills tied to verified settlement transactions; distinguish
  pending, partial, filled, expired, and cancelled states.
- Verify every reported fill through its receipt, 1inch settlement/router evidence, transfers, and underlying liquidity
  calls. The API can associate an order hash with fills but does not replace on-chain execution evidence.
- Report maker, receiver, maker/taker assets, filled amounts, resolver/settlement, gas payer, and order hash. The
  resolver normally pays settlement gas; a native-order escrow creation may still produce a separate maker transaction.
- Decode any order extension carrying an ERC-20 permit or Permit2 authorization. Report it as consumed only when the
  verified settlement call, event, or allowance state proves consumption; signature data alone is intent.
- Treat a signed or API-accepted order without a verified fill as an order lifecycle record, not a completed trade.
  Expiry, invalid signature, insufficient allowance, and cancellation are not zero-value swaps.

## Fusion+ Cross-Chain

Route Fusion+ here from both DEX and bridge prompts.

1. Resolve and validate both source and destination chains. If either is outside the target set, stop that leg and
   report partial coverage.
2. Query the current Fusion+ Orders status endpoint only for a known order hash. Preserve `srcChainId`, `dstChainId`,
   order version, receiver, fill status, and every source/destination escrow event returned.
3. Verify the source fill and escrow creation on the source chain and the destination escrow funding/withdrawal on the
   destination chain. Correlate order hash, escrow immutables/factory provenance, hashlock, amounts, and fill index
   where available.
4. Report source and destination asset changes, resolver safety deposits, withdrawal recipient, and gas payer per leg.
   Do not present resolver deposits as user bridge fees.
5. For partial fills, keep each fill and escrow pair separate. Do not merge secrets, fill amounts, or settlement
   transactions across resolvers.
6. For cancellation, recovery, rescue, or refund, require the applicable escrow call/event and actual return transfer.
   An API `refunding`, `refunded`, `cancelled`, or `executed` status alone is enrichment.

Fusion+ uses linked source/destination escrows rather than proving a conventional lock-and-mint bridge route. Describe
the execution mode as Fusion+ and the observed escrow lifecycle; do not invent a bridge provider or messaging leg.

## Legacy Liquidity, Claims, and Rewards

- The historical 1inch Liquidity Protocol/Mooniswap is obsolete. Verify the pool through the archived official
  repository, deployment artifacts, factory provenance, and code before using the 1inch label.
- Classify pool `deposit`/`withdraw` and swap calls with LP-token mint/burn/transfers and underlying wallet deltas.
  Preserve native-token legs and referral-fee LP minting when evidenced.
- Identify farms, staking, and reward distributors separately from the liquidity pool. A stake/unstake, claim, or reward
  transfer is not a swap or LP withdrawal.
- For legacy reward claims, report the verified distributor, claimant, reward asset, and received amount. A generic
  `claim` selector or token transfer without contract provenance is insufficient.

## Failure and Coverage

- Approval-only: report token, owner, spender, amount, and receipt; do not report a trade.
- Failed Classic or settlement transaction: describe decoded intent as attempted and report no surviving trade events or
  wallet asset changes other than gas.
- Unverified router, pool, fork, or executor: retain observed transfers and selector bytes, but leave protocol/version
  or route attribution unknown.
- Unavailable traces: report wallet net changes and proven pool events; mark split percentages and hidden liquidity
  sources unknown.
- Missing or stale API data: keep on-chain evidence authoritative and show the indexing/authentication gap.

## Sources

- https://business.1inch.com/portal/documentation/apis/swap/swap
- https://business.1inch.com/portal/documentation/apis/swap/classic-swap/introduction
- https://business.1inch.com/portal/documentation/apis/swap/intent-swap/introduction
- https://business.1inch.com/portal/documentation/apis/swap/intent-swap/orders/v2.0/1/order/status/method/post
- https://business.1inch.com/portal/documentation/apis/swap/cross-chain-swap/introduction
- https://business.1inch.com/portal/documentation/apis/swap/cross-chain-swap/orders/v1.2/order/status/orderHash/method/get
- https://github.com/1inch/limit-order-protocol
- https://github.com/1inch/cross-chain-swap
- https://github.com/1inch/fusion-sdk
- https://github.com/1inch/liquidity-protocol
