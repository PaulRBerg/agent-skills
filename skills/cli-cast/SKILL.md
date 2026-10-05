---
coordination: exempt
name: cli-cast
skill-dependencies:
  - evm-atlas
user-invocable: false
description:
  "Use for Foundry cast transaction actions: prepare, trace, simulate, sign, or broadcast. Also use to sign messages or
  encode/decode ABI/calldata. Delegate every standalone RPC read to evm-atlas."
---

# Foundry Cast CLI

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Separate read, preparation, simulation, signing, and broadcast. Do not hide state-changing actions inside command
construction.

For EIP-7702 authorization or delegation revocation, read [references/eip7702.md](references/eip7702.md) before
preparation. It defines signer selection, authority versus transaction sender, and conditional staged approval. It also
defines authorization verification in addition to the transaction receipt.

## Resolve Chain and Provider

Invoke `$evm-atlas` before every network operation. It owns chain resolution, discrete reads, and bounded live
subscriptions. Pass the explicit chain name or ID, JSON-RPC method, exact parameters or call object, and block selector
or checkpoint requirement. State why you need the result. Require its read packet containing the resolved chain name and
ID, provider route, result, observed block or checkpoint, and coverage gaps.

If the chain is absent from `evm-atlas`, stop RPC-dependent work. Do not accept an arbitrary RPC URL to bypass that
restriction. When the chain is ambiguous, never infer Ethereum.

Use an RPC URL in this skill only for a trace, fork, or send flow you cannot express as bounded discrete reads. For
RouteMesh-backed continuous transport, first require `evm-atlas` to confirm current RouteMesh coverage. Then, if the
explicit Foundry alias is configured, use it:

```sh
ROUTEMESH_CHAIN_ID="$CHAIN_ID" cast COMMAND --rpc-url routemesh
```

Otherwise, use the nonsecret public RPC that `evm-atlas` verified. Never construct, inspect, or print a RouteMesh URL.
Treat Cast stderr as secret-bearing because Foundry may reveal a resolved alias URL on transport failure. Never repeat
that output in chat, logs, or external reports.

Resolve every RPC value from `evm-atlas`'s current read packet or the RouteMesh alias above. A hard-coded public RPC URL
in a prepare, simulate, or verify command violates this delegation even when the chain matches. This includes a literal
`https://` endpoint from memory or a prior response.

When a local `cast` command against that provider returns a `--json` hex quantity, decode it. Use `cast to-dec <hex>` or
a field-select form such as `cast receipt <hash> <field>` or `cast tx <hash> <field>`. Do not pipe `--json` output
through `jq tonumber`. It fails on hex strings and on `null`.

## Authority Phases

An instruction to execute a contract call authorizes preparation, simulation, signing, broadcast, and necessary wallet
confirmations within the requested scope. Present the review. Then proceed without a separate chat approval or manual
wallet click from the user. Apply existing authorization across phases. A read-only, prepare-only, or simulation-only
request does not authorize execution.

Explicit user instructions take precedence over skill defaults. Honor user-imposed approval gates and host restrictions.
Ask only when execution needs authority or a material decision the user has not supplied. Message/typed-data signatures
and EIP-7702 authorizations retain their separate payload and use approvals below.

### Read

Local ABI encoding/decoding and selector derivation may run without transaction approval. Delegate chain, block, fee,
nonce, `eth_call`, `eth_estimateGas`, transaction, receipt, log, balance, code, storage, proof, and ENS reads to
`$evm-atlas`, including reads needed to prepare or verify a transaction. Do not ask `evm-atlas` to hand a read back to
this skill merely because later work may change state.

Use `env -i PATH="$PATH" cast <command> --help` for exact local syntax. Cast help can print inherited environment
values, including API keys. Do not load credentials or decrypted dotenv for capability checks. Typical local operations:

```sh
cast calldata 'transfer(address,uint256)' "$TO" "$AMOUNT"
cast decode-calldata 'transfer(address,uint256)' "$CALLDATA"
```

### Prepare

Without signing, resolve chain ID, sender, target, function signature, arguments, calldata, native value, nonce, and fee
assumptions. Without signing, validate those fields and assumptions. Obtain every on-chain fact through `evm-atlas`. Do
not request or load key material during preparation. Before simulation or helper construction, select a supported signer
under Sign and Broadcast. Before either activity, check that signer's command capabilities.

Request both the confirmed nonce at the numeric checkpoint block and the pending nonce. When they differ, or a wallet
reports `replacement transaction underpriced`, a queued transaction occupies the lowest unconfirmed nonce. Later nonces
stay stuck behind that transaction. Target that nonce instead of queueing another transaction behind it. Use fees that
clear the node's replacement bump, usually 10-20% on both the fee cap and the tip.

