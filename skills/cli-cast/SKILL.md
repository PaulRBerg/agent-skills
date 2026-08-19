---
coordination: exempt
name: cli-cast
skill-dependencies:
  - evm-atlas
user-invocable: false
description:
  "Use for Foundry cast transaction actions: prepare, trace, simulate, sign, or broadcast; sign messages; or
  encode/decode ABI/calldata. Delegate every standalone RPC read to evm-atlas."
---

# Foundry Cast CLI

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Separate read, preparation, simulation, signing, and broadcast so no state-changing action is hidden inside command
construction.

## Resolve Chain and Provider

Invoke `$evm-atlas` before every network operation. It owns chain resolution and all discrete reads. Pass the explicit
chain name or ID, JSON-RPC method and exact parameters or call object, block selector or checkpoint requirement, and why
the result is needed. Require its read packet containing the resolved chain name and ID, provider route, result,
observed block or checkpoint, and coverage gaps.

If the chain is absent from `evm-atlas`, stop RPC-dependent work. Do not accept an arbitrary RPC URL as an escape hatch
and never infer Ethereum when the chain is ambiguous.

Use an RPC URL in this skill only for a trace, fork, or send flow that cannot be expressed as bounded discrete reads.
For RouteMesh-backed continuous transport, first require `evm-atlas` to confirm current RouteMesh coverage, then use the
explicit Foundry alias when it is configured:

```sh
ROUTEMESH_CHAIN_ID="$CHAIN_ID" cast COMMAND --rpc-url routemesh
```

Otherwise use the nonsecret public RPC verified by `evm-atlas`. Never construct, inspect, or print a RouteMesh URL.
Treat Cast stderr as secret-bearing because Foundry may reveal a resolved alias URL on transport failure; never repeat
that output in chat, logs, or external reports.

## Authority Phases

### Read

Local ABI encoding/decoding and selector derivation may run without transaction approval. Delegate chain, block, fee,
nonce, `eth_call`, `eth_estimateGas`, transaction, receipt, log, balance, code, storage, proof, and ENS reads to
`$evm-atlas`, including reads needed to prepare or verify a transaction. Do not ask `evm-atlas` to hand a read back to
this skill merely because later work may change state.

Use current `cast <command> --help` for exact local syntax. Typical local operations:

```sh
cast calldata 'transfer(address,uint256)' "$TO" "$AMOUNT"
cast decode-calldata 'transfer(address,uint256)' "$CALLDATA"
```

### Prepare

Resolve and validate chain ID, sender, target, function signature, arguments, calldata, native value, nonce, and fee
assumptions without signing. Obtain every on-chain fact through `evm-atlas`; do not request or load key material during
preparation.

For an Ethereum mainnet transaction, read [references/ethereum-gas.md](references/ethereum-gas.md), fetch a fresh Rabby
`slow` quote, and bind its EIP-1559 fee pair to the transaction before simulation. Apply this policy to every signer;
never let a keystore, hardware-wallet, or private-key flow silently select Normal or Fast. For browser signing, use the
quote as the initial wallet request; the user may deliberately edit gas settings in the wallet confirmation UI under the
Review rules below. Do not reuse Ethereum fee values on another chain.

### Simulate

Simulate the exact prepared call from the intended sender and value, then estimate gas. Delegate bounded `eth_call` and
`eth_estimateGas` evidence to `evm-atlas`. Use a local fork, project simulation, or Cast trace only when the simulation
requires a continuous provider, following Resolve Chain and Provider. A successful simulation is evidence, not
authorization to sign.

### Review

Before any signature or broadcast, present one concrete review containing:

- chain name and ID, RPC source, and latest block used;
- sender, target, function, decoded arguments, calldata, and native value;
- nonce, gas estimate/limit, fee assumptions, and maximum native-token cost;
- for Ethereum mainnet, the Rabby oracle URL, `slow` tier, quote time, estimated inclusion time, max fee per gas, and
  max priority fee per gas;
- expected approvals, transfers, or other state changes;
- simulation command and outcome;
- selected signer and the exact signing/broadcast command with secrets redacted.

Lead the review with `### ⚠️ Transaction approval required`. Put repeated fields in a compact table, keep the exact
command in a fenced block, and state precisely what confirmation authorizes. Stop and require explicit user confirmation
of this review in a subsequent message. If any reviewed field changes outside the browser-wallet exception below,
simulate again and present a revised review.

For browser signing only, the reviewed gas limit and fee caps are starting values. The user may deliberately change the
gas limit, max fee per gas, or max priority fee per gas in the wallet confirmation UI. Their approval of that final
wallet screen authorizes those edited gas settings and the resulting maximum transaction cost; do not stop, require a
second approval, or resimulate solely because they differ from the prepared values. Continue only when the chain,
sender, target, calldata, native value, nonce, and decoded intent still match the approved review. Wallet changes to any
of those fields require rejection and a revised review.

### Sign and Broadcast

Read [references/browser-signing.md](references/browser-signing.md) only after approval when a browser wallet is
available. Prefer browser, encrypted keystore, or hardware wallet in that order. Use an environment-backed private key
only when the user explicitly opts in or no safer method is available; never ask for a key in chat or print it.

`cast send` signs and broadcasts in one command. Run it only after the review approval. Signing a message or typed data
also requires a review of the exact payload, domain, chain binding, and intended use before approval.

Every Ethereum mainnet transaction command must start with the approved Rabby Slow values explicitly as `--gas-price`
and `--priority-gas-price`, regardless of signer. Before opening the signer, recheck that the approved max fee is not
below the latest base fee. If the quote must change before signing, simulate again and present a revised review; never
upgrade to Normal or Fast as a fallback. A browser-wallet user may override those starting gas settings in its
confirmation UI as described under Review. These requirements do not apply to message or typed-data signatures because
they consume no gas.

After broadcast, capture the transaction hash and have `evm-atlas` verify the receipt on the reviewed chain. Report
status, block, gas used, and the explorer link under `### ✅ Transaction confirmed` for a successful receipt or
`### ↩ Transaction reverted` for a mined failure. For an ambiguous outcome, lead with
`### ⛔ Broadcast unresolved — do not retry` and state the evidence still needed. Do not retry a failed or uncertain
broadcast without first checking whether the transaction exists — for browser-wallet signing specifically, read
[references/browser-signing.md](references/browser-signing.md)'s Timing and Recovering sections before concluding
nothing was sent: a killed or timed-out process does not prove non-broadcast, since wallet approval is an unbounded
human wait and the wallet may broadcast via its own RPC provider.

## Stop Conditions

Stop before signing when the signer, sender, chain, target, decoded intent, cost ceiling, or simulation result is
unresolved; for browser signing, the user's approval of final wallet-edited gas settings resolves the cost ceiling. Stop
before retrying when broadcast outcome is ambiguous. Completion requires either a verified read result, a local encoding
result, an approved signature artifact, or a mined receipt verified by `evm-atlas` that matches the reviewed transaction
apart from user-approved browser-wallet gas settings. Never decorate or truncate addresses, calldata, signatures,
hashes, RPC URLs, fee values, commands, or safety wording.
