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

Handle tabular data with exact values, minimal diffs, recipient-scoped data handling, atomic writes, and structural
validation.

## Invariants

1. Keep precision-sensitive amounts as strings and compute with `decimal.Decimal` or DuckDB `DECIMAL(38, 18)`, never
   binary floats.
2. Touch only requested rows, columns, formulas, and formatting. Existing file conventions override house defaults.
3. For newly authored text tables, prefer TSV, UTF-8 without BOM, LF, one trailing newline, lowercase `snake_case`
   headers, ISO dates, `.` decimals, and `-` nulls.
4. Read unknown text tables with BOM-tolerant UTF-8; never write a BOM.
5. Write in place atomically through a sibling temporary file, validate it, then replace the target.
6. Escape external cells beginning with `=`, `+`, or `@`; a bare `-` null is exempt. Formula-prefix cells in trusted
   authored data are observations, not proof of injection.
7. Treat transaction, bank, exchange, and tax data as user-owned. Use unredacted samples in internal agent reports when
   materially useful; use `--redact-samples` for public or third-party disclosures or when the user asks.

## Factual Profiling

Resolve helper paths from this `SKILL.md`. Profile unknown data before choosing a transformation tool:

```sh
uv run "<skill-dir>/scripts/profile.py" <file>
```

The JSON output has `schema_version: 2`. It reports structural facts, header quality, cardinality/statistics when qsv is
available, frequency facts, formula-prefix cells, workbook metadata, and local tool availability. It contains no tool
recommendations and does not infer identifiers from uniqueness. Choose the tool from the requested transformation,
provenance, output format, and preservation requirements.

Use `--external-data` only when the cells came from an external or otherwise untrusted source and will be written to a
formula-capable consumer. With that flag, formula-prefix cells affect `status`; without it, legitimate formulas such as
`=SUM(...)` remain factual observations and do not fail the profile.

## Tool Routing

| Need                                       | Tool                                             |
| ------------------------------------------ | ------------------------------------------------ |
| Fast structural preview/validation         | `uv run scripts/peek.py <file>`                  |
| Factual local quality profile              | `uv run scripts/profile.py <file>`               |
| Counts, stats, frequencies, select, dedupe | `qsv`                                            |
| Joins, pivots, aggregation, conversion     | DuckDB with `all_varchar = true`                 |
| Exact custom transforms                    | `uv run` Python, stdlib `csv`, `decimal.Decimal` |
| Any `.xlsx`/`.xlsm` input or output        | Read `references/xlsx.md` first                  |
| Exact transformation/validation recipes    | Read `references/recipes.md` only when needed    |

Prefer `qsv --cache-threshold 0` where supported. When qsv stdout must remain TSV, use `-o out.tsv`; stdout otherwise
defaults to CSV.

## Workflow

1. Inspect with `peek.py`; add `profile.py` when cardinality, formula prefixes, metadata, or available tooling matters.
   For a no-shape-change edit, save the peek JSON. For intentional row/schema changes, record the expected width and
   invariants.
2. Decide whether formula-prefix cells are dangerous from provenance and output context. Decide the smallest tool that
   preserves values and formatting. Avoid pandas unless necessary; if used, load every column as strings.
3. Apply the transformation atomically. For idempotent appends with legitimate duplicate rows, use multiset difference
   rather than set deduplication.
4. Validate:
   - unchanged shape: `peek.py --strict --expect-like <before-report>`;
   - changed shape: `peek.py --strict --expect-columns <n>` plus task-specific counts/keys;
   - authored house TSV: add `--house`;
   - formulas: `uv run scripts/recalc.py <file.xlsx>` and require success.
5. Report paths, row/column effects, validation, and any workbook features that could not be preserved.

For human output, lead with `### 📊 Spreadsheet — ✅ updated` only after the write and required validation pass, or
`### 📊 Spreadsheet — 🔎 inspected, no files written` for read-only work. On required validation failure, use
`### 📊 Spreadsheet — ⛔ not deliverable`. Include profile JSON only when it materially supports the report, and keep
JSON, cells, headers, formulas, paths, commands, and diagnostics undecorated.

## prb-finance

Never hand-edit generated `.pool.tsv`, `.annual.tsv`, or Markdown reports. After source edits, run `just tsv-check`,
then `just cli::write-changed`. Cap financial output to counts and file references unless raw rows materially support
the task or were requested. Perform an external-disclosure review before sending financial data outside the agent
workspace.

Completion requires the requested artifact, an intentional diff, atomic replacement where applicable, and structural
plus domain validation evidence.
