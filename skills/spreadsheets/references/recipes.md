# Spreadsheet Recipes

Use these as starting points. Keep amounts as strings, preserve source data, and validate the output with `peek.py`.

## Exact Decimal Transform

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
import csv
from decimal import Decimal

with open("in.tsv", encoding="utf-8-sig", newline="") as f:
    reader = csv.DictReader(f, delimiter="\t")
    rows = list(reader)
    fields = list(reader.fieldnames or [])

if "value_usd" not in fields:
    fields.append("value_usd")

for row in rows:
    amount = Decimal(row["amount"])
    price = Decimal(row["price_usd"])
    row["value_usd"] = str(amount * price)

with open("out.tsv", "w", encoding="utf-8", newline="") as f:
    writer = csv.DictWriter(f, fieldnames=fields, delimiter="\t", lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
```

## Idempotent Append

Use multiset difference, not set difference: identical rows can be legitimate.

```python
from collections import Counter

existing_counts = Counter(tuple(row.items()) for row in existing_rows)
to_append = []
for row in candidate_rows:
    key = tuple(row.items())
    if existing_counts[key] > 0:
        existing_counts[key] -= 1
    else:
        to_append.append(row)
```

## Schema Validation

Generate a draft schema from representative data, edit it, then validate future files. `qsv schema` creates stats
sidecars next to its input, so run it against a temp copy when the source directory must stay clean.

```sh
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
cp txs.tsv "$tmpdir/txs.tsv"
(cd "$tmpdir" && qsv schema --stdout txs.tsv > txs.schema.json)
cp "$tmpdir/txs.schema.json" txs.schema.json
qsv validate schema txs.schema.json
qsv validate txs.tsv txs.schema.json
```

For house TSV validation after edits:

```sh
uv run "<skill-dir>/scripts/peek.py" txs.tsv --strict --house
```

## Keyed Diff

Use qsv when primary key values are unique; use daff for row/column-aware human review.

```sh
qsv extdedup --select tx_id txs.tsv --no-output
qsv diff --key tx_id --delimiter-output '\t' -o txs.diff.tsv before.tsv after.tsv
bunx daff before.tsv after.tsv
```

## External-Disclosure Profile

Use redaction when a report will be posted, published, or sent to a third party, or when the user asks. Internal agent
reports may include relevant unredacted samples.

```sh
uv run "<skill-dir>/scripts/profile.py" txs.tsv --markdown --redact-samples
```

The profile includes shape, issues, inferred types, null/cardinality signals, formula-prefix cells, and tool
availability without making transformation recommendations.

## Safe Workbook Creation

Use XlsxWriter for new workbooks and write formulas, not Python-computed constants.

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["XlsxWriter"]
# ///
import xlsxwriter

wb = xlsxwriter.Workbook("report.xlsx", {"constant_memory": True})
ws = wb.add_worksheet("Report")
header = wb.add_format({"bold": True})
money = wb.add_format({"num_format": "$#,##0.00"})
ws.write_row(0, 0, ["asset", "amount", "price_usd", "value_usd"], header)
ws.write_row(1, 0, ["ETH", 1.5, 2400])
ws.write_formula(1, 3, "=B2*C2", money)
ws.freeze_panes(1, 0)
wb.close()
```

Then run:

```sh
uv run "<skill-dir>/scripts/recalc.py" report.xlsx
```

Deliver only when the command exits `0` with `"status": "success"`.
