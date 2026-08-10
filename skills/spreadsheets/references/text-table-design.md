# Text Table Design

Design CSV and TSV files as durable interfaces for human maintenance and machine consumption. Existing project and
source contracts override house defaults.

## Vocabulary

Use these terms consistently:

- **Table interface** — everything a reader or writer must know: artifact class, serialization, header and column order,
  row grain, key, value grammar, ordering, provenance, and evolution expectations.
- **Artifact class** — whether the table is source evidence, a canonical authored table, or a generated output.
- **Row grain** — the single fact or entity represented by one row.
- **Key** — the stable column tuple that identifies a row, including whether duplicate keys or duplicate rows are
  legitimate.
- **Value grammar** — the allowed representation of each column: identifiers, enums, units, time zones, precision,
  dates, nulls, and other sentinel values.
- **Ordering contract** — whether row and column order is semantic, deterministic for review, or unconstrained.
- **Evolution contract** — how a schema change reaches producers, consumers, existing rows, generated artifacts, and
  validators.

## Design Workflow

### 1. Classify the artifact

- **Source evidence:** follow the owning project's preservation contract. Preserve source meaning, uncertainty, and any
  required schema or row ordering; never apply house defaults silently.
- **Canonical authored table:** optimize for clear maintenance and deterministic machine use. Apply the entrypoint's
  authored text-table invariants unless the project defines a stronger convention.
- **Generated output:** find and edit the owning source or generator. Treat the emitted table as a review surface, not
  an independent schema.

### 2. Define row identity

State the row grain in one sentence before choosing columns. Give each row one coherent meaning; do not hide repeated
records, nested structures, or unrelated facts in a delimited cell unless the source contract requires it.

Define the key explicitly. Use a natural key when its components are stable and unambiguous; otherwise use an owned
stable identifier. Record whether duplicate keys or identical rows are valid. Never infer a key solely from uniqueness
in a sample.

### 3. Define columns and value grammar

- Give each column one meaning. Split overloaded fields when their parts are independently queried, validated, or
  changed.
- Make units and time zones explicit in the header or owning schema, and use one convention consistently.
- Distinguish null, unknown, not applicable, zero, and empty text. Do not alternate between blank and `-` for the same
  meaning.
- Define enum spelling and case, identifier canonicalization, decimal precision, and date/time representation.
- Order columns for the dominant reading and editing flow. Keep related fields adjacent and prefer stable semantic order
  over alphabetical order.
- Keep provenance sufficient to trace authored or reconstructed facts without embedding an entire narrative in one cell.

Normalize entities that change independently or whose repetition creates inconsistent copies. Denormalize when
self-contained rows materially improve auditability or maintenance and the duplicated facts are stable or validated. Do
not maintain two columns that compete as the source of truth for the same fact.

### 4. Choose serialization

Follow an existing table's established contract. For a newly authored text table, use the entrypoint's house defaults;
choose CSV instead of TSV only for source fidelity or consumer interoperability. Use a real CSV/TSV parser and writer
whenever delimiters, quotes, or newlines can occur in cells.

### 5. Design validation before writing

Identify the owning schema or validator and the task-specific semantic invariants. Structural validation must cover
encoding, delimiter, header, width, and row count expectations. Semantic validation must cover keys, required values,
enums, ordering, cross-column relationships, and any intentional duplicates.

## Evolving a Table Interface

For every added, removed, renamed, reordered, split, or merged column, or any change to row grain, key, value grammar,
or ordering:

1. Inventory the schema source, producers, consumers, existing tables, generated artifacts, and validators.
2. Update the owning schema or producer rather than patching generated outputs.
3. Migrate every in-scope existing row atomically, preserving values not changed by the migration.
4. Regenerate derived artifacts through their owner.
5. Validate the exact header and order, row counts, keys, and task-specific semantic invariants.
6. Decide compatibility explicitly when an active consumer requires it; do not add speculative aliases or shims.

A schema that parses while existing rows, producers, or consumers remain stale is not a completed migration.

## Review Checklist

- The artifact class and row grain are explicit.
- The key and duplicate policy are explicit.
- Every column has one meaning and a defined value grammar.
- Units, time zones, nulls, enums, and identifiers are unambiguous.
- Row and column ordering are intentional.
- Provenance and generated/source ownership are clear.
- Validation proves both structure and domain semantics.
- Schema evolution covers existing data and every active producer and consumer.
