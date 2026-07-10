---
argument-hint: "[file]"
disable-model-invocation: false
name: spreadsheets
user-invocable: true
description:
  "Use when CSV, TSV, or Excel (.xlsx) is the primary input/output: inspect, transform, validate, convert, recalc
  formulas, or create/fix spreadsheets. Do not trigger when tabular data is incidental."
---

# Spreadsheets

Handle tabular data with exact values, minimal diffs, local privacy, atomic writes, and structural validation.

## Invariants

1. Keep precision-sensitive amounts as strings and compute with `decimal.Decimal` or DuckDB `DECIMAL(38, 18)`, never
   binary floats.
2. Touch only requested rows, columns, formulas, and formatting. Existing file conventions override house defaults.
3. For newly authored text tables, prefer TSV, UTF-8 without BOM, LF, one trailing newline, lowercase `snake_case`
   headers, ISO dates, `.` decimals, and `-` nulls.
4. Read unknown text tables with BOM-tolerant UTF-8; never write a BOM.
5. Write in place atomically through a sibling temp file, validate it, then replace the target.
6. Escape external cells beginning with `=`, `+`, or `@` to prevent formula injection; a bare `-` null is exempt.
7. Keep transaction, bank, exchange, and tax data local. Redact sample values in reports unless raw rows were explicitly
   requested.

## Tool Routing

Resolve bundled scripts relative to this `SKILL.md`, not the target repository.

| Task                                       | Tool                                          |
| ------------------------------------------ | --------------------------------------------- |
| First look / CSV or TSV structure          | `uv run scripts/peek.py <file>`               |
| Detailed local quality profile             | `uv run scripts/profile.py <file>`            |
| Counts, stats, frequencies, select, dedupe | `qsv`                                         |
| Joins, pivots, aggregation, conversion     | DuckDB with `all_varchar = true`              |
| Exact row transforms                       | `uv run` Python, stdlib `csv`, `Decimal`      |
| Any `.xlsx`/`.xlsm` input or output        | read `references/xlsx.md` first               |
| Exact transform and validation recipes     | read `references/recipes.md` only when needed |

Prefer `qsv --cache-threshold 0` where supported to avoid sidecar caches. When qsv stdout must remain TSV, use
`-o out.tsv`; do not rely on redirection because stdout defaults to CSV.

## Workflow

1. Inspect the file with `peek.py`. For a no-shape-change edit, save its JSON report; for intentional row/schema
   changes, record the expected resulting width and invariants.
2. Choose the smallest tool that preserves values and formatting. Avoid pandas unless necessary; if used, load all
   columns as strings.
3. Apply the transformation atomically. For idempotent appends with legitimate duplicate rows, use multiset difference
   rather than set deduplication.
4. Validate:
   - unchanged shape: `peek.py --strict --expect-like <before-report>`;
   - changed shape: `peek.py --strict --expect-columns <n>` plus task-specific counts/keys;
   - authored house TSV: add `--house`;
   - formulas: recalculate with `uv run scripts/recalc.py <file.xlsx>` and require success.
5. Report paths, row/column effects, validation results, and any formulas or workbook features that could not be
   preserved.

## prb-finance

Never hand-edit generated `.pool.tsv`, `.annual.tsv`, or Markdown reports. After source edits, run `just tsv-check`,
then `just cli::write-changed`. Cap private table output to counts and file references unless raw rows were explicitly
requested.

Completion requires the requested artifact, an intentional diff, atomic replacement where applicable, and structural
plus domain-specific validation evidence.