For Ethereum mainnet, [references/ethereum-gas.md](references/ethereum-gas.md) defines the default gas policy. Under
that policy, fetch a fresh Rabby `slow` quote before simulation. Bind its EIP-1559 fee pair before simulation.

The user or a consuming skill may explicitly choose a different gas policy. Their choice may include a fixed legacy gas
price for an exact-zero sweep. Honor that choice. It does not require a separate policy-exception approval. Record its
source, transaction type, fee values, and any constraints in the transaction review.

Apply the selected policy to every signer. Do not silently substitute a different tier or transaction type. Do not reuse
Ethereum fee values on another chain.

On other chains without a selected policy, set an EIP-1559 max fee with headroom over the latest base fee. Use a value
such as `2 * baseFee + priorityFee` instead of passing `eth_gasPrice` as the cap. The charge stays base fee plus tip. An
exact cap can fall below the base fee before signing and force a revised review.

#### Chain-specific gas accounting

Resolve the chain's active fee model before choosing a transaction type or subtracting fees from a balance. Use current
official protocol documentation for semantics and `$evm-atlas` for the target chain's parameters, fee-oracle calls,
estimates, and receipts. EVM compatibility, a native symbol of ETH, and support for legacy transactions do not establish
Ethereum fee semantics. A chain absent from atlas remains unsupported.

For the exact transaction, retain a public fee record with:

- chain/checkpoint, active fork, and sources,
- transaction type, gas estimate and limit, and price/caps,
- expected total fee,
- upfront fee reserve with each component and margin,
- refund behavior.

Identify which charges estimated gas includes and which charges cause additional native debits. Use arbitrary-precision
integer arithmetic. Round reserves up. Never count a component twice or subtract an anticipated refund from the upfront
funding requirement. When value, calldata, nonce, type, gas, or fees change, re-estimate. Serialization can change the
L1 charge even for an empty-calldata transfer.

- **Ethereum-style accounting:** establish that gas covers all fees for this transaction. Reserve `gasLimit * gasPrice`
  for legacy, or `gasLimit * maxFeePerGas` for EIP-1559. Actual cost uses receipt `gasUsed` and `effectiveGasPrice`.
  Exact-zero native sweeps require evidence of a fixed charged price, fixed gas used, and no additional charges or
  credits. A plain undelegated EOA transfer on Ethereum with empty calldata and legacy pricing can use exactly 21000
  gas. Do not apply that constant to other fee models.
