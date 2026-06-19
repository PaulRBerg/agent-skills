#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Inspect or validate a delimited text file (CSV/TSV) and print a JSON report.

Reports encoding, BOM, newline style, delimiter, header, shape, ragged and
empty rows, `-` null usage, issues, and sample rows. Read-only.

Usage: uv run scripts/peek.py <file> [--rows N] [--strict] [--expect-like REPORT]
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import sys
from collections import Counter
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Iterator, NoReturn

SNIFF_BYTES = 64 * 1024
CANDIDATE_DELIMITERS = ",\t;|"
BINARY_SUFFIXES = {".xlsx", ".xlsm", ".xls", ".ods", ".numbers", ".parquet"}
CELL_PREVIEW_LIMIT = 120
NEWLINE_CHUNK = 1024 * 1024


def fail(message: str, hint: str | None = None) -> NoReturn:
    payload: dict[str, str] = {"error": message}
    if hint:
        payload["hint"] = hint
    print(json.dumps(payload, indent=2))
    sys.exit(2)


def decode_sample(raw: bytes) -> tuple[str, str, str | None, int]:
    """Return (text, encoding label, BOM label, bytes to skip while streaming)."""
    if raw.startswith(b"\xef\xbb\xbf"):
        return raw[3:].decode("utf-8", errors="replace"), "utf-8", "utf-8", 3
    if raw.startswith(b"\xff\xfe"):
        return raw[2:].decode("utf-16-le", errors="replace"), "utf-16-le", "utf-16-le", 2
    if raw.startswith(b"\xfe\xff"):
        return raw[2:].decode("utf-16-be", errors="replace"), "utf-16-be", "utf-16-be", 2
    try:
        return raw.decode("utf-8"), "utf-8", None, 0
    except UnicodeDecodeError:
        return raw.decode("latin-1"), "unknown-8bit (decoded as latin-1)", None, 0


def stream_codec(encoding_label: str) -> str:
    if encoding_label.startswith("unknown-8bit"):
        return "latin-1"
    return encoding_label


@contextmanager
def open_text(path: Path, encoding: str, skip_bytes: int = 0) -> Iterator[io.TextIOWrapper]:
    raw = path.open("rb")
    try:
        if skip_bytes:
            raw.seek(skip_bytes)
        with io.TextIOWrapper(raw, encoding=encoding, errors="replace", newline="") as text:
            yield text
    finally:
        if not raw.closed:
            raw.close()


def newline_report(path: Path, encoding: str, skip_bytes: int) -> dict:
    crlf = 0
    lf = 0
    cr = 0
    pending_cr = False
    last_char: str | None = None

    with open_text(path, encoding, skip_bytes) as handle:
        while True:
            chunk = handle.read(NEWLINE_CHUNK)
            if chunk == "":
                break
            last_char = chunk[-1]
            if pending_cr:
                chunk = "\r" + chunk
                pending_cr = False
            if chunk.endswith("\r"):
                pending_cr = True
                chunk = chunk[:-1]

            crlf += chunk.count("\r\n")
            without_crlf = chunk.replace("\r\n", "")
            lf += without_crlf.count("\n")
            cr += without_crlf.count("\r")

    if pending_cr:
        cr += 1

    present = [name for name, count in (("crlf", crlf), ("lf", lf), ("cr", cr)) if count]
    style = present[0] if len(present) == 1 else ("mixed" if present else "none")
    return {
        "style": style,
        "counts": {"crlf": crlf, "lf": lf, "cr": cr},
        "trailing_newline": last_char in ("\n", "\r"),
    }


def pick_delimiter(path: Path, text: str) -> tuple[str, str]:
    if path.suffix.lower() in {".tsv", ".tab"}:
        return "\t", "extension"
    try:
        dialect = csv.Sniffer().sniff(text[:SNIFF_BYTES], delimiters=CANDIDATE_DELIMITERS)
        return dialect.delimiter, "sniffed"
    except csv.Error:
        first_line = text.splitlines()[0] if text else ""
        counts = {candidate: first_line.count(candidate) for candidate in CANDIDATE_DELIMITERS}
        best = max(counts, key=lambda candidate: counts[candidate])
        if counts[best] > 0:
            return best, "counted (most frequent in first line)"
        return ",", "fallback (no delimiter found)"


def preview(cell: str) -> str:
    return cell if len(cell) <= CELL_PREVIEW_LIMIT else cell[:CELL_PREVIEW_LIMIT] + "…"


