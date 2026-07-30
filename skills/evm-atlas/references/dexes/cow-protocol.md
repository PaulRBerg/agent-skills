# CoW Protocol Evidence

Use this reference after `references/workflows/dex-transactions.md` for CoW Swap, CoWSwap, CoW Protocol, and GPv2
orders, approvals, cancellations, EthFlow refunds, fills, and settlements. CoW Swap is an interface; CoW Protocol is the
execution protocol and retains historical GPv2 contract names.

Do not create, sign, submit, or cancel orders. Do not add CoW AMM liquidity-position interpretation; a CoW AMM
interaction observed inside a settlement may be named only as an underlying liquidity source.

## Deployment and Identity

Resolve the chain, then use CoW's current contract reference/deployment feed and verified explorer source for
`GPv2Settlement`, `GPv2VaultRelayer`, and EthFlow. Verify runtime code and proxy/implementation state where applicable.
Do not copy a deployment table into this reference.

An order UID is 56 bytes: order digest, owner, and `validTo`. Preserve it in full. Verify its digest against the
chain-specific EIP-712 domain and exact order fields when those fields and signature are available. Supported signature
schemes include EIP-712, `eth_sign`, ERC-1271, and pre-signature; report the observed scheme rather than assuming an EOA
signature.

## Read-Only Order Book API

Use the public chain-specific Order Book API only when the user supplies or on-chain evidence resolves a known order
UID, settlement transaction hash, or owner address:

- `GET /api/v1/orders/{UID}` and `/api/v1/orders/{UID}/status` for one known order.
- `GET /api/v1/trades` or `/api/v2/trades` filtered to the known UID for fills.
- `GET /api/v1/transactions/{txHash}/orders` for orders in a known settlement.
- `GET /api/v1/account/{owner}/orders` only for an explicitly requested, chain-scoped owner history.
- `GET /api/v1/app_data/{app_data_hash}` for known `appData`.

Do not call quote, auction-discovery, order-creation, cancellation, or app-data registration endpoints. If the chain is
missing or the identifier cannot select a chain-specific API, ask; do not default or query every API deployment.

Preserve API-native status and executed amounts. Verify fulfilled, cancelled, expired, partially filled, and
EthFlow-refund claims against finalized on-chain evidence where such evidence exists. An API-only off-chain cancellation
remains a provider fact with an on-chain coverage gap.

## Orders, Approvals, and Lifecycle

- Record sell/buy tokens and amounts, receiver, `validTo`, kind, partial-fill flag, balance source/destination,
  `feeAmount`, signature scheme, owner, UID, and `appData` only from the order/API/signature evidence.
- A CoW order signature and API submission are off-chain. Do not invent a maker transaction or maker-paid settlement
  gas.
- Direct ERC-20 approvals normally target `GPv2VaultRelayer`, not `GPv2Settlement`. Report the actual spender and
  distinguish ERC-20 allowance from Balancer external/internal-balance authorization.
- An approval, pre-signature, or signature revocation is not a fill. Use `PreSignature` and exact state/call evidence.
- On-chain `invalidateOrder` and `OrderInvalidated` prove protocol invalidation. API-side signed cancellation can
  explain Order Book status but has no transaction receipt. Expiry requires the signed `validTo`, a finalized timestamp
  after it, and no later fill evidence.
- For partially fillable orders, report each `Trade` and cumulative executed amount. A cancelled or expired order may
  still have earlier partial fills.

Do not use `filledAmounts` as sole historical proof: storage may be cleared after expiry. Use receipt logs and indexed
trades.

## Settlement Evidence

`GPv2Settlement.settle` can execute many orders in one transaction and arbitrary interactions with underlying liquidity.

1. Decode the verified settlement ABI, including token list, clearing prices, trades, and pre/intra/post interactions.
2. Match the target UID/digest/owner to its `Trade` event and API trade record. Do not assign all settlement transfers
   or gas to one wallet.
3. Use the `Settlement` event and transaction sender for solver evidence. Use `Interaction` events, calldata, traces,
   and downstream pool events for underlying AMMs.
4. Report execution protocol as CoW Protocol, integration wrapper if any, and underlying liquidity separately.
5. Compute the target wallet's sold/received assets from its trade and transfers. Report solver-paid transaction gas
   separately.

If an explorer oddly decodes the settlement selector, recompute the first four bytes from the official ABI, verify the
calldata target and runtime implementation, and decode locally. Keep the explorer name as a hint and the raw selector as
observed evidence.

## EthFlow

EthFlow wraps a user's native ETH into WETH and creates an ERC-1271 CoW order through an intermediary contract.

- For creation, require a successful `createOrder`, native value, `OrderPlacement`, stored owner/validity, and the
  derived contract order. Report the user intent (ETH) and settlement intent (WETH) without collapsing them.
- The EthFlow contract is the order signer/owner while the user is the native depositor and buy-token receiver.
- For fills, connect the EthFlow order to the settlement `Trade` and recipient transfer.
- For invalidation/refund, require `invalidateOrder` evidence and the actual unmatched native return. Expiry or API
  status alone does not prove refund.

## Fees, Surplus, and appData

- Separate signed `feeAmount`, executed protocol/partner fee fields, solver gas, and wallet asset deltas. Do not infer a
  fee from the spread between current market price and execution.
- Report surplus or price improvement only when the Order Book/solver-competition data or clearing-price accounting for
  the exact settlement proves it. Preserve the provider and formula.
- Resolve `appData` by its hash through the public read endpoint or IPFS/schema evidence. Treat app code, partner fee,
  referral, hooks, and metadata as declared data unless on-chain effects independently confirm them.

## Failure and Coverage

- Missing API record: continue with receipt/log evidence; do not conclude the order never existed.
- API `fulfilled` without a resolvable finalized `Trade`: report the API status and an on-chain verification gap.
- Failed settlement: no orders in it filled and no interactions survived; report only attempted calldata and gas.
- Multi-order settlement with unavailable traces: report the target `Trade` and wallet deltas, but leave underlying
  liquidity and interaction semantics incomplete.
- Unverified settlement, relayer, EthFlow deployment, or fork: retain raw evidence and leave canonical attribution
  unknown.

## Sources

- https://api.cow.fi/docs/
- https://docs.cow.fi/cow-protocol/reference/apis/orderbook
- https://docs.cow.fi/cow-protocol/reference/contracts/core
- https://docs.cow.fi/cow-protocol/reference/contracts/core/settlement
- https://docs.cow.fi/cow-protocol/reference/contracts/core/vault-relayer
- https://docs.cow.fi/cow-protocol/reference/contracts/periphery/eth-flow
- https://docs.cow.fi/cow-protocol/reference/core/intents
- https://docs.cow.fi/cow-protocol/reference/core/intents/app-data
- https://docs.cow.fi/cow-protocol/reference/core/signing-schemes
- https://github.com/cowprotocol/services
