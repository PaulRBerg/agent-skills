---
coordination: exempt
name: cli-cast
skill-dependencies:
  - evm-atlas
user-invocable: false
description:
  "Use for Foundry cast/on-chain actions: prepare, simulate, sign, or broadcast transactions; call contracts; sign
  messages; encode/decode ABI/calldata; or make RPC calls. For read-only balances/history, prefer evm-atlas."
---

# Foundry Cast CLI

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Separate read, preparation, simulation, signing, and broadcast so no state-changing action is hidden inside command
construction.

## Resolve Chain and RPC

Resolve the target chain through `evm-atlas`. Use RouteMesh only when that chain is supported and `ROUTEMESH_API_KEY` is
available:

```text
https://lb.routeme.sh/rpc/CHAIN_ID/ROUTEMESH_API_KEY
```

Otherwise use the target chain's verified public RPC. Read [references/chains.md](references/chains.md) only as a
limited fallback when `evm-atlas` is unavailable. Never infer Ethereum when the chain is ambiguous.

## Authority Phases

### Read

Read-only calls, transaction/receipt inspection, ABI encoding/decoding, selector lookup, ENS resolution, and RPC queries
may run without a transaction approval. Prefer `evm-atlas` for balances, holdings, transfers, and history.

Use current `cast <command> --help` for exact syntax. Typical operations:

```sh
cast call "$CONTRACT" 'balanceOf(address)(uint256)' "$ACCOUNT" --rpc-url "$RPC_URL"
cast tx "$TX_HASH" --rpc-url "$RPC_URL"
cast receipt "$TX_HASH" --rpc-url "$RPC_URL"
cast calldata 'transfer(address,uint256)' "$TO" "$AMOUNT"
cast decode-calldata 'transfer(address,uint256)' "$CALLDATA"
```

For two or more compatible reads on one chain, batch through Multicall3 at `0xcA11bde05977b3631167028862bE2a173976CA11`.
Do not batch calls whose result depends on the original `msg.sender`.

### Prepare

Resolve and validate chain ID, sender, target, function signature, arguments, calldata, native value, nonce, and fee
assumptions without signing. Do not request or load key material during preparation.

For an Ethereum mainnet transaction, read [references/ethereum-gas.md](references/ethereum-gas.md), fetch a fresh Rabby
`slow` quote, and bind its EIP-1559 fee pair to the transaction before simulation. Apply this policy to every signer;
never let a browser wallet, keystore, hardware wallet, or private-key flow silently select Normal or Fast. Do not reuse
Ethereum fee values on another chain.

### Simulate

Simulate the exact prepared call from the intended sender and value, then estimate gas. Use a fork or project simulation
when available; otherwise use read-only `cast call`, `cast estimate`, and trace facilities. A successful simulation is
evidence, not authorization to sign.

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
of this review in a subsequent message. If any reviewed field changes, simulate again and present a revised review.

### Sign and Broadcast

Read [references/browser-signing.md](references/browser-signing.md) only after approval when a browser wallet is
available. Prefer browser, encrypted keystore, or hardware wallet in that order. Use an environment-backed private key
only when the user explicitly opts in or no safer method is available; never ask for a key in chat or print it.

`cast send` signs and broadcasts in one command. Run it only after the review approval. Signing a message or typed data
also requires a review of the exact payload, domain, chain binding, and intended use before approval.

Every Ethereum mainnet transaction command must use the approved Rabby Slow values explicitly as `--gas-price` and
`--priority-gas-price`, regardless of signer. Before broadcast, recheck that the approved max fee is not below the
latest base fee. If the quote must change, simulate again and present a revised review; never upgrade to Normal or Fast
as a fallback. These requirements do not apply to message or typed-data signatures because they consume no gas.

After broadcast, capture the transaction hash and verify the receipt on the reviewed chain. Report status, block, gas
used, and the explorer link under `### ✅ Transaction confirmed` for a successful receipt or
`### ↩ Transaction reverted` for a mined failure. For an ambiguous outcome, lead with
`### ⛔ Broadcast unresolved — do not retry` and state the evidence still needed. Do not retry a failed or uncertain
broadcast without first checking whether the transaction exists — for browser-wallet signing specifically, read
[references/browser-signing.md](references/browser-signing.md)'s Timing and Recovering sections before concluding
nothing was sent: a killed or timed-out process does not prove non-broadcast, since wallet approval is an unbounded
human wait and the wallet may broadcast via its own RPC provider.

## Stop Conditions

Stop before signing when the signer, sender, chain, target, decoded intent, cost ceiling, or simulation result is
unresolved. Stop before retrying when broadcast outcome is ambiguous. Completion requires either a verified read result,
an approved signature artifact, or a mined receipt matching the reviewed transaction. Never decorate or truncate
addresses, calldata, signatures, hashes, RPC URLs, fee values, commands, or safety wording.