def issue(code: str, message: str, **details: Any) -> dict[str, Any]:
    payload: dict[str, Any] = {"code": code, "message": message}
    payload.update(details)
    return payload


def delimiter_label(delimiter: str) -> str:
    return "\\t" if delimiter == "\t" else delimiter


def structural_issues(report: dict[str, Any]) -> list[dict[str, Any]]:
    issues: list[dict[str, Any]] = []
    if report["encoding"] != "utf-8":
        issues.append(issue("non_utf8_encoding", "file encoding is not UTF-8", encoding=report["encoding"]))
    if report["bom"] is not None:
        issues.append(issue("bom_present", "file has a BOM", bom=report["bom"]))
    if report["newlines"]["style"] not in ("lf", "none"):
        issues.append(
            issue(
                "newline_style_not_lf",
                "newline style is not LF",
                style=report["newlines"]["style"],
                counts=report["newlines"]["counts"],
            )
        )
    if report["size_bytes"] > 0 and not report["newlines"]["trailing_newline"]:
        issues.append(issue("missing_trailing_newline", "file is missing a trailing newline"))
    if report["duplicate_headers"]:
        issues.append(
            issue(
                "duplicate_headers",
                "duplicate header names",
                headers=report["duplicate_headers"],
            )
        )
    if report["ragged_rows"]["count"]:
        issues.append(
            issue(
                "ragged_rows",
                "rows with column counts different from the header",
                count=report["ragged_rows"]["count"],
                first=report["ragged_rows"]["first"],
            )
        )
    if report["empty_rows"]:
        issues.append(issue("empty_rows", "empty data rows", count=report["empty_rows"]))
    return issues


def load_expected_report(path: Path) -> dict[str, Any]:
    try:
        with path.open(encoding="utf-8") as handle:
            payload = json.load(handle)
    except OSError as exc:
        fail(f"could not read expected report {path}: {exc}")
    except json.JSONDecodeError as exc:
        fail(f"{path} is not valid JSON: {exc}")
    if not isinstance(payload, dict):
        fail(f"{path} is not a peek.py JSON object")
    return payload


def nested(payload: dict[str, Any], *keys: str) -> Any:
    value: Any = payload
    for key in keys:
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    return value


def expected_count(expected: dict[str, Any], *keys: str) -> int:
    value = nested(expected, *keys)
    if value is None:
        return 0
    if type(value) is int:
        return value
    fail(f"expected report has non-integer {'.'.join(keys)}: {value!r}")


def compare_expected_like(report: dict[str, Any], expected: dict[str, Any]) -> list[dict[str, Any]]:
    issues: list[dict[str, Any]] = []

    comparisons = [
        (("columns",), "columns_changed", "column count changed"),
        (("header",), "header_changed", "header changed"),
        (("delimiter",), "delimiter_changed", "delimiter changed"),
        (("encoding",), "encoding_changed", "encoding changed"),
        (("newlines", "style"), "newline_style_changed", "newline style changed"),
        (("newlines", "trailing_newline"), "trailing_newline_changed", "trailing newline changed"),
        (("data_rows",), "data_rows_changed", "data row count changed"),
    ]
    for keys, code, message in comparisons:
        before = nested(expected, *keys)
        after = nested(report, *keys)
        if before != after:
            if keys == ("delimiter",):
                before = delimiter_label(str(before))
                after = delimiter_label(str(after))
            issues.append(issue(code, message, before=before, after=after))

    before_ragged = expected_count(expected, "ragged_rows", "count")
    after_ragged = int(nested(report, "ragged_rows", "count") or 0)
    if after_ragged > before_ragged:
        issues.append(issue("new_ragged_rows", "new ragged rows", before=before_ragged, after=after_ragged))

    before_empty = expected_count(expected, "empty_rows")
    after_empty = int(report.get("empty_rows") or 0)
    if after_empty > before_empty:
        issues.append(issue("new_empty_rows", "new empty rows", before=before_empty, after=after_empty))

    before_duplicates = set(expected.get("duplicate_headers") or [])
    after_duplicates = set(report.get("duplicate_headers") or [])
    new_duplicates = sorted(after_duplicates - before_duplicates)
    if new_duplicates:
        issues.append(issue("new_duplicate_headers", "new duplicate headers", headers=new_duplicates))

    if expected.get("bom") is None and report.get("bom") is not None:
        issues.append(issue("new_bom", "new BOM introduced", bom=report["bom"]))

    return issues


