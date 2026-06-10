---
argument-hint: "[file]"
disable-model-invocation: false
name: spreadsheets
user-invocable: true
description: This skill should be used when a CSV, TSV, or Excel (.xlsx) file is the primary input or output — reading, inspecting, cleaning, transforming, deduplicating, merging, validating, or converting tabular data; creating a spreadsheet from data or fixing a messy one; or recalculating formulas. Trigger phrases include "open this CSV", "clean this spreadsheet", "convert xlsx to TSV", "merge these CSVs", "sum by category", "make a spreadsheet". Do not trigger when tabular data is incidental to a non-spreadsheet deliverable.
---

# Spreadsheets

Opinionated tabular-data handling for macOS. TSV/CSV is the primary format; `.xlsx` is the exception. Stack: `duckdb` and `qsv` for wrangling, `uv`-run Python (stdlib `csv` + `Decimal`) for transforms, `openpyxl` for Excel mechanics, headless LibreOffice for formula recalculation. Run all Python through `uv` — never bare `python` or `pip`.

## Tool Selection

| Job                                                          | Use                                                 |
| ------------------------------------------------------------ | --------------------------------------------------- |
| First look at an unknown CSV/TSV                             | `uv run scripts/peek.py <file>`                     |
| Counts, stats, frequencies, dedupe, column select            | `qsv` (infers the tab delimiter from `.tsv`)        |
| Joins, group-bys, pivots, cross-file SQL, format conversion  | `duckdb -c "..."`                                   |
| Row-level transforms, precision-critical edits               | `uv run` Python with a PEP 723 header               |
| Anything `.xlsx` in or out                                   | read [references/xlsx.md](references/xlsx.md) first |
| Recalculating `.xlsx` formulas                               | `uv run scripts/recalc.py <file.xlsx>`              |
| Row/column-aware diff of two tables                          | `bunx daff old.tsv new.tsv`                         |
| Interactive viewing (suggest to the user; never launch TUIs) | `csvlens`, `vd`, Numbers.app                        |

## Hard Rules

1. **Decimals, never floats.** Crypto amounts carry up to 18 decimals — beyond float64. Keep amounts as strings end to end; compute with `decimal.Decimal` or DuckDB `DECIMAL(38, 18)`. Read with `all_varchar = true` in DuckDB and plain stdlib `csv` in Python. If pandas is unavoidable, pass `dtype=str`.
2. **Touch only what was asked.** No reordering, re-quoting, renumbering, or whitespace "tidying" outside the requested change. The diff must contain the change and nothing else.
3. **House format for authored files**: TSV; UTF-8 without BOM; LF line endings; single trailing newline; lowercase `snake_case` headers; ISO 8601 dates (`YYYY-MM-DD`; prb-finance timestamps use `YYYY-MM-DD@HH:MM:SS`); `.` decimal point; no thousands separators or currency symbols inside cells; `-` for null. Conventions already present in an existing file override every one of these.
4. **Strip BOMs on read, never write them.** Open files of unknown provenance with `encoding="utf-8-sig"`.
5. **Validate after editing.** Re-run `peek.py` (column count unchanged, no new ragged rows) or the owning repo's checks. In prb-finance: `just tsv-check`, then `just cli::write-changed` to regenerate derived reports — never hand-edit generated `.pool.tsv`/`.annual.tsv`/`.md` artifacts.
6. **In-place edits are atomic.** Write to a temp file next to the target, verify it, then `mv` over the original.
7. **Finance data stays local.** Treat transaction logs and bank/exchange exports as private tax records: never send their contents to web services or external APIs.
8. **Escape spreadsheet formula injection** when writing cells sourced from external data: prefix a leading `=`, `+`, or `@` with `'` (a bare `-` null is exempt).

## Inspect

`uv run scripts/peek.py <file> [--rows N]` prints a JSON report: encoding and BOM, newline style and trailing newline, delimiter and how it was detected, header with duplicates flagged, column/row counts, ragged and empty rows, `-` null usage, and sample rows. Run it before editing any delimited file; on a binary spreadsheet it exits with a pointer to the xlsx workflow.

Quick follow-ups with `qsv`:

```sh
qsv count txs.tsv                 # row count (excludes header)
qsv headers txs.tsv               # numbered column names
qsv stats -E txs.tsv | qsv table  # per-column types, ranges, cardinality
qsv frequency -s event txs.tsv    # value distribution of one column
qsv select date_utc,amount txs.tsv
qsv dedup txs.tsv
```

qsv infers the input delimiter from the file extension, but stdout is always comma-separated. When the result must stay TSV, write it with `-o out.tsv` (the output extension sets the delimiter) — never shell redirection.

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
- After writing any formula, run `uv run scripts/recalc.py <file.xlsx>` and deliver only when it reports zero `#REF!`/`#DIV/0!`/`#VALUE!`/`#NAME?` errors.

Recalculation needs LibreOffice: `brew install --cask libreoffice`. The script finds the app bundle on its own; `soffice` does not need to be on `PATH`.
