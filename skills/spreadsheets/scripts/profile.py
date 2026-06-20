#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Profile a delimited spreadsheet file and print JSON or Markdown.

Read-only. Uses peek.py for structure, qsv for fast column statistics when
available, and a local scan for spreadsheet formula-injection risks.

Usage:
  uv run scripts/profile.py data.tsv
  uv run scripts/profile.py data.tsv --markdown --redact-samples
"""

from __future__ import annotations

import argparse
import csv
import json
import shutil
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any

import peek

FORMULA_PREFIXES = ("=", "+", "@")
MAX_FINDINGS = 20
XLSX_SUFFIXES = {".xlsx", ".xlsm", ".xls", ".xlsb", ".ods"}


def run_command(args: list[str], timeout: int = 60) -> dict[str, Any]:
    try:
        result = subprocess.run(args, capture_output=True, text=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"ok": False, "error": str(exc), "args": args}
    return {
        "ok": result.returncode == 0,
        "returncode": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "args": args,
    }


def tool_version(binary: str) -> dict[str, Any]:
    path = shutil.which(binary)
    if path is None:
        return {"available": False}
    result = run_command([path, "--version"], timeout=10)
    version = (result.get("stdout") or result.get("stderr") or "").strip().splitlines()
    return {"available": True, "path": path, "version": version[0] if version else None}


def qsv_args(command: str, delimiter: str, file: Path, extra: list[str] | None = None) -> list[str] | None:
    qsv = shutil.which("qsv")
    if qsv is None:
        return None
    args = [qsv, command]
    if extra:
        args.extend(extra)
    args.extend(["-d", delimiter, str(file)])
    return args


def parse_csv_stdout(stdout: str) -> list[dict[str, str]]:
    return list(csv.DictReader(stdout.splitlines()))


def collect_stats(file: Path, delimiter: str) -> dict[str, Any]:
    args = qsv_args("stats", delimiter, file, ["--cache-threshold", "0", "--cardinality"])
    if args is None:
        return {"available": False}
    result = run_command(args)
    if not result["ok"]:
        return {
            "available": True,
            "ok": False,
            "stderr": result.get("stderr", "").strip(),
            "stdout": result.get("stdout", "").strip(),
        }
    rows = parse_csv_stdout(result["stdout"])
    columns = []
    for row in rows:
        columns.append(
            {
                "field": row.get("field", ""),
                "type": row.get("type", ""),
                "nullcount": parse_number(row.get("nullcount", "")),
                "sparsity": parse_number(row.get("sparsity", "")),
                "cardinality": parse_number(row.get("cardinality", "")),
                "uniqueness_ratio": parse_number(row.get("uniqueness_ratio", "")),
                "min": row.get("min", ""),
                "max": row.get("max", ""),
                "max_precision": row.get("max_precision", ""),
            }
        )
    return {"available": True, "ok": True, "columns": columns}


def collect_frequency(file: Path, delimiter: str, top: int, redact: bool) -> dict[str, Any]:
    args = qsv_args("frequency", delimiter, file, ["--limit", str(top), "--json", "--no-stats"])
    if args is None:
        return {"available": False}
    result = run_command(args)
    if not result["ok"]:
        return {
            "available": True,
            "ok": False,
            "stderr": result.get("stderr", "").strip(),
            "stdout": result.get("stdout", "").strip(),
        }
    try:
        payload = json.loads(result["stdout"])
    except json.JSONDecodeError as exc:
        return {"available": True, "ok": False, "error": str(exc)}
    if redact:
        for field in payload.get("fields", []):
            for frequency in field.get("frequencies", []):
                value = frequency.get("value")
                if value not in ("", "-", "<ALL_UNIQUE>", "HIGH_CARDINALITY"):
                    frequency["value"] = "<redacted>"
    return {"available": True, "ok": True, "report": payload}


def parse_number(value: str) -> int | float | str | None:
    if value == "":
        return None
    try:
        if "." in value:
            return float(value)
        return int(value)
    except ValueError:
        return value


def scan_formula_injection(
    file: Path,
    delimiter: str,
    encoding: str,
    header: list[str],
    redact: bool,
) -> dict[str, Any]:
    codec = peek.stream_codec(encoding)
    findings: list[dict[str, Any]] = []
    count = 0
    try:
        with peek.open_text(file, codec) as handle:
            reader = csv.reader(handle, delimiter=delimiter)
            next(reader, None)
            for row_number, row in enumerate(reader, start=2):
                for index, cell in enumerate(row):
                    if cell.startswith(FORMULA_PREFIXES):
                        count += 1
                        if len(findings) < MAX_FINDINGS:
                            findings.append(
                                {
                                    "record": row_number,
                                    "column": header[index] if index < len(header) else str(index + 1),
                                    "value": "<redacted>" if redact else peek.preview(cell, False),
                                }
                            )
    except csv.Error as exc:
        return {"count": count, "first": findings, "error": str(exc)}
    return {"count": count, "first": findings}


def header_quality(header: list[str]) -> dict[str, Any]:
    duplicate_headers = sorted(name for name, count in Counter(header).items() if count > 1)
    unsafe = [name for name in header if not peek.HOUSE_HEADER_RE.fullmatch(name)]
    empty = [index + 1 for index, name in enumerate(header) if name == ""]
    return {"duplicates": duplicate_headers, "unsafe_house_headers": unsafe, "empty_header_positions": empty}


def recommendations(report: dict[str, Any]) -> list[str]:
    recs: list[str] = []
    peek_report = report.get("peek", {})
    if peek_report.get("issues"):
        recs.append("Fix structural issues before transforming or aggregating.")
    if not report["tools"]["qsv"]["available"]:
        recs.append("Install qsv for faster validation, profiling, frequency, and Excel export workflows.")
    if not report["tools"]["duckdb"]["available"]:
        recs.append("Install DuckDB for SQL joins, pivots, and exact DECIMAL aggregations.")
    formula_count = report.get("formula_injection", {}).get("count", 0)
    if formula_count:
        recs.append("Escape external cells starting with =, +, or @ before writing to CSV/XLSX.")
    high_cardinality = [
        col["field"]
        for col in report.get("stats", {}).get("columns", [])
        if isinstance(col.get("uniqueness_ratio"), (float, int)) and col["uniqueness_ratio"] >= 0.95
    ]
    if high_cardinality:
        recs.append("Treat high-uniqueness columns as identifiers; avoid full frequency tables on them.")
    if not recs:
        recs.append("File is structurally clean; proceed with qsv for simple operations or DuckDB for SQL.")
    return recs


def profile_delimited(args: argparse.Namespace) -> dict[str, Any]:
    peek_report, _should_fail = peek.inspect_path(
        args.file,
        rows=args.rows,
        strict=False,
        engine=args.engine,
        house=args.house,
        redact_samples=args.redact_samples,
    )
    delimiter = peek_report["delimiter"]
    report: dict[str, Any] = {
        "file": str(args.file),
        "kind": "delimited",
        "tools": {"qsv": tool_version("qsv"), "duckdb": tool_version("duckdb")},
        "peek": peek_report,
        "header_quality": header_quality(peek_report["header"]),
        "stats": collect_stats(args.file, delimiter),
        "frequency": collect_frequency(args.file, delimiter, args.top, args.redact_samples),
        "formula_injection": scan_formula_injection(
            args.file,
            delimiter,
            peek_report["encoding"],
            peek_report["header"],
            args.redact_samples,
        ),
    }
    report["recommendations"] = recommendations(report)
    report["status"] = "issues_found" if peek_report["issues"] or report["formula_injection"]["count"] else "ok"
    return report


def profile_workbook(args: argparse.Namespace) -> dict[str, Any]:
    qsv = shutil.which("qsv")
    metadata: dict[str, Any]
    if qsv is None:
        metadata = {"available": False}
    else:
        result = run_command([qsv, "excel", "--metadata", "J", str(args.file)])
        if result["ok"]:
            try:
                metadata = {"available": True, "ok": True, "report": json.loads(result["stdout"])}
            except json.JSONDecodeError as exc:
                metadata = {"available": True, "ok": False, "error": str(exc)}
        else:
            metadata = {
                "available": True,
                "ok": False,
                "stdout": result.get("stdout", "").strip(),
                "stderr": result.get("stderr", "").strip(),
            }
    return {
        "file": str(args.file),
        "kind": "workbook",
        "status": "ok" if metadata.get("ok") else "needs_manual_workflow",
        "tools": {"qsv": tool_version("qsv"), "duckdb": tool_version("duckdb")},
        "metadata": metadata,
        "recommendations": [
            "Use qsv excel for fast values-only export or sheet metadata.",
            "Use DuckDB read_xlsx(all_varchar=true) for SQL over .xlsx values.",
            "Use openpyxl only for editing existing workbook structure or formulas.",
            "Run recalc.py after writing formulas and treat non-success as a failed delivery.",
        ],
    }


def markdown_table_row(values: list[Any]) -> str:
    escaped = [str(value if value is not None else "").replace("|", "\\|") for value in values]
    return "| " + " | ".join(escaped) + " |"


def render_markdown(report: dict[str, Any]) -> str:
    lines = ["# Spreadsheet Profile", ""]
    lines.append(f"- File: `{report['file']}`")
    lines.append(f"- Kind: `{report['kind']}`")
    lines.append(f"- Status: `{report['status']}`")
    if report["kind"] == "delimited":
        peek_report = report["peek"]
        lines.append(f"- Shape: {peek_report['data_rows']} rows x {peek_report['columns']} columns")
        lines.append(f"- Delimiter: `{peek.delimiter_label(peek_report['delimiter'])}`")
        lines.append("")
        lines.append("## Issues")
        if peek_report["issues"]:
            lines.extend(f"- `{item['code']}`: {item['message']}" for item in peek_report["issues"])
        else:
            lines.append("- None")
        if report["formula_injection"]["count"]:
            lines.append(f"- `formula_injection`: {report['formula_injection']['count']} risky cells")
        lines.append("")
        lines.append("## Columns")
        lines.append(markdown_table_row(["field", "type", "nulls", "cardinality", "unique"]))
        lines.append(markdown_table_row(["---", "---", "---", "---", "---"]))
        for column in report.get("stats", {}).get("columns", [])[:30]:
            lines.append(
                markdown_table_row(
                    [
                        column.get("field"),
                        column.get("type"),
                        column.get("nullcount"),
                        column.get("cardinality"),
                        column.get("uniqueness_ratio"),
                    ]
                )
            )
    else:
        lines.append("")
        lines.append("## Workbook Metadata")
        metadata = report["metadata"]
        if metadata.get("ok"):
            payload = metadata.get("report", {})
            lines.append(f"- Sheets: {payload.get('sheet_count', payload.get('number_of_sheets', 'unknown'))}")
        else:
            lines.append(f"- Metadata unavailable: {metadata.get('stderr') or metadata.get('error') or 'qsv missing'}")
    lines.append("")
    lines.append("## Recommendations")
    lines.extend(f"- {item}" for item in report["recommendations"])
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("file", type=Path)
    parser.add_argument("--rows", type=int, default=5, help="sample rows to include from peek.py")
    parser.add_argument("--top", type=int, default=5, help="top values per column for qsv frequency")
    parser.add_argument("--house", action="store_true", help="also enforce authored TSV house conventions")
    parser.add_argument("--redact-samples", action="store_true", help="redact sample and frequency values")
    parser.add_argument("--markdown", action="store_true", help="print Markdown instead of JSON")
    parser.add_argument("--engine", choices=("auto", "python"), default="auto", help="peek.py engine")
    args = parser.parse_args()

    if args.top < 1:
        peek.fail("--top must be >= 1")
    if not args.file.is_file():
        peek.fail(f"{args.file} is not a file")

    suffix = args.file.suffix.lower()
    report = profile_workbook(args) if suffix in XLSX_SUFFIXES else profile_delimited(args)
    if args.markdown:
        print(render_markdown(report), end="")
    else:
        print(json.dumps(report, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
