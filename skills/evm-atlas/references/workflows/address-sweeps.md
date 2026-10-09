# Address Sweeps

Use for address-wide historical activity and bootstrap discovery on target mainnets. The deterministic request/evidence
rules live in `scripts/sweep-core.py`. This reference defines safety, agent decisions, and coverage interpretation.

## Safety Model

- Accept only a 20-byte public EVM address. Never read or expose keys, mnemonics, or credentials.
- Query only rows in `references/generated/target-mainnets.json`. A `cross-vm` result covers that chain's EVM execution
  environment only.
- Fix one ISO-8601 UTC cutoff and one finalized or independently verified block per target chain. Record requested
  cutoff, resolution kind, chain ID, block number/hash/timestamp, and observation time. The checkpoint block must be at
  or before the cutoff. Reuse its exact hash and height for state and indexed history.
- Never turn `latest`, an explorer page count, a provider error, an unbound zero counter, or a lagging index into a
  negative result.
- Keep credentials out of plan/response artifacts. Supply them only to the agent-selected read-only provider command.

If a checkpoint cannot be established, or required indexed channels cannot cover it, the result is unknown/partial.

## Profiles

Name the profile before planning:

- `general`: normal transactions, internal transactions, ERC-20, ERC-721, and ERC-1155 transfers, plus checkpointed
  nonce/native balance facts. This is the default historical-activity profile.
- `bootstrap-discovery`: a narrower activity-discovery profile. It includes checkpointed native state, qualifying
  normal/internal native activity, ERC-20, and ERC-721. ERC-1155 and nonqualifying native noise are outside it.

The helper owns the exact success/noise predicates. Do not silently apply bootstrap exclusions to a general sweep. A
`bootstrap-discovery` negative means only “no activity qualifying under the `bootstrap-discovery` profile was found.”

For the `bootstrap-discovery` profile, the helper may omit normal/internal history only when checkpointed nonce and
balance are both zero and `accountActivityModel` is exactly `ethereum-eoa`. It default-denies this shortcut for missing,
unrecognized, native-account-abstraction, cross-VM, or unknown models. Token history remains required.

## Agent Decisions Before Planning

The agent owns:

- target-chain selection and whether to stop after the first positive
- cutoff acquisition and checkpoint proof
- provider selection and fallback activation using `provider-routing.md`
- declared provider capabilities and independence groups
- optional quorum requirement
- final observed-fact versus inference wording.

On declared overlaps, prefer Blockscout. Where Etherscan is primary or after a concrete fallback trigger, use Etherscan.
A valid empty authoritative response does not trigger fallback. Run the existing plan-detection helpers before using
paid Etherscan or Blockscout PRO capabilities. Quorum counts independent indexers, not hosts or state RPCs. Provider
disagreement is unknown, never a majority decision.

For Etherscan, apply dated chain-access notes and endpoint gates from `references/explorers/etherscan-api.md`. With a
Free or unknown plan, request at most 1,000 records per page. Under that plan condition, exhaust pagination at the fixed
cutoff. Use CLI 1.1.1 with an explicit numeric `--chain` and `--output json`. Prefer manual `--page` and `--offset`
requests with fixed bounds and sorting. CLI `--all` can stop at its page bound with exit 0. It does not prove complete
coverage. Retain exit codes and credential-safe stderr.

Shared community quota exhaustion leaves coverage incomplete. Under that exhaustion, retain the reset time. Under that
exhaustion, activate the indexed fallback or wait until reset. Address-filtered internal history remains available
without PRO. Block-range-only internal history requires Standard or higher.

## Plan Interface

Create an agent-selected input JSON with:

- `address`
- `chain`: at least `id`, display name, and `accountActivityModel`
- `goal`: `historical-activity` or `bootstrap-discovery`
- named `profile`
- `checkpoint`: `requestedAt`, `resolutionKind`, `blockNumber`, `blockHash`, `blockTimestamp`, `observedAt`
- `providers`: stable ID, provider kind, independent-index group, and supported history-channel names
- `quorum`: positive integer, default `1`.

