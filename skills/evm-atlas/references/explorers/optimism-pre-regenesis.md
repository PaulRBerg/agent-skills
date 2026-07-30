# OP Mainnet Pre-Regenesis History

## Evidence Boundary

Use this reference for OP Mainnet (`chain_id=10`) queries that target activity before the final regenesis on
`2021-11-11`.

Current OP Mainnet JSON-RPC cannot return a canonical `eth_getTransactionReceipt` response for an OVM1 transaction
removed by the final regenesis. A current-RPC `null`, provider archive failure, or explorer omission proves only native
receipt unavailability; it is not negative evidence that the historical transaction occurred.

Dune's `optimism_legacy_ovm1` transaction, log, and trace rows are historical execution evidence. When the required
components agree, reconstruct a **legacy execution packet**. Never call that packet a receipt: it is neither an
authenticated JSON-RPC receipt nor a historical state-trie proof. Preserve its Dune provenance and every missing
component.

## Exact-Transaction Workflow

1. Establish that the target is on OP Mainnet and predates the final regenesis. Use supplied context, another dated
   source, or the exact Dune transaction row. Do not infer nonexistence when current OP Mainnet routes cannot resolve
   the hash.
2. Query all three legacy tables by the exact hash. Replace `<TX_HASH_HEX>` with the 64 hexadecimal digits while
   retaining DuneSQL's `0x` varbinary prefix:

   ```sql
   SELECT *
   FROM optimism_legacy_ovm1.transactions
   WHERE hash = 0x<TX_HASH_HEX>;

   SELECT *
   FROM optimism_legacy_ovm1.logs
   WHERE tx_hash = 0x<TX_HASH_HEX>
   ORDER BY "index";

   SELECT *
   FROM optimism_legacy_ovm1.traces
   WHERE tx_hash = 0x<TX_HASH_HEX>
   ORDER BY trace_address;
   ```

3. Require exactly one transaction row. Preserve zero rows or duplicate rows as a coverage outcome; do not choose a row
   heuristically.
4. Verify the transaction hash, block number, block hash, and block time agree across every returned component. Verify
   transaction `from`/`to` against each log's `tx_from`/`tx_to`; verify transaction `success` against each trace's
   `tx_success`. A trace's own `from`, `to`, and `success` describe that call, so reconcile them through the call
   hierarchy rather than forcing them to equal the root transaction fields.
5. Preserve log indices exactly in ascending order. Preserve trace-address arrays in ascending hierarchy order,
   including the root when present. Record duplicate, missing, or contradictory ordering metadata as a coverage gap;
   never renumber rows.
6. Use the address dashboard only to discover candidate hashes for an address-wide request. An exact-hash request uses
   the table workflow above.

## Legacy Execution Packet

Return exact values; do not truncate identifiers or normalize away nulls.

| Component   | Dune provenance                                | Required packet fields                                                                                                   |
| ----------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Transaction | `optimism_legacy_ovm1.transactions`            | `success`, `from`, `to`, `value`, `data`, gas fields, `block_number`, `block_hash`, `block_time`, transaction index/hash |
| Logs        | `optimism_legacy_ovm1.logs`, ordered by index  | `contract_address`, topics, `data`, `index`, transaction index/hash, block fields, `tx_from`, `tx_to`                    |
| Traces      | `optimism_legacy_ovm1.traces`, ordered by path | call `from`/`to`, `type`, `call_type`, `input`, `output`, `value`, `success`, `error`, `tx_success`, `trace_address`     |

Attach the namespace and table name to each component, plus an explicit `coverage_gaps` list. A packet is complete only
for the requested determination; complete execution rows do not independently prove historical contract state,
deployment identity, wallet ownership, or an LP principal/fee split.

## Coverage Outcomes

Record native-receipt availability separately from packet coverage:

| Outcome                              | Record when                                                                                               |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| Native receipt unavailable           | Current RPC returns `null`, an archive route fails, or the current explorer omits the transaction.        |
| Historical execution evidence absent | The exact transaction row is absent. This is absence from Dune, not proof the transaction never existed.  |
| Legacy execution packet partial      | The transaction exists but zero log rows return and non-emission is not otherwise proven.                 |
| Legacy execution packet partial      | Transaction and logs exist but traces are absent, contradictory, or structurally incomplete.              |
| Legacy execution packet partial      | Optimism's known January-July 2021 execution-effects loss may cover a required component.                 |
| Execution packet complete, state gap | Execution is reconstructable, but required historical contract, wallet, or position state is unavailable. |
| Complete enough for determination    | Transaction, ordering, logs, traces, identity, state, and wallet deltas needed by the request are proven. |

Zero logs can be a legitimate execution result. Promote that outcome beyond partial only when verified calldata, traces,
and contract semantics show that no relevant event should exist. Likewise, trace rows are not complete merely because at
least one row returned; reconcile their hierarchy, transaction status, errors, and required internal value movement.

## Concentrated-Liquidity Reconstruction

When the accounting result depends on a concentrated-liquidity position:

1. Identify the position token and prove the exact position-manager deployment used historically. Do not substitute a
   current explorer label or a same-interface deployment.
2. Retrieve the full prior lifecycle for that position, from creation through the target transaction. Include every
   position transfer, liquidity increase/decrease, collection, and other state-changing action supported by the exact
   deployment's events and calls.
3. Reconstruct the last pre-transaction liquidity state and whether principal or fees were already owed. Never infer
   historical state from a current position lookup. Treat an unproven zero balance as unknown, not zero.
4. Decode the target's exact command order and maximum collect bounds. Bounds and calldata express intent; ordered logs,
   traces, and wallet movements establish execution.
5. Reconcile decrease, collect, unwrap, sweep, refund, and final transfer evidence using exact integer amounts. Keep
   native and wrapped legs distinct.
6. Verify a principal/fee split only when the complete lifecycle and target execution support the residual. Otherwise
   report the combined collection and name the missing pre-state or lifecycle evidence.

## Browser and Persistence Boundary

Chrome DevTools may run bounded, read-only Dune queries. Executing a query may consume Dune credits. Leave queries
unsaved by default; saving, publishing, scheduling, or otherwise externally persisting one requires explicit authority.
Never put a private hash, wallet, position identifier, or financial value into this installable reference.

## Deeper Fallback

For the January-July 2021 period, Optimism documents partial loss of transaction execution effects, including emitted
events and success state. When Dune genuinely lacks the exact transaction or a required lifecycle event, the raw inputs
published through Ethereum's `CanonicalTransactionChain` are a deeper reconstruction path. Re-execution is laborious,
costly, and may remain incomplete. Do not require it when the Dune packet already answers the requested question.

## Sources

- Optimism, "Accessing pre-regenesis history":
  `https://docs.optimism.io/op-mainnet/pre-bedrock-history/regenesis-history`
- Optimism, "Lost pre-regenesis data": `https://docs.optimism.io/op-mainnet/pre-bedrock-history/lost-pre-regenesis-data`
- Dune catalog: `https://dune.com/data/optimism_legacy_ovm1.transactions`,
  `https://dune.com/data/optimism_legacy_ovm1.logs`, and `https://dune.com/data/optimism_legacy_ovm1.traces`
- DuneSQL varbinary literals: `https://docs.dune.com/query-engine/datatypes#varbinary`
- Dune query execution and credit behavior: `https://docs.dune.com/query-engine/query-executions`
- OVM1 address dashboard: `https://dune.com/optimismfnd/OVM1.0-User-Address-Transactions`