- **Arbitrum Nitro:** use the complete `eth_estimateGas` result, or `NodeInterface.gasEstimateComponents()` through
  atlas. It includes the parent-chain posting charge converted into child-chain gas. Budget the full gas limit at the
  reviewed price cap. Do not add a second L1 fee.

  Arbitrum Nitro supports legacy transactions, but their charged price depends on the active ArbOS version and
  tip-collection setting. Verify that behavior through atlas and the current
  [fee processor](https://github.com/OffchainLabs/nitro/blob/master/arbos/tx_processor.go). Newer versions make tip
  collection configurable. When tips are disabled, prefer EIP-1559 with zero priority fee. Under that condition, legacy
  bids and equal caps still charge the inclusion base fee. When tips are collected, apply the active effective-price
  rules.

  A fixed charged price alone does not fix the variable posting-gas component or prove an exact-zero sweep. Reconcile
  unused gas/price headroom against the reviewed residual policy. Use receipt `gasUsed * effectiveGasPrice` for total
  cost. `gasUsedForL1` is an included gas component, not an extra wei charge. See
  [gas and fees](https://docs.arbitrum.io/how-arbitrum-works/deep-dives/gas-and-fees) and
  [estimation](https://docs.arbitrum.io/arbitrum-essentials/how-to-estimate-gas).

- **OP Stack:** reserve execution gas plus the L1 data fee and any enabled operator fee. Through atlas, query the
  verified `GasPriceOracle` for `getL1Fee(bytes)` using the serialized unsigned transaction. Where supported, query it
  through atlas for `getL1FeeUpperBound(uint256)` using the transaction's unsigned byte length. These interfaces account
  for signature overhead. Do not add it twice. The latter is a practical size bound at the current oracle prices, not a
  cap on fees at inclusion.

  Query `getOperatorFee(gasLimit)` for upfront budgeting. Reconstruct the included operator charge using the inclusion
  fork's parameters, gas use, and exact client rounding/refund semantics. Before Isthmus, the operator charge is absent.
  Isthmus and Jovian use different scalar formulas. Establish absence from fork/configuration evidence, not a failed
  call or a missing provider field. Receipt total is execution cost plus `l1Fee` plus operator cost, each once.

  See [transaction fees](https://docs.optimism.io/op-stack/transactions/fees),
  [Fjord oracle](https://specs.optimism.io/protocol/fjord/predeploys.html),
  [Isthmus operator accounting](https://specs.optimism.io/protocol/isthmus/exec-engine.html), and
  [Jovian changes](https://specs.optimism.io/protocol/jovian/exec-engine.html).

- **Other models, including ZKsync:** obtain the current chain-native estimator and receipt semantics through atlas and
  official docs. Establish coverage of pubdata, resource overhead, custom debits, and refunds for the actual transaction
  type. Use an ordinary type `0`/`2` transfer only if the chain and browser transport support it with complete fee
  coverage. Do not substitute a deprecated estimator or add custom transaction fields without a supported signing path.

  ZKsync's [fee structure](https://docs.zksync.io/zksync-protocol/era-vm/transactions/fee-model/fee-structure) includes
  pubdata/overhead and refunds. Neither 21000 gas nor exact-zero accounting follows from EVM compatibility. Stop with
  the missing component when complete accounting cannot be established.

On chains with fees outside transaction caps, label the total as an **estimated reserve**. For those chains, disclose
the uncovered price movement. Immediately before signing on those chains, recheck affordability. A margin is not a
protocol-enforced maximum. Require `value + upfront fee reserve <= balance`. Ordinary gas estimation or a successful
`eth_call` alone does not prove this.

If final wallet fee edits are allowed, recompute dependent additional fees and affordability from those values. Sweeps
whose value depends on the reserve must preserve all reviewed fields under the fixed-fee exception below.

The final charged fee and sender balance normally already reflect refunded or unspent gas. Do not subtract it again.
Treat asynchronous refunds, such as Arbitrum retryable tickets, separately. Never spend an expected credit before it
arrives. Never claim a snapshot balance is permanent.

Discovery of unrelated pending refunds is not implicit in a plain transfer. A consuming sweep must state its
residual-balance policy and any known pending credits in the review.

### Simulate

Simulate the exact prepared call, preserving sender, target, value, calldata, nonce, transaction type, gas limit, and
fee fields. Then estimate gas. Delegate bounded `eth_call` and `eth_estimateGas` evidence to `evm-atlas`. When an RPC
error contradicts the supplied gas or checkpointed balance, have atlas diagnose the exact simulation path. Do this
before attributing the error to transaction invalidity or chain-wide type support.

Changing fields to make a diagnostic call pass does not validate the prepared transaction. Preserve the consuming
workflow's simulation requirements and user-imposed approval gates. Use a local fork, project simulation, or Cast trace
only when the simulation requires a continuous provider, following Resolve Chain and Provider. A successful simulation
provides evidence. It does not authorize signing.

When exact EIP-7702 simulation needs a signed authorization, use the reference's approved authorization-signing stage
first. Transaction signing and broadcast still follow simulation and transaction approval.

### Review

Before a transaction signature or broadcast, present one concrete review containing:

- chain name and ID, RPC source, and latest block used,
- sender, target, function, decoded arguments, calldata, and native value,
- nonce, gas estimate/limit, fee assumptions, expected total cost, upfront reserve, and any protocol-enforced fee caps,
- chain-specific fee components, inclusion-price uncertainty outside those caps, and expected refunds or residuals,
- the selected gas policy and source,
- for Rabby Slow, the oracle URL, tier, quote time, estimated inclusion time, max fee per gas, and max priority fee per
  gas,
- for a legacy policy, the fixed gas price and transaction type,
- expected approvals, transfers, or other state changes,
- simulation command and outcome,
- selected signer and the exact signing/broadcast command with secrets redacted.

For an authorized contract call, lead with `### ⏳ Transaction review`. For that call, continue to signing and broadcast
in the same turn. Put repeated fields in a compact table. Keep the exact command in a fenced block. For other
transactions or an explicit approval gate, use `### ⚠️ Transaction approval required`. For those cases, obtain
confirmation of the concrete review unless existing approval already covers it.

If reviewed fields change outside the browser-wallet exception below, simulate again. Under that condition, present a
revised review. When the revision remains within the user's authorized intent and limits, proceed. Otherwise, request
the missing authorization. EIP-7702 signatures follow the reference's staged review.

For browser signing only, the reviewed gas limit and fees are starting values unless the consuming workflow requires
them to remain fixed. The user may deliberately change the gas limit, gas price, max fee per gas, or max priority fee
per gas in the wallet confirmation UI. Their approval of that final wallet screen authorizes those edited gas settings.
Apply chain-specific accounting to the additional fees and resulting affordability. Do not stop, require a second
approval, or resimulate solely because they differ from the prepared values.

Continue only when the chain, sender, target, calldata, native value, nonce, authorization list (if present), and
decoded intent still match the authorized review. Wallet changes to any of those fields require rejection and a revised
review.

When the agent confirms the wallet request, preserve the reviewed gas settings. The user-edit exception does not
authorize the agent to accept wallet-selected fee changes. Before accepting wallet-selected fee changes, rebuild the
transaction. Simulate those changes. Review them.

When fees determine the transfer value or another reviewed invariant, such as leaving exactly zero native balance, the
browser exception does not apply. Preserve the reviewed transaction type, gas limit, and fee values. If the wallet
changes those fields, reject before signing. Under that condition, recompute the dependent values. Then simulate. Apply
the revised-review authority rule above.

### Sign and Broadcast

Read [references/browser-signing.md](references/browser-signing.md) for browser capability checks and sender handling.
Open a signing request only after review and within the authority established above. Prefer browser, encrypted keystore,
or hardware wallet in that order unless the user or consuming skill restricts the signer. If browser signing is
unavailable, a browser-only workflow must stop. Never substitute another signer in that workflow.

Use an environment-backed private key only when the user explicitly opts in or no safer method is available. Never ask
for a key in chat or print it.

`cast send` signs and broadcasts in one command. Run it after review when execution is authorized. The Cast 1.8.3+
helpers also sign and broadcast:

- `cast erc20-token transfer|approve|mint|burn`,
- `cast erc20-token permit --broadcast`,
- `cast erc4626 deposit|mint|withdraw|redeem`,
- `cast safe propose|sign|execute`.

Apply the same Prepare, Simulate, and Review phases to those helpers. A Safe proposal or confirmation is a signature
artifact that needs its own payload review. Treat any other subcommand whose installed help shows it signs or submits,
such as `cast safe create`, `add-delegate`, or `remove-delegate`, the same way. Signing a message or typed data,
including `cast erc20-token permit` without `--broadcast`, also requires a review of the exact payload, domain, chain
binding, and intended use before approval.

Pass the selected fees explicitly. EIP-1559 uses `--gas-price` and `--priority-gas-price`. A fixed legacy policy uses
`--legacy --gas-price` without `--priority-gas-price`. Under the default Ethereum policy, use the reviewed Rabby Slow
pair.

Before opening the signer, recheck the active chain's gas and additional-fee requirements. Before opening it, recheck
that the reviewed cap or legacy gas price covers the current base fee where applicable. If fees must change before
signing, simulate again. Under that condition, present a revised review. Never silently change the selected policy.

Wallet fee edits follow Review, including its fixed-fee exception. Message and typed-data signatures consume no gas.

After broadcast, capture the transaction hash. Have `evm-atlas` verify the receipt on the reviewed chain. When a receipt
is still pending, `evm-atlas` may use one bounded RouteMesh `newHeads` subscription to wait for the next block before
checking again. Receipt verification remains required. A pending-transaction or log notification alone cannot confirm
the transaction. Reconcile actual fees under the active chain's model, including additional receipt components and
refunds without double counting.

Missing fee evidence means accounting is incomplete. It does not establish zero fees. Report status, block, gas used,
actual total fee or its exact evidence gap, and the explorer link under `### ✅ Transaction confirmed` for a successful
receipt or `### ↩ Transaction reverted` for a mined failure. For an ambiguous outcome, lead with
`### ⛔ Broadcast unresolved — do not retry`. For that outcome, state the evidence still needed.

Do not retry a failed or uncertain broadcast without first checking whether the transaction exists. For browser-wallet
signing, read [references/browser-signing.md](references/browser-signing.md)'s Timing and Recovering sections before
concluding nothing was sent. A killed or timed-out process does not prove non-broadcast. Wallet interaction can outlast
the command. The wallet may broadcast via its own RPC provider.

## Stop Conditions

Stop before signing when the signer, sender, chain, target, or decoded intent is unresolved. Transaction signing also
requires resolved fee accounting, affordability, and simulation. Only the explicitly approved EIP-7702 authorization
stage may precede those transaction checks. Its authorization payload, signer, and intended use must already be
resolved. Review must distinguish enforced caps from an estimated reserve.

Browser approval of edited gas settings authorizes those settings but does not establish coverage of omitted
chain-specific charges.

Stop before retrying when broadcast outcome is ambiguous. Completion requires one of these results:

- a verified read result,
- a local encoding result,
- an approved signature artifact,
- a mined receipt that `evm-atlas` verified and that matches the reviewed transaction apart from user-approved
  browser-wallet gas settings.

Never decorate or truncate addresses, calldata, signatures, hashes, RPC URLs, fee values, commands, or safety wording.
Revocation completion additionally requires the authorization and cleared-code checks in
[references/eip7702.md](references/eip7702.md).
