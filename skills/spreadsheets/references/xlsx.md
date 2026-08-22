# Excel (.xlsx) Workflows

Contents: [Choose the Path](#choose-the-path) · [Read](#read) · [Create](#create) · [Edit](#edit) ·
[Formulas](#formulas) · [Recalculate](#recalculate) · [Convert](#convert) · [Formatting Defaults](#formatting-defaults)
· [Pitfalls](#pitfalls)

## Choose the Path

- **Values only** (analyze, extract, convert): use `qsv excel` for fast export/metadata, DuckDB for SQL over `.xlsx`, or
  `fastexcel` when Python/Polars/Arrow is truly needed. Prefer converting to TSV early and doing the real work there.
- **New workbook deliverable** (formulas, styling, tables, charts): build with XlsxWriter, then run the mandatory
  recalculation loop.
- **Existing workbook edit** (preserve sheets, formulas, macros where possible): use openpyxl surgically, then run the
  recalculation loop.

## Read

Fast metadata/export with qsv:

```sh
qsv excel --metadata J book.xlsx                 # sheet/table/header metadata as JSON
qsv excel --sheet Trades -d '\t' -q -o trades.tsv book.xlsx
qsv excel --table Table1 -d '\t' -q -o table1.tsv book.xlsx
```

DuckDB — values-only SQL, precision-safe:

```sql
FROM read_xlsx('book.xlsx', all_varchar = true);                    -- first sheet
FROM read_xlsx('book.xlsx', sheet = 'Trades', all_varchar = true);  -- named sheet
```

fastexcel — fast Python read when a dataframe/Arrow path is needed:

```sh
uv run --with fastexcel --with polars python -c "
import fastexcel, polars as pl
sheet = fastexcel.read_excel('book.xlsx').load_sheet_by_name('Trades')
df = pl.DataFrame(sheet)
print(df.shape)
"
```

openpyxl — existing workbook structure and formulas:

```python
from openpyxl import load_workbook

wb = load_workbook("book.xlsx")                        # formulas as strings
wb_values = load_workbook("book.xlsx", data_only=True) # cached values from last save
for ws in wb.worksheets:
    print(ws.title, ws.max_row, ws.max_column)
```

- `data_only=True` returns the values cached by the last application that saved the file. A workbook freshly written by
  openpyxl has no cache — formula cells read as `None` until recalculated.
- Never save a workbook loaded with `data_only=True`: formulas are silently replaced by values, permanently.
- Large values-only reads should not default to openpyxl; prefer `qsv excel`, DuckDB, or fastexcel.
- Bulk multi-sheet dump when pandas is genuinely convenient: `uv run --with pandas python -c "..."` with
  `pd.read_excel(path, sheet_name=None, dtype=str)` — `dtype=str` is non-negotiable for amount columns.

## Create

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["XlsxWriter"]
# ///
import xlsxwriter

wb = xlsxwriter.Workbook("portfolio.xlsx", {"constant_memory": True})
ws = wb.add_worksheet("Portfolio")
header = wb.add_format({"bold": True})
money = wb.add_format({"num_format": "$#,##0.00"})
ws.write_row(0, 0, ["asset", "amount", "price_usd", "value_usd"], header)
ws.write_row(1, 0, ["ETH", 1.5, 2400])
ws.write_formula(1, 3, "=B2*C2", money)
ws.write_row(2, 0, ["BTC", 0.25, 64000])
ws.write_formula(2, 3, "=B3*C3", money)
ws.write(3, 0, "total")
ws.write_formula(3, 3, "=SUM(D2:D3)", money)
ws.freeze_panes(1, 0)
ws.set_column(0, 0, 10)
ws.set_column(1, 3, 14)
wb.close()
```

Then recalculate — see [Recalculate](#recalculate).

## Edit

```python
from openpyxl import load_workbook

wb = load_workbook("book.xlsx")
ws = wb["Sheet1"]
ws["B2"] = "new value"
ws.insert_rows(3)
ws.delete_cols(5)
extra = wb.create_sheet("Extra")
wb.save("book.xlsx")
```

- Match the existing workbook's conventions — fonts, number formats, layout — exactly; never restyle while editing.
- Cell coordinates are 1-based: `ws.cell(row=1, column=1)` is `A1`.
- Keep the original file (or a copy) until the edited output is verified.

## Formulas

A spreadsheet must stay recalculable: when source cells change, derived cells must follow. Write formulas, not constants
computed in Python.

```python
ws["D10"] = "=SUM(D2:D9)"                        # not the Python-side sum
ws["E2"] = "=D2/$D$10"                           # absolute ref for a shared denominator
ws["B2"] = "=Inputs!B2*(1+Inputs!B3)"            # assumptions live in cells, not literals
```

- Put assumptions (rates, fees, multipliers) in dedicated cells and reference them; no magic numbers inside formulas.
- Guard divisions: `=IF(C2=0, 0, B2/C2)`.
- Mind the offset: with one header row, list/DataFrame row `N` lands on worksheet row `N + 2`. Verify two or three
  references against the actual data before filling a whole column.
- Cross-sheet references quote names containing spaces: `='FX Rates'!B2`.

## Recalculate

Python libraries write formula strings without computing them; errors only become visible after a real engine
recalculates. Always finish with:

```sh
uv run "<skill-dir>/scripts/recalc.py" book.xlsx [timeout-seconds]   # exits nonzero unless status is success
```

The script resolves LibreOffice (`soffice` on `PATH`, else the macOS app bundle under `/Applications` or
`~/Applications`), uses an isolated temporary LibreOffice profile, recalculates and saves the workbook in place, then
audits every cell and prints JSON:

```json
{
  "status": "errors_found",
  "total_formulas": 41,
  "uncached_formulas": 0,
  "total_errors": 2,
  "errors": { "#DIV/0!": { "count": 2, "cells": ["Portfolio!D7", "Portfolio!E7"] } }
}
```

Loop until `status` is `success`: fix the listed cells, rerun. Statuses:

- `success` — deliverable.
- `errors_found` — exits `1`; fix and rerun. Typical causes: `#REF!` broken references after inserting/deleting rows or
  columns; `#DIV/0!` unguarded division; `#VALUE!` text where a number is expected; `#NAME?` misspelled function or
  unquoted sheet name.
- `recalc_incomplete` — exits `1`; LibreOffice did not write cached values.
- `error` — exits `1`; the JSON `hint` says what to do (e.g. `brew install --cask libreoffice` when LibreOffice is
  missing).

Use `--soft` only when automation must capture a non-success report without failing the shell command.

## Convert

```sh
# xlsx/xls/xlsb/ods -> tsv (one sheet, fast values-only export)
qsv excel --sheet Trades -d '\t' -q -o trades.tsv book.xlsx

# xlsx -> tsv via DuckDB when you need SQL/range/filter control
duckdb -c "COPY (FROM read_xlsx('book.xlsx', sheet = 'Trades', all_varchar = true)) TO 'trades.tsv' (FORMAT csv, DELIMITER '\t', HEADER true)"

# tsv -> xlsx (values only, no styling)
duckdb -c "INSTALL excel; LOAD excel; COPY (FROM read_csv('data.tsv', delim = '\t', header = true, all_varchar = true)) TO 'data.xlsx' (FORMAT xlsx, HEADER true)"
```

`read_xlsx` autoloads DuckDB's excel extension; `COPY ... (FORMAT xlsx)` does not — keep the
`INSTALL excel; LOAD excel;` prefix.

`all_varchar` keeps amounts as text on both sides — the precision rule survives conversion. Type the columns only when
explicitly asked.

After any `.xlsx` to TSV export, validate the TSV before using or delivering it:

```sh
uv run "<skill-dir>/scripts/peek.py" book.tsv --strict
```

When replacing an existing TSV export and the sheet shape should not change, save a before-report first and finish with
`--expect-like`.

## Formatting Defaults

For new workbooks; conventions in an existing template always win.

- One font family for the whole workbook (Calibri or Arial); bold header row; freeze it (`ws.freeze_panes = "A2"`).
- Number formats, not data mangling: set `cell.number_format` (`"yyyy-mm-dd"`, `"0.0%"`, `"#,##0.00;(#,##0.00)"`)
  instead of writing formatted strings into cells.
- Money: a currency `number_format` like `"$#,##0.00"` or a unit-suffixed header (`value_usd`); never currency symbols
  inside cell values.
- Set column widths so nothing displays truncated.
- Zero formula errors at delivery — enforced by the recalc loop.

## Pitfalls

- Homebrew-cask LibreOffice stays Gatekeeper-quarantined, and `soffice` writes `.pyc` files into the app bundle on first
  run, breaking its signature seal — a later GUI launch then claims "LibreOffice.app is damaged". The app is fine; do
  not trash it: `xattr -dr com.apple.quarantine /Applications/LibreOffice.app` (or install with `--no-quarantine`).
  Headless recalculation is unaffected either way.
- openpyxl round-trips drop charts and images, and can degrade pivot tables and other advanced features. If a workbook
  contains them, confine edits to what was asked, save to a new file, and tell the user what may be lost.
- `.xlsm`: pass `keep_vba=True` to `load_workbook`, or the macros are stripped.
- Write `datetime`/`date` objects for date cells (with a date `number_format`), not strings — strings stay text and
  break date arithmetic.
- Column letters: use `openpyxl.utils.get_column_letter` / `column_index_from_string`; never hand-compute (column 64 is
  `BL`, not `BK`).
- `.numbers` files are out of scope: ask the user to export CSV/xlsx from Numbers first (`open -a Numbers <file>`).
