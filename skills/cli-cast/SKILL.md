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

Invoke `$evm-atlas` before every network operation. It owns chain resolution, discrete reads, and bounded live
subscriptions. Pass the explicit chain name or ID, JSON-RPC method and exact parameters or call object, block selector
or checkpoint requirement, and why the result is needed. Require its read packet containing the resolved chain name and
ID, provider route, result, observed block or checkpoint, and coverage gaps.

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

For Ethereum mainnet, the default gas policy is [references/ethereum-gas.md](references/ethereum-gas.md): fetch a fresh
Rabby `slow` quote and bind its EIP-1559 fee pair before simulation. The user or a consuming skill may explicitly choose
a different gas policy, including a fixed legacy gas price for an exact-zero sweep. Honor that choice; it does not
require a separate policy-exception approval. Record its source, transaction type, fee values, and any constraints in
the transaction review. Apply the selected policy to every signer and do not silently substitute a different tier or
transaction type. Do not reuse Ethereum fee values on another chain.

#### Chain-specific gas accounting

Resolve the chain's active fee model before choosing a transaction type or subtracting fees from a balance. Use current
official protocol documentation for semantics and `$evm-atlas` for the target chain's parameters, fee-oracle calls,
estimates, and receipts. EVM compatibility, a native symbol of ETH, and support for legacy transactions do not establish
Ethereum fee semantics. A chain absent from atlas remains unsupported.

For the exact transaction, retain a public fee record containing the chain/checkpoint, active fork and sources,
transaction type, gas estimate and limit, price/caps, expected total fee, upfront fee reserve with each component and
margin, and refund behavior. Identify which charges are included in estimated gas and which are additional native
debits. Use arbitrary-precision integer arithmetic and round reserves up. Never count a component twice or subtract an
anticipated refund from the upfront funding requirement. Re-estimate when value, calldata, nonce, type, gas, or fees
change; serialization can change the L1 charge even for an empty-calldata transfer.

- **Ethereum-style accounting:** establish that all fees for this transaction are covered by gas. Reserve
  `gasLimit * gasPrice` for legacy, or `gasLimit * maxFeePerGas` for EIP-1559. Actual cost uses receipt `gasUsed` and
  `effectiveGasPrice`. Exact-zero native sweeps require evidence of a fixed charged price, fixed gas used, and no
  additional charges or credits; a plain undelegated EOA transfer on Ethereum with empty calldata and legacy pricing can
  use exactly 21000 gas. Do not apply that constant to other fee models.