def parse_rows(path: Path, encoding: str, skip_bytes: int, delimiter: str, sample_rows: int) -> dict[str, Any]:
    with open_text(path, encoding, skip_bytes) as handle:
        reader = csv.reader(handle, delimiter=delimiter)
        try:
            header = next(reader)
        except StopIteration:
            fail(f"{path.name} is empty")
        except csv.Error as exc:
            fail(f"{path.name} is not parseable as delimited text: {exc}")

        width = len(header)
        duplicate_headers = sorted(name for name, count in Counter(header).items() if count > 1)

        ragged_count = 0
        ragged_first: list[dict[str, int]] = []
        empty = 0
        dash_nulls = 0
        data_rows = 0
        sample: list[list[str]] = []

        try:
            for record_number, record in enumerate(reader, start=2):  # header is record 1
                data_rows += 1
                if len(sample) < sample_rows:
                    sample.append([preview(cell) for cell in record])
                if not record or all(cell.strip() == "" for cell in record):
                    empty += 1
                    continue
                if len(record) != width:
                    ragged_count += 1
                    if len(ragged_first) < 10:
                        ragged_first.append({"record": record_number, "columns": len(record)})
                dash_nulls += sum(1 for cell in record if cell == "-")
        except csv.Error as exc:
            fail(f"{path.name} is not parseable as delimited text: {exc}")

    return {
        "columns": width,
        "header": header,
        "duplicate_headers": duplicate_headers,
        "data_rows": data_rows,
        "empty_rows": empty,
        "ragged_rows": {"count": ragged_count, "first": ragged_first},
        "dash_null_cells": dash_nulls,
        "sample": sample,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("file", type=Path)
    parser.add_argument("--rows", type=int, default=5, help="sample rows to include (default: 5)")
    parser.add_argument("--strict", action="store_true", help="exit 1 on structural or house-format issues")
    parser.add_argument("--expect-like", type=Path, help="exit 1 if shape/format drift from a previous peek report")
    parser.add_argument("--expect-columns", type=int, help="exit 1 unless the file has this many columns")
    args = parser.parse_args()

    if args.rows < 0:
        fail("--rows must be >= 0")
    if args.expect_columns is not None and args.expect_columns < 1:
        fail("--expect-columns must be >= 1")

    path: Path = args.file
    if not path.is_file():
        fail(f"{path} is not a file")

    size = path.stat().st_size
    with path.open("rb") as handle:
        raw = handle.read(SNIFF_BYTES)

    if path.suffix.lower() in BINARY_SUFFIXES or raw[:4] == b"PK\x03\x04":
        fail(
            f"{path.name} is a binary spreadsheet, not delimited text",
            "follow references/xlsx.md (openpyxl or DuckDB read_xlsx) instead of peek.py",
        )
    if b"\x00" in raw[:SNIFF_BYTES] and not raw.startswith((b"\xff\xfe", b"\xfe\xff")):
        fail(f"{path.name} looks binary (NUL bytes found)")

    text, encoding, bom, skip_bytes = decode_sample(raw)
    codec = stream_codec(encoding)
    delimiter, delimiter_source = pick_delimiter(path, text)
    row_report = parse_rows(path, codec, skip_bytes, delimiter, args.rows)

    report: dict[str, Any] = {
        "file": str(path),
        "size_bytes": size,
        "analysis_truncated": False,
        "encoding": encoding,
        "bom": bom,
        "newlines": newline_report(path, codec, skip_bytes),
        "delimiter": delimiter,
        "delimiter_source": delimiter_source,
    }
    report.update(row_report)

    reported_issues = structural_issues(report)
    should_fail = args.strict and bool(reported_issues)

    if args.expect_columns is not None and report["columns"] != args.expect_columns:
        reported_issues.append(
            issue(
                "expected_columns_mismatch",
                "column count does not match --expect-columns",
                expected=args.expect_columns,
                actual=report["columns"],
            )
        )
        should_fail = True

    if args.expect_like:
        expected = load_expected_report(args.expect_like)
        expected_issues = compare_expected_like(report, expected)
        reported_issues.extend(expected_issues)
        if expected_issues:
            should_fail = True

    report["status"] = "issues_found" if reported_issues else "ok"
    report["issues"] = reported_issues
    print(json.dumps(report, indent=2, ensure_ascii=False))
    if should_fail:
        sys.exit(1)


if __name__ == "__main__":
    main()