Then run:

```sh
uv run scripts/sweep-core.py plan --input <spec.json> > <plan.json>
```

The credential-free plan has `schemaVersion: 1`, normalized required channels, checkpoint-bound JSON-RPC state requests,
bounded indexed-history request specifications, and quorum ordering requirements. It does not acquire a checkpoint,
choose a provider, or construct credential-bearing URLs.

Execute only the emitted requests through the selected provider's documented adapter. Save responses in this shape:

```json
{
  "state": {
    "nonce": { "ok": true, "result": "0x0", "blockHash": "<checkpoint-hash>" },
    "native-balance": { "ok": true, "result": "0x0", "blockHash": "<checkpoint-hash>" }
  },
  "providers": {
    "<provider-id>": {
      "indexedThrough": 123,
      "channels": {
        "txlist": { "ok": true, "complete": true, "rows": [] }
      }
    }
  }
}
```

`complete: true` means bounded pagination for that channel is exhausted through the checkpoint. Preserve native provider
row fields. Do not preclassify them.

For the Etherscan CLI adapter, map each emitted account action to
`etherscan --chain CHAIN_ID --output json account ACTION`. Use `--address ADDRESS` for `txlistinternal` and a positional
address for the other history actions. Map `startblock`, `endblock`, `page`, `offset`, and `sort` to their matching
flags. Keep the emitted values fixed except the advancing page. CLI history stdout is the row array directly, not
`.result`. A validated manual `[]` can exhaust the bounded channel. Blank stdout, nonzero exit, malformed rows, or
truncated pagination leave a gap.

Checkpointed state still follows `provider-routing.md`. Never claim an API-envelope validator passed on unwrapped CLI
output. When conformance requires the original envelope, record that direct API exception and retain the raw response.

## Evaluate Interface

```sh
uv run scripts/sweep-core.py evaluate \
  --plan <plan.json> --responses <responses.json> > <result.json>
```

The evaluator validates checkpoint binding, quantities, index bounds, response shapes, required channels, profile
predicates, EOA zero-state eligibility, earliest qualifying evidence, and quorum agreement. It returns
`positive|negative|unknown`, `checked`, `omitted`, explicit `gaps`, coverage, earliest evidence, and provider/quorum
facts. When malformed external responses can be isolated, they become coverage gaps. Malformed plan input fails.

For offline conformance checks, use:

```sh
uv run scripts/sweep-core.py validate blockscout-address-counters < response.json
uv run scripts/sweep-core.py validate etherscan-transfer-topics --address <address> < response.json
```

The existing shell validator commands remain output-compatible adapters.

## Coverage and Reporting

A positive needs qualifying checkpoint-bounded evidence. It may still have partial channel coverage. A negative is valid
only when every profile channel is checkpoint-bound and complete or appears in `omitted` under the named safe invariant.
Missing channels, lag, malformed responses, provider disagreement, and unsupported actions make the result
unknown/partial.

For yes/no requests, lead with the result and first qualifying chain/action. For reports, include chain, checkpoint,
profile, activity result, checked/omitted channels, coverage, provider, fallback, and quorum. Express checkpoints as at
least `chain_id:block_number:block_hash` plus cutoff time.

Do not claim inactivity on all EVM chains unless the exact target scope and general profile are complete. For
`cross-vm`, append “in the EVM execution environment.” Keep observed provider facts separate from inference.

## Current Holdings

For current native/token holdings, use `debank-portfolio.md` first (`blockscan-balances.md` for a named chain or DeBank
gaps) and `provider-routing.md` for NFTs and API gaps. Pin native state to a finalized/verified checkpoint. Treat
holdings endpoints observed at a provider head as separately timed evidence. Never infer token/NFT emptiness from native
RPC alone. Indexed token amounts can be stale. When an amount decides an action, confirm it with RPC `balanceOf`, as
`address-usd-value.md` does.