- **Arbitrum Nitro:** use the complete `eth_estimateGas` result, or `NodeInterface.gasEstimateComponents()` through
  atlas. It includes the parent-chain posting charge converted into child-chain gas. Budget the full gas limit at the
  reviewed price cap; do not add a second L1 fee. Prefer EIP-1559 with zero priority fee: Nitro ignores tips and charges
  the inclusion base fee, so a legacy bid is also not a promise of the charged price. Allow unused gas/price headroom to
  remain with the sender. Use receipt `gasUsed * effectiveGasPrice` for total cost; `gasUsedForL1` is an included gas
  component, not an extra wei charge. See
  [gas and fees](https://docs.arbitrum.io/how-arbitrum-works/deep-dives/gas-and-fees) and
  [estimation](https://docs.arbitrum.io/arbitrum-essentials/how-to-estimate-gas).
- **OP Stack:** reserve execution gas plus the L1 data fee and any enabled operator fee. Through atlas, query the
  verified `GasPriceOracle` for `getL1Fee(bytes)` using the serialized unsigned transaction, and, where supported,
  `getL1FeeUpperBound(uint256)` using its unsigned byte length. These interfaces account for signature overhead; do not
  add it twice. The latter is a practical size bound at the current oracle prices, not a cap on fees at inclusion. Query
  `getOperatorFee(gasLimit)` for upfront budgeting and reconstruct the included operator charge using the inclusion
  fork's parameters, gas use, and exact client rounding/refund semantics. Before Isthmus it is absent; Isthmus and
  Jovian use different scalar formulas. Establish absence from fork/configuration evidence, not a failed call or a
  missing provider field. Receipt total is execution cost plus `l1Fee` plus operator cost, each once. See
  [transaction fees](https://docs.optimism.io/op-stack/transactions/fees),
  [Fjord oracle](https://specs.optimism.io/protocol/fjord/predeploys.html),
  [Isthmus operator accounting](https://specs.optimism.io/protocol/isthmus/exec-engine.html), and
  [Jovian changes](https://specs.optimism.io/protocol/jovian/exec-engine.html).
- **Other models, including ZKsync:** obtain the current chain-native estimator and receipt semantics through atlas and
  official docs. Establish coverage of pubdata, resource overhead, custom debits, and refunds for the actual transaction
  type. Use an ordinary type `0`/`2` transfer only if the chain and browser transport support it with complete fee
  coverage. Do not substitute a deprecated estimator or add custom transaction fields without a supported signing path.
  ZKsync's [fee structure](https://docs.zksync.io/zksync-protocol/era-vm/transactions/fee-model/fee-structure) includes
  pubdata/overhead and refunds; neither 21000 gas nor exact-zero accounting follows from EVM compatibility. Stop with
  the missing component when complete accounting cannot be established.

On chains with fees outside transaction caps, label the total as an **estimated reserve**, disclose the uncovered price
movement, and recheck affordability immediately before signing. A margin is not a protocol-enforced maximum. Require
`value + upfront fee reserve <= balance`; ordinary gas estimation or a successful `eth_call` alone does not prove this.
If final wallet fee edits are allowed, recompute dependent additional fees and affordability from those values. Sweeps
whose value depends on the reserve must preserve all reviewed fields under the fixed-fee exception below.

Refunded or unspent gas is normally already reflected in the final charged fee and sender balance; do not subtract it
again. Treat asynchronous refunds, such as Arbitrum retryable tickets, separately: never spend an expected credit before
it arrives or claim a snapshot balance is permanent. Discovery of unrelated pending refunds is not implicit in a plain
transfer. A consuming sweep must state its residual-balance policy and any known pending credits before approval.

### Simulate

Simulate the exact prepared call from the intended sender and value, then estimate gas. Delegate bounded `eth_call` and
`eth_estimateGas` evidence to `evm-atlas`. Use a local fork, project simulation, or Cast trace only when the simulation
requires a continuous provider, following Resolve Chain and Provider. A successful simulation is evidence, not
authorization to sign.

### Review

Before any signature or broadcast, present one concrete review containing:

- chain name and ID, RPC source, and latest block used;
- sender, target, function, decoded arguments, calldata, and native value;
- nonce, gas estimate/limit, fee assumptions, expected total cost, upfront reserve, and any protocol-enforced fee caps;
- chain-specific fee components, inclusion-price uncertainty outside those caps, and expected refunds or residuals;
- the selected gas policy and source; for Rabby Slow, the oracle URL, tier, quote time, estimated inclusion time, max
  fee per gas, and max priority fee per gas; for a legacy policy, the fixed gas price and transaction type;
- expected approvals, transfers, or other state changes;
- simulation command and outcome;
- selected signer and the exact signing/broadcast command with secrets redacted.

Lead the review with `### ⚠️ Transaction approval required`. Put repeated fields in a compact table, keep the exact
command in a fenced block, and state precisely what confirmation authorizes. Stop and require explicit user confirmation
of this review in a subsequent message. If any reviewed field changes outside the browser-wallet exception below,
simulate again and present a revised review.

For browser signing only, the reviewed gas limit and fees are starting values unless the consuming workflow requires
them to remain fixed. The user may deliberately change the gas limit, gas price, max fee per gas, or max priority fee
per gas in the wallet confirmation UI. Their approval of that final wallet screen authorizes those edited gas settings;
apply chain-specific accounting to the additional fees and resulting affordability. Do not stop, require a second
approval, or resimulate solely because they differ from the prepared values. Continue only when the chain, sender,
target, calldata, native value, nonce, and decoded intent still match the approved review. Wallet changes to any of
those fields require rejection and a revised review.

When fees determine the transfer value or another reviewed invariant, such as leaving exactly zero native balance, the
browser exception does not apply. Preserve the reviewed transaction type, gas limit, and fee values. If the wallet
changes them, reject before signing, recompute the dependent values, simulate, and obtain approval of the revised
review.

### Sign and Broadcast

Read [references/browser-signing.md](references/browser-signing.md) for browser capability checks and sender handling;
open a signing request only after approval. Prefer browser, encrypted keystore, or hardware wallet in that order unless
the user or consuming skill restricts the signer. A browser-only workflow must stop if browser signing is unavailable;
never substitute another signer. Use an environment-backed private key only when the user explicitly opts in or no safer
method is available; never ask for a key in chat or print it.

`cast send` signs and broadcasts in one command. Run it only after the review approval. Signing a message or typed data
also requires a review of the exact payload, domain, chain binding, and intended use before approval.

Pass the selected fees explicitly: EIP-1559 uses `--gas-price` and `--priority-gas-price`; a fixed legacy policy uses
`--legacy --gas-price` without `--priority-gas-price`. Under the default Ethereum policy, use the approved Rabby Slow
pair. Before opening the signer, recheck the active chain's gas and additional-fee requirements and that the approved
cap or legacy gas price covers its current base fee where applicable. If fees must change before signing, simulate again
and present a revised review; never silently change the selected policy. Wallet fee edits follow Review, including its
fixed-fee exception. Message and typed-data signatures consume no gas.

After broadcast, capture the transaction hash and have `evm-atlas` verify the receipt on the reviewed chain. When a
receipt is still pending, it may use one bounded RouteMesh `newHeads` subscription to wait for the next block before
checking again. Receipt verification remains required; a pending-transaction or log notification alone cannot confirm
the transaction. Reconcile actual fees under the active chain's model, including additional receipt components and
refunds without double counting. Missing fee evidence is incomplete accounting, not zero fees. Report status, block, gas
used, actual total fee or its exact evidence gap, and the explorer link under `### ✅ Transaction confirmed` for a
successful receipt or `### ↩ Transaction reverted` for a mined failure. For an ambiguous outcome, lead with
`### ⛔ Broadcast unresolved — do not retry` and state the evidence still needed. Do not retry a failed or uncertain
broadcast without first checking whether the transaction exists — for browser-wallet signing specifically, read
[references/browser-signing.md](references/browser-signing.md)'s Timing and Recovering sections before concluding
nothing was sent: a killed or timed-out process does not prove non-broadcast, since wallet approval is an unbounded
human wait and the wallet may broadcast via its own RPC provider.

## Stop Conditions

Stop before signing when the signer, sender, chain, target, decoded intent, fee accounting, affordability, or simulation
result is unresolved. Review must distinguish enforced caps from an estimated reserve; browser approval of edited gas
settings authorizes those settings but does not establish coverage of omitted chain-specific charges. Stop before
retrying when broadcast outcome is ambiguous. Completion requires either a verified read result, a local encoding
result, an approved signature artifact, or a mined receipt verified by `evm-atlas` that matches the reviewed transaction
apart from user-approved browser-wallet gas settings. Never decorate or truncate addresses, calldata, signatures,
hashes, RPC URLs, fee values, commands, or safety wording.
