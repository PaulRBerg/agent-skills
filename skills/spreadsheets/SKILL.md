---
argument-hint: "[file]"
disable-model-invocation: false
name: spreadsheets
user-invocable: true
description: 'Use when CSV, TSV, or Excel (.xlsx) is the primary input/output: inspect, transform, validate, convert, recalc formulas, or create/fix spreadsheets. Do not trigger when tabular data is incidental.'
---

# Spreadsheets

Opinionated tabular-data handling for macOS. TSV/CSV is the primary format; `.xlsx` is the exception. Stack: `qsv` for fast validation/profiling, `duckdb` for SQL, `uv`-run Python (stdlib `csv` + `Decimal`) for precision transforms, `qsv excel`/DuckDB/fastexcel for values-only Excel reads, XlsxWriter for new workbooks, openpyxl for existing workbook mechanics, and headless LibreOffice for formula recalculation. Run all Python through `uv` — never bare `python` or `pip`.

## Tool Selection

`scripts/peek.py`, `scripts/profile.py`, and `scripts/recalc.py` are bundled with this skill. Resolve them relative to this `SKILL.md`'s directory, not the user's current project.

| Job                                                          | Use                                                 |
| ------------------------------------------------------------ | --------------------------------------------------- |
| First look or structural validation of CSV/TSV               | `uv run scripts/peek.py <file> [--strict]`          |
| Quality profile of CSV/TSV                                   | `uv run scripts/profile.py <file> [--markdown]`     |
| Counts, stats, frequencies, dedupe, column select            | `qsv` (use `--cache-threshold 0` for stats)         |
| Joins, group-bys, pivots, cross-file SQL, format conversion  | `duckdb -c "..."`                                   |
| Row-level transforms, precision-critical edits               | `uv run` Python with a PEP 723 header               |
| Anything `.xlsx` in or out                                   | read [references/xlsx.md](references/xlsx.md) first |
| Recalculating `.xlsx` formulas                               | `uv run scripts/recalc.py <file.xlsx>`              |
| Row/column-aware diff of two tables                          | `bunx daff old.tsv new.tsv`                         |
| Interactive viewing (suggest to the user; never launch TUIs) | `csvlens`, `vd`, Numbers.app                        |

Read [references/recipes.md](references/recipes.md) for common exact-decimal transforms, idempotent appends, schema validation, keyed diffs, and safe workbook output patterns.

## Hard Rules

1. **Decimals, never floats.** Crypto amounts carry up to 18 decimals — beyond float64. Keep amounts as strings end to end; compute with `decimal.Decimal` or DuckDB `DECIMAL(38, 18)`. Read with `all_varchar = true` in DuckDB and plain stdlib `csv` in Python. If pandas is unavoidable, pass `dtype=str`.
2. **Touch only what was asked.** No reordering, re-quoting, renumbering, or whitespace "tidying" outside the requested change. The diff must contain the change and nothing else.
3. **House format for authored files**: TSV; UTF-8 without BOM; LF line endings; single trailing newline; lowercase `snake_case` headers; ISO 8601 dates (`YYYY-MM-DD`; prb-finance timestamps use `YYYY-MM-DD@HH:MM:SS`); `.` decimal point; no thousands separators or currency symbols inside cells; `-` for null. Conventions already present in an existing file override every one of these.
4. **Strip BOMs on read, never write them.** Open files of unknown provenance with `encoding="utf-8-sig"`.
5. **Validate after editing.** Before no-shape-change edits, save a `peek.py` report; after the edit, run `peek.py --strict --expect-like <before-report>`. For intentional row/schema changes, use `--strict --expect-columns <n>` instead. Add `--house` for authored TSVs that should follow this skill's house format. Do not skip this because `peek.py` is absent from the target repo; the script lives next to this `SKILL.md`. In prb-finance: `just tsv-check`, then `just cli::write-changed` to regenerate derived reports — never hand-edit generated `.pool.tsv`/`.annual.tsv`/`.md` artifacts.
6. **In-place edits are atomic.** Write to a temp file next to the target, verify it, then `mv` over the original.
7. **Finance data stays local.** Treat transaction logs and bank/exchange exports as private tax records: never send their contents to web services or external APIs.
8. **Escape spreadsheet formula injection** when writing cells sourced from external data: prefix a leading `=`, `+`, or `@` with `'` (a bare `-` null is exempt).

## Inspect

`peek.py` is at `scripts/peek.py` inside this skill directory. In normal installed-skill use, that means:

- `~/.agents/skills/spreadsheets/scripts/peek.py`

If your current directory is the skill directory, run:

```sh
uv run scripts/peek.py <file> [--rows N] [--strict] [--house] [--expect-like before.peek.json]
```

If your current directory is the target project, run it by absolute path:

```sh
uv run ~/.agents/skills/spreadsheets/scripts/peek.py <file> [--rows N] [--strict] [--house] [--expect-like before.peek.json]
```

