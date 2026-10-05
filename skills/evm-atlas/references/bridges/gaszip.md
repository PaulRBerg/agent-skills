# Gas.zip Bridging

## Overview

Use Gas.zip after confirming the known origin and destination chains against
`references/generated/target-mainnets.json`. Use it as a read-only source for native-gas bridge quotes, per-chain limits
and liquidity, deposit status, and destination fills.

Default to the API base URL, which needs no key:

```text
https://backend.gas.zip
```

Never execute bridge steps from this skill. Returned `calldata` and `contractDepositTxn` objects are for inspection
only. Route any deposit preparation, simulation, and broadcast to `cli-cast`.

Gas.zip is a solver-fill native-gas bridge. The user deposits native currency on the origin chain. A Gas.zip signer
sends a plain native transfer (empty calldata) to the recipient on each destination chain. Gas.zip identifies
destinations by its own `short` IDs, not chain IDs. Resolve them through `GET /v2/chains`. Never hardcode a table.

Contract deposits call `deposit(uint256,bytes32)` (selector `0xc9630cb0`). The `uint256` packs destination `short` IDs.
The `bytes32` is the recipient address left-aligned (right-padded with zeros), not ABI address padding. Verified
2026-10-01: GasZipV2 at `0x5D5a72859b8EBAFcf459164F64400012F5A3C5E0` on Arbitrum Nova (Blockscout-verified source).
Verify the contract on each other origin chain before treating a deposit as Gas.zip.

## Read-Only Router

1. **Known origin deposit tx hash:** call `GET /v2/deposit/{hash}`. `deposit` carries origin chain, block, sender,
   recipient, `shorts`, `value`, and `status`. `txs[]` lists destination fills with `chain`, `hash`, `signer`, `value`,
   and `status`. An empty `{}` is indexing lag, not absence. This lasted about six minutes after the origin receipt on
   2026-10-01.

   Recheck later. Verify the fill on the destination chain through provider routing.

   A `txs[]` entry with `refund: true`, `chain` equal to the origin, and an `errtime` is a refund to the sender on the
   origin chain, not a destination fill. It can stay `SEEN` until Gas.zip has origin-chain liquidity. Verify it by the
   sender's origin balance, not the signer nonce. `errtime` advances on each failed retry. Thus, a recent value means
   Gas.zip is still retrying.

   Users have no on-chain recovery path. Deposits stay in the deposit contract, which only its `owner` can `withdraw`.
   The separate signer balance pays for fills and refunds. Observed 2026-10-02: the Nova contract held about 1 ETH while
   the Nova signer `bal` was below the pending refund.

   For stuck refunds, compare the contract balance with the origin `bal`. For those refunds, point the user to Gas.zip
   support with the deposit hash. Support is available through Discord or Telegram, linked from `https://www.gas.zip/`.

2. **Known sender/recipient address:** call `GET /v2/user/{address}`. An empty `user` array has the same lag caveat.
3. **Quote:** call `GET /v2/quotes/{originChainId}/{amountWei}/{destinationChainIds}?from=<address>&to=<address>`.
   `quotes[].expected` is destination wei. `expires` is a Unix timestamp. The output is an estimate, not a fill
   guarantee.
4. **Supported chains, limits, and liquidity:** call `GET /v2/chains`. Per chain: `chain` (chain ID), `short`,
   `inbound`, `minInbound`/`maxInbound` and `minOutbound`/`maxOutbound` (USD), the `*Native` wei equivalents, and `bal`
   (destination liquidity in wei). Check the destination's `maxOutbound` and `bal` before relying on a quote. A quote
   above `maxOutbound` fails with `Chain Limit Exceeded`. Thus, larger amounts need sequential deposits.

   Before each such deposit, obtain a new quote. Before each, recheck `bal` because fills drain it. Observed 2026-10-01:
   a deposit accepted while destination liquidity was short stayed `SEEN` with a null fill hash, and later quotes
   returned `Insufficent Liquidity`.

Example status lookup:

```bash
curl -sS "https://backend.gas.zip/v2/deposit/0xTX_HASH"
```

## Coverage Notes

Verified 2026-10-01: for Arbitrum Nova to Arbitrum One native ETH, Relay and Across listed no Nova route, Layerswap
reported the asset unsupported, and LI.FI's only route was Gas.zip with an added LI.FI fee. Recheck live coverage before
reusing this conclusion.

## Sources

- https://dev.gas.zip/
- https://arbitrum-nova.blockscout.com/address/0x5D5a72859b8EBAFcf459164F64400012F5A3C5E0
