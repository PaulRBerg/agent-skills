# Arbitrum Canonical Bridge Withdrawals

## Overview

Use this reference for read-only status of an Arbitrum One or Arbitrum Nova (Nitro) native withdrawal to Ethereum. A
withdrawal is an L2 `ArbSys` (`0x0000000000000000000000000000000000000064`) call, such as `withdrawEth(address)`. It
becomes claimable on Ethereum only after an assertion covering its L2 block is confirmed. Confirmation takes roughly 6.4
days. Never execute the claim from this skill. Route the `Outbox.executeTransaction` signing and broadcast to
`cli-cast`. Anyone can submit the claim. The funds always go to the recorded destination, so the gas payer may differ
from the withdrawer.

Resolve contract addresses on-chain instead of hardcoding them. On Ethereum, `Outbox.rollup()` returns the rollup, and
`Rollup.outbox()` must point back. Verified 2026-10-10 for Nova: Outbox `0xD4B80C3D7240325D18E645B49e6535A3Bf95cc58` and
Rollup `0xE7E8cCC7c381809BDC4b213CE44016300707B7Bd`.

## Read-Only Router

1. **Withdrawal facts:** read the L2 receipt and select the `ArbSys` log whose topic0 is
   `L2ToL1Tx(address,address,uint256,uint256,uint256,uint256,uint256,uint256,bytes)`. Its indexed topics are
   `destination`, `hash` (the outbox leaf), and `position` (the withdrawal index). Its data holds `caller`,
   `arbBlockNum`, `ethBlockNum`, `timestamp`, `callvalue`, and `data`.
2. **Already claimed:** call `Outbox.isSpent(uint256 position)` on Ethereum. A `true` result means claimed, and the
   claim tx is the Outbox `OutBoxTransactionExecuted` log for that index.
3. **Confirmed:** read the newest rollup `AssertionConfirmed(bytes32,bytes32,bytes32)` log on Ethereum. Its data holds
   `blockHash` and `sendRoot`. Fetch that L2 block with `eth_getBlockByHash`. Nitro blocks expose `sendCount` and
   `sendRoot`. The withdrawal is confirmed when `position < sendCount`.
4. **Claimable proof:** on L2, `eth_call` NodeInterface (`0x00000000000000000000000000000000000000C8`)
   `constructOutboxProof(uint64 size, uint64 leaf)` with `size = sendCount` and `leaf = position`. It returns
   `(send, root, proof)`. Require `send` to equal the log's `hash` topic and `root` to equal the confirmed `sendRoot`.
   Then simulate
   `Outbox.executeTransaction(proof, position, caller, destination, arbBlockNum, ethBlockNum, timestamp, callvalue, data)`
   with `eth_call` and `eth_estimateGas` from the intended payer. That simulation is the claimability evidence handed to
   `cli-cast`.
5. **Unconfirmed ETA:** confirmation lag is the newest confirmation's L1 timestamp minus its L2 block timestamp. Add
   that lag to the withdrawal's L2 timestamp. Report the result as an estimate. Assertion cadence adds roughly an hour
   of jitter.
