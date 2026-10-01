# Chain categories and exact-zero native sweeps

`references/generated/target-mainnets.json` is the source of truth for every target's current `category`. Categories
describe current architecture, not historical protocol behavior. Do not duplicate a target-chain roster here: resolve
the row first, then report its category with the chain evidence.

| Category | Key        | Scope and exact-zero implication                                                                                                                                                                                                                                                                                                                   |
| -------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Mainnet  | `mainnet`  | Ethereum only. A fixed-price plain native transfer is eligible when its current inclusion and standard-transfer proof holds.                                                                                                                                                                                                                       |
| Alt L1   | `alt-l1`   | Ethereum-style L1s, deliberately including Polygon PoS. They can be eligible only with target-specific proof of fixed gas, a fixed full `gasPrice` debit, and no extra debits or credits; BNB Chain and Polygon PoS are examples.                                                                                                                  |
| L2 OP    | `op-stack` | Ineligible for a guaranteed direct exact-zero transfer: automatic uncapped L1 data fees and operator fees can add to execution gas.                                                                                                                                                                                                                |
| Nitro    | `nitro`    | Ineligible for a guaranteed direct exact-zero transfer: parent-chain posting economics and CollectTips-dependent charged pricing prevent the required fixed total debit proof.                                                                                                                                                                     |
| ZK       | `zk`       | Includes ZK Stack, validium, and hybrid targets. Linea and Taiko can be eligible when a standard fixed legacy debit is proven at inclusion. Scroll and Morph are ineligible because of separate dynamic L1 fees. ZK Stack targets, including ZKsync Era, Abstract, and Sophon, are ineligible because pubdata, overhead, and refunds are variable. |
| Alt L2   | `alt-l2`   | Fallback category. Treat as unknown: require bespoke target proof of the complete debit, or block. Lightlink can be eligible when a standard fixed legacy debit is proven at inclusion.                                                                                                                                                            |

Category is only the first gate. An eligible category still requires current target-specific evidence that the
transaction is an ordinary empty-calldata transfer, uses a fixed full charge at inclusion, has no additional debit or
credit, and has a fixed gas use. Type-0 acceptance, a fixed bid shape, or successful simulation alone does not prove
eligibility.

For Ethereum-style accounting, an included legacy transaction pays its signed `gasPrice`: changes to base fees or
admission quotes can prevent inclusion, but do not reprice the signed transaction. Linea's L1-cost-based profitability
check is an admission rule, not an additional sender debit; Taiko also charges the signed legacy price without a
separate L1 fee. With proven `21000` gas, ordinary EOAs, empty calldata, and unchanged balance/nonce/code, both permit
`value = balance - 21000 * gasPrice`. This is conditional on inclusion, not a guarantee that a public transaction will
be included.

OP Stack means OP Stack-based execution, including forks: its L1 data fee and any enabled operator fee are added to
execution cost. Nitro means the Arbitrum family, including AnyTrust chains: its parent-chain posting charge is converted
into child-chain gas, not added again as a separate wei fee. Fixing the legacy bid does not fix either family's total
charge. ZK covers distinct execution and proving systems, so use the named chain's fee branch rather than inferring a
shared fee model from its proof technology.

Scroll charges its L1 data fee on the full signed RLP bytes, and under Feynman that fee scales with byte length. Its
`L1GasPriceOracle` predeploy (`0x5300000000000000000000000000000000000002`) prices exactly the bytes passed to
`getL1Fee(bytes)` and, unlike OP Stack's `GasPriceOracle`, adds no signature overhead. A quote on an unsigned
empty-calldata EIP-1559 transfer (about 46 bytes) is therefore less than half the charge on the signed transaction
(about 114 bytes). Quote a maximum-size signed stand-in instead: the final fields serialized with nonzero 32-byte
`r`/`s` placeholders. Verified 2026-09-29: the oracle quote on real signed bytes at the parent block equals the receipt
`l1Fee`.

