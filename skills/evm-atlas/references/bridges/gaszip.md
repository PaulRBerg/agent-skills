# Gas.zip Bridging

## Overview

Use Gas.zip as a read-only source for native-gas bridge quotes, per-chain limits and liquidity, deposit status, and
destination fills after the known origin and destination chains are confirmed against
`references/generated/target-mainnets.json`.

Default to the API base URL, which needs no key:

```text
https://backend.gas.zip
```

Never execute bridge steps from this skill. Returned `calldata` and `contractDepositTxn` objects are for inspection
only; route any deposit preparation, simulation, and broadcast to `cli-cast`.

Gas.zip is a solver-fill native-gas bridge: the user deposits native currency on the origin chain, and a Gas.zip signer
sends a plain native transfer (empty calldata) to the recipient on each destination chain. It identifies destinations by
its own `short` IDs, not chain IDs; resolve them through `GET /v2/chains` and never hardcode a table.

Contract deposits call `deposit(uint256,bytes32)` (selector `0xc9630cb0`). The `uint256` packs destination `short` IDs;
the `bytes32` is the recipient address left-aligned (right-padded with zeros), not ABI address padding. Verified
2026-10-01: GasZipV2 at `0x5D5a72859b8EBAFcf459164F64400012F5A3C5E0` on Arbitrum Nova (Blockscout-verified source).
Verify the contract on each other origin chain before treating a deposit as Gas.zip.

## Read-Only Router

1. **Known origin deposit tx hash:** call `GET /v2/deposit/{hash}`. `deposit` carries origin chain, block, sender,
   recipient, `shorts`, `value`, and `status`; `txs[]` lists destination fills with `chain`, `hash`, `signer`, `value`,
   and `status`. An empty `{}` is indexing lag, not absence: observed for about six minutes after the origin receipt on
   2026-10-01. Recheck later and verify the fill on the destination chain through provider routing. A `txs[]` entry with
   `refund: true`, `chain` equal to the origin, and an `errtime` is a refund to the sender on the origin chain, not a
   destination fill; it can stay `SEEN` until Gas.zip has origin-chain liquidity. Verify it by the sender's origin
   balance, not the signer nonce. `errtime` advances on each failed retry, so a recent value means Gas.zip is still
   retrying. Users have no on-chain recovery path: deposits stay in the deposit contract, which only its `owner` can
   `withdraw`, while fills and refunds are paid from the separate signer balance. Observed 2026-10-02: the Nova contract
   held about 1 ETH while the Nova signer `bal` was below the pending refund. For stuck refunds, compare the contract
   balance with the origin `bal` and point the user to Gas.zip support (Discord or Telegram, linked from
   `https://www.gas.zip/`) with the deposit hash.
2. **Known sender/recipient address:** call `GET /v2/user/{address}`. An empty `user` array has the same lag caveat.
3. **Quote:** call `GET /v2/quotes/{originChainId}/{amountWei}/{destinationChainIds}?from=<address>&to=<address>`.
   `quotes[].expected` is destination wei; `expires` is a Unix timestamp. The output is an estimate, not a fill
   guarantee.
4. **Supported chains, limits, and liquidity:** call `GET /v2/chains`. Per chain: `chain` (chain ID), `short`,
   `inbound`, `minInbound`/`maxInbound` and `minOutbound`/`maxOutbound` (USD), the `*Native` wei equivalents, and `bal`
   (destination liquidity in wei). Check the destination's `maxOutbound` and `bal` before relying on a quote. A quote
   above `maxOutbound` fails with `Chain Limit Exceeded`, so larger amounts need sequential deposits; re-quote and
   recheck `bal` before each, because fills drain it. Observed 2026-10-01: a deposit accepted while destination
   liquidity was short stayed `SEEN` with a null fill hash, and later quotes returned `Insufficent Liquidity`.

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