The report includes `status`, `issues`, encoding and BOM, newline style and trailing newline, delimiter and how it was detected, header with duplicates flagged, column/row counts, ragged and empty rows, `-` null usage, qsv validation metadata when available, and sample rows. Default inspection exits `0` for parseable delimited files. Validation flags exit `1` when they find issues. Operational errors such as missing files, binary spreadsheets, or empty files exit `2`.

Fast validation loop:

```sh
# Before edits that should preserve shape and format
uv run ~/.agents/skills/spreadsheets/scripts/peek.py txs.tsv > txs.before.peek.json

# After the edit
uv run ~/.agents/skills/spreadsheets/scripts/peek.py txs.tsv --strict --expect-like txs.before.peek.json

# If rows or headers intentionally changed, lock only the resulting width
uv run ~/.agents/skills/spreadsheets/scripts/peek.py txs.tsv --strict --expect-columns 12

# If the file is private and you need to show the report, hide sample values
uv run ~/.agents/skills/spreadsheets/scripts/peek.py txs.tsv --redact-samples
```

`--expect-like` catches drift in column count, header, delimiter, encoding, newline style, trailing newline, and data row count; it also fails on newly introduced ragged, empty, duplicate-header, BOM, or qsv validation problems. `--engine auto` uses a fast parser for simple unquoted delimited files and falls back to stdlib `csv`; use `--engine python` if you need to force the conservative path. On a binary spreadsheet, `peek.py` exits with a pointer to the xlsx workflow.

Full local quality profile:

```sh
uv run ~/.agents/skills/spreadsheets/scripts/profile.py txs.tsv --markdown --redact-samples
```

The profile combines `peek.py`, qsv stats/frequencies without sidecar caches, header safety, formula-injection detection, and next-step recommendations. Use JSON output by default for machine reading; use `--markdown` for a concise human report.

Quick follow-ups with `qsv`:

```sh
qsv count txs.tsv                 # row count (excludes header)
qsv headers txs.tsv               # numbered column names
qsv stats --cache-threshold 0 -E txs.tsv | qsv table  # per-column types/ranges/cardinality without sidecars
qsv frequency -s event txs.tsv    # value distribution of one column
qsv select date_utc,amount txs.tsv
qsv dedup txs.tsv
```

qsv infers the input delimiter from the file extension, but stdout is always comma-separated. When the result must stay TSV, write it with `-o out.tsv` (the output extension sets the delimiter) — never shell redirection. `qsv stats` creates sidecar caches by default when runs are slow; use `--cache-threshold 0` unless the user explicitly wants reusable qsv caches or temporary indexes for very large files.

## Query with DuckDB

Canonical read — everything as strings, `-` mapped to NULL:

```sql
FROM read_csv('txs.tsv', delim = '\t', header = true, all_varchar = true, nullstr = '-');
```

```sql
-- Profile every column
SUMMARIZE SELECT * FROM read_csv('txs.tsv', delim = '\t', all_varchar = true, nullstr = '-');

-- Aggregate with exact decimals
SELECT event, SUM(amount::DECIMAL(38, 18)) AS total
FROM read_csv('txs.tsv', delim = '\t', all_varchar = true, nullstr = '-')
GROUP BY event
ORDER BY total DESC;

-- Write a TSV back out
COPY (SELECT ...) TO 'out.tsv' (FORMAT csv, DELIMITER '\t', HEADER true, NULLSTR '-');
```

DuckDB also reads and writes `.xlsx` (`read_xlsx`, `COPY ... (FORMAT xlsx)`) — see [references/xlsx.md](references/xlsx.md).

## Transform with uv-run Python

Stdlib `csv` keeps every cell a string — precision-safe by default. Script template:

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
import csv
from decimal import Decimal

with open("in.tsv", encoding="utf-8-sig", newline="") as f:
    rows = list(csv.DictReader(f, delimiter="\t"))

# transform here; use Decimal(row["amount"]) for arithmetic

with open("out.tsv", "w", encoding="utf-8", newline="") as f:
    writer = csv.DictWriter(f, fieldnames=rows[0].keys(), delimiter="\t", lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
```

- Pass `newline=""` to every `open()` the `csv` module touches, and `lineterminator="\n"` for LF output.
- Third-party deps go in the PEP 723 block; for one-liners use `uv run --with <pkg> python -c "..."`.
- For idempotent backfills, dedupe by multiset difference — count existing identical rows and append only the surplus, because identical rows can be legitimate (e.g. batch payouts).

## Excel (.xlsx)

Read [references/xlsx.md](references/xlsx.md) whenever a `.xlsx`/`.xlsm` is input or deliverable: openpyxl create/edit, DuckDB xlsx I/O, styling, conversion recipes, and the recalculation loop. The two absolutes:

- Write real formulas (`=SUM(B2:B9)`), not values precomputed in Python.
- After writing any formula, run `uv run scripts/recalc.py <file.xlsx>` and deliver only when it exits `0` with `"status": "success"`. Formula errors and incomplete cached values exit nonzero by default; use `--soft` only when you deliberately want a report without failing automation.

Recalculation needs LibreOffice: `brew install --cask libreoffice`. The script finds the app bundle on its own; `soffice` does not need to be on `PATH`.