Linea (`59144`) is eligible as a ZK target. Blocks carry `baseFeePerGas = 7`; legacy transactions are debited exactly
`gasUsed * gasPrice` with no L1 or other extra debit; receipts report `effectiveGasPrice` equal to the signed
`gasPrice`; and an empty-calldata EOA transfer estimates and uses `21000` gas. `linea_estimateGas` returns the
sequencer's profitability floor as `baseFeePerGas + priorityFeePerGas`, while `eth_gasPrice` and
`eth_maxPriorityFeePerGas` sit above it, rose about 50% within two minutes, and differed between reads at one head;
price from the highest suggestion observed at the checkpoint. Verified 2026-10-01: a third-party empty-calldata legacy
transfer in block `32202314` reconciled to `value + 21000 * gasPrice` across its block, and an exact-zero sweep at
`21000` gas left a `0` balance.

Lightlink (`1890`) is an Alt L2 exception. It runs a Geth 1.10 fork without London: blocks carry no `baseFeePerGas`,
`eth_maxPriorityFeePerGas` and `eth_feeHistory` are unsupported, and receipts omit `effectiveGasPrice`, so use legacy
pricing only. No separate L1 data, operator, or rollup fee is debited from the sender. Enterprise Mode gasless
transactions are signed with `gasPrice = 0`; the node does not reprice a signed nonzero price. Verified 2026-09-30:
consecutive-block sender balance deltas equalled `gasUsed * gasPrice` exactly for ordinary legacy transactions, and an
empty-calldata EOA transfer estimated `21000`. Because receipts lack `effectiveGasPrice`, verify the charged price as
the signed `gasPrice` plus an exact receipt-block balance reconciliation.

IoTeX (`4689`) is an Alt L1 exception. Legacy transactions are debited exactly `gasUsed * gasPrice` with no refund or
extra debit. Receipts, including an empty-calldata transfer's on RouteMesh and the public RPC (verified 2026-09-30),
report `effectiveGasPrice` equal to the signed `gasPrice`; if one omits it, fall back to exact receipt-block balance
reconciliation. An empty-calldata EOA transfer charges `10000` gas, not `21000`, and `eth_estimateGas` still returns
`21000`; a `21000` gas limit therefore leaves `11000 * gasPrice` behind. For an exact drain, use a `10000` gas limit,
which `eth_call` and the node accept, instead of the estimate. Verified 2026-09-30: two empty-calldata transfers used
exactly `10000` gas, and sender balance deltas reconciled to `value + gasUsed * gasPrice`; an exact-zero sweep at that
limit left a `0` balance.

Polygon PoS (`137`) is eligible as an ordinary Alt L1 target. Legacy transactions are debited exactly
`gasUsed * gasPrice` with no refund or extra debit; receipts report `effectiveGasPrice` equal to the signed `gasPrice`,
and an empty-calldata EOA transfer estimates and uses `21000` gas. Plain transfer receipts carry Bor system logs, which
are not sender debits. Take the required tip from `eth_maxPriorityFeePerGas` (observed 25-30 gwei). Verified 2026-10-01:
a third-party empty-calldata legacy transfer reconciled to `value + 21000 * gasPrice` across its block, and an
exact-zero sweep at `21000` gas left a `0` balance.

BNB Chain (`56`) is eligible as an ordinary Alt L1 target. Blocks carry `baseFeePerGas = 0`; legacy transactions are
debited exactly `gasUsed * gasPrice` with no refund or extra debit; receipts report `effectiveGasPrice` equal to the
signed `gasPrice`; and an empty-calldata EOA transfer estimates and uses `21000` gas. Take the required price from
`eth_gasPrice` and `eth_maxPriorityFeePerGas` (both observed at 0.05 gwei). Verified 2026-10-01: undelegated third-party
empty-calldata legacy transfers in block `125118813` reconciled to `value + 21000 * gasPrice` across their block, and an
exact-zero sweep at `21000` gas left a `0` balance.

