#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Inspect a delimited text file (CSV/TSV) and print a JSON report.

Reports encoding, BOM, newline style, delimiter, header, shape, ragged and
empty rows, `-` null usage, and sample rows. Read-only.

Usage: uv run scripts/peek.py <file> [--rows N]
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import sys
from collections import Counter
from pathlib import Path
from typing import NoReturn

ANALYSIS_CAP = 100 * 1024 * 1024  # bytes read for analysis
SNIFF_CHARS = 64 * 1024
CANDIDATE_DELIMITERS = ",\t;|"
BINARY_SUFFIXES = {".xlsx", ".xlsm", ".xls", ".ods", ".numbers", ".parquet"}
CELL_PREVIEW_LIMIT = 120


def fail(message: str, hint: str | None = None) -> NoReturn:
    payload: dict[str, str] = {"error": message}
    if hint:
        payload["hint"] = hint
    print(json.dumps(payload, indent=2))
    sys.exit(2)


def decode(raw: bytes) -> tuple[str, str, str | None]:
    """Return (text, encoding label, BOM label)."""
    if raw.startswith(b"\xef\xbb\xbf"):
        return raw[3:].decode("utf-8", errors="replace"), "utf-8", "utf-8"
    if raw.startswith(b"\xff\xfe"):
        return raw[2:].decode("utf-16-le", errors="replace"), "utf-16-le", "utf-16-le"
    if raw.startswith(b"\xfe\xff"):
        return raw[2:].decode("utf-16-be", errors="replace"), "utf-16-be", "utf-16-be"
    try:
        return raw.decode("utf-8"), "utf-8", None
    except UnicodeDecodeError:
        return raw.decode("latin-1"), "unknown-8bit (decoded as latin-1)", None


def newline_report(text: str) -> dict:
    crlf = text.count("\r\n")
    lf = text.count("\n") - crlf
    cr = text.count("\r") - crlf
    present = [name for name, count in (("crlf", crlf), ("lf", lf), ("cr", cr)) if count]
    style = present[0] if len(present) == 1 else ("mixed" if present else "none")
    return {
        "style": style,
        "counts": {"crlf": crlf, "lf": lf, "cr": cr},
        "trailing_newline": text.endswith(("\n", "\r")),
    }


def pick_delimiter(path: Path, text: str) -> tuple[str, str]:
    if path.suffix.lower() in {".tsv", ".tab"}:
        return "\t", "extension"
    try:
        dialect = csv.Sniffer().sniff(text[:SNIFF_CHARS], delimiters=CANDIDATE_DELIMITERS)
        return dialect.delimiter, "sniffed"
    except csv.Error:
        first_line = text.splitlines()[0] if text else ""
        counts = {candidate: first_line.count(candidate) for candidate in CANDIDATE_DELIMITERS}
        best = max(counts, key=lambda candidate: counts[candidate])
        if counts[best] > 0:
            return best, "counted (most frequent in first line)"
        return ",", "fallback (no delimiter found)"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("file", type=Path)
    parser.add_argument("--rows", type=int, default=5, help="sample rows to include (default: 5)")
    args = parser.parse_args()

    path: Path = args.file
    if not path.is_file():
        fail(f"{path} is not a file")

    size = path.stat().st_size
    with path.open("rb") as handle:
        raw = handle.read(ANALYSIS_CAP)
    truncated = size > len(raw)

    if path.suffix.lower() in BINARY_SUFFIXES or raw[:4] == b"PK\x03\x04":
        fail(
            f"{path.name} is a binary spreadsheet, not delimited text",
            "follow references/xlsx.md (openpyxl or DuckDB read_xlsx) instead of peek.py",
        )
    if b"\x00" in raw[:SNIFF_CHARS] and not raw.startswith((b"\xff\xfe", b"\xfe\xff")):
        fail(f"{path.name} looks binary (NUL bytes found)")

    text, encoding, bom = decode(raw)
    delimiter, delimiter_source = pick_delimiter(path, text)

    records = list(csv.reader(io.StringIO(text), delimiter=delimiter))
    if not records:
        fail(f"{path.name} is empty")

    header, *data = records
    width = len(header)
    duplicate_headers = sorted(name for name, count in Counter(header).items() if count > 1)

    ragged: list[dict[str, int]] = []
    empty = 0
    dash_nulls = 0
    for record_number, record in enumerate(data, start=2):  # header is record 1
        if not record or all(cell.strip() == "" for cell in record):
            empty += 1
            continue
        if len(record) != width:
            ragged.append({"record": record_number, "columns": len(record)})
        dash_nulls += sum(1 for cell in record if cell == "-")

    def preview(cell: str) -> str:
        return cell if len(cell) <= CELL_PREVIEW_LIMIT else cell[:CELL_PREVIEW_LIMIT] + "…"

    report = {
        "file": str(path),
        "size_bytes": size,
        "analysis_truncated": truncated,
        "encoding": encoding,
        "bom": bom,
        "newlines": newline_report(text),
        "delimiter": delimiter,
        "delimiter_source": delimiter_source,
        "columns": width,
        "header": header,
        "duplicate_headers": duplicate_headers,
        "data_rows": len(data),
        "empty_rows": empty,
        "ragged_rows": {"count": len(ragged), "first": ragged[:10]},
        "dash_null_cells": dash_nulls,
        "sample": [[preview(cell) for cell in record] for record in data[: args.rows]],
    }
    print(json.dumps(report, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