Gnosis (`100`) is eligible as an ordinary Alt L1 target. Legacy transactions are debited exactly `gasUsed * gasPrice`
with no refund or extra debit; receipts report `effectiveGasPrice` equal to the signed `gasPrice`, and an empty-calldata
EOA transfer estimates and uses `21000` gas. Base fees are only tens of wei and volatile: a 1024-block `eth_feeHistory`
window spanned 8-53 wei, and the base fee doubled within minutes, so a price derived from one block can fall below the
inclusion base fee and wait in the mempool. Verified 2026-10-01: a third-party empty-calldata legacy transfer at 14 wei
in block `48535334` reconciled to `value + 21000 * gasPrice` across its block, and an exact-zero sweep at `21000` gas
and 16 wei left a `0` balance after waiting for the base fee to fall back.

Filecoin FEVM is an Alt L1 exception. FVM fee translation and overestimation require bespoke evidence; do not generalize
Ethereum-style L1 fee behavior to it. A fresh EOA recipient does not alter the standard top-level transfer gas cost: the
`25000` new-account `CALL` cost concerns the contract opcode, not a top-level transfer. Still exclude precompiles and
protocol system destinations; `eth_getCode == 0x` alone does not establish that an address is an ordinary recipient.

For a category-fee or exact-drain question, return the current category, target row, protocol/source evidence, and the
decision before any signer setup. Atlas remains read-only; return evidence for `$cli-cast` and the calling workflow to
construct, simulate, and account for a transaction.

## Sources

- [EIP-1559](https://eips.ethereum.org/EIPS/eip-1559)
- [OP Stack transaction fees](https://docs.optimism.io/op-stack/transactions/fees)
- [Arbitrum gas and fees](https://docs.arbitrum.io/how-arbitrum-works/deep-dives/gas-and-fees) and
  [Nitro L1 pricing](https://github.com/OffchainLabs/nitro/blob/master/arbos/l1pricing/l1pricing.go)
- [ZKsync Era fee structure](https://docs.zksync.io/zksync-protocol/era-vm/transactions/fee-model/fee-structure) and
  [bootloader](https://docs.zksync.io/zksync-protocol/era-vm/contracts/bootloader)
- [Scroll transaction fees](https://docs.scroll.io/en/developers/transaction-fees-on-scroll/)
- [Morph state transition](https://github.com/morph-l2/go-ethereum/blob/main/core/state_transition.go) and
  [rollup fee](https://github.com/morph-l2/go-ethereum/blob/main/rollup/fees/rollup_fee.go)
- [Linea gas fees](https://github.com/Consensys/doc.linea/blob/main/docs/network/how-to/gas-fees.mdx) and
  [sequencer profitability validation](https://github.com/Consensys/linea-monorepo/blob/main/linea-besu/plugins/linea-sequencer/sequencer/src/main/java/lineth/sequencer/txpoolvalidation/validators/ProfitabilityValidator.java)
- [Taiko legacy transaction](https://github.com/taikoxyz/taiko-geth/blob/taiko/core/types/tx_legacy.go) and
  [state transition](https://github.com/taikoxyz/taiko-geth/blob/taiko/core/state_transition.go)
- [Filecoin FIP-0091](https://github.com/filecoin-project/FIPs/blob/master/FIPS/fip-0091.md),
  [FEVM gas differences](https://docs.filecoin.io/smart-contracts/filecoin-evm-runtime/difference-with-ethereum), and
  [Filecoin gas estimation](https://docs.filecoin.io/reference/exchanges/exchange-integration#automatic-gas-values)
- [LightLink Enterprise Mode](https://docs.lightlink.io/lightlink-protocol/building-on-lightlink/enterprise-mode-overview)
- [Geth top-level transaction gas](https://github.com/ethereum/go-ethereum/blob/master/core/state_transition.go)
