#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Inspect or validate a delimited text file (CSV/TSV) and print a JSON report.

Reports encoding, BOM, newline style, delimiter, header, shape, ragged and
empty rows, `-` null usage, issues, and sample rows. Read-only.

Usage:
  uv run scripts/peek.py <file> [--rows N] [--strict] [--expect-like REPORT]
  uv run scripts/peek.py <file> --house --redact-samples
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import re
import shutil
import subprocess
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
HOUSE_HEADER_RE = re.compile(r"^[a-z][a-z0-9_]*$")


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


def summarize_newlines(crlf: int, lf: int, cr: int, trailing_newline: bool) -> dict[str, Any]:
    present = [name for name, count in (("crlf", crlf), ("lf", lf), ("cr", cr)) if count]
    style = present[0] if len(present) == 1 else ("mixed" if present else "none")
    return {
        "style": style,
        "counts": {"crlf": crlf, "lf": lf, "cr": cr},
        "trailing_newline": trailing_newline,
    }


def newline_report(path: Path, encoding: str, skip_bytes: int) -> dict[str, Any]:
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

    return summarize_newlines(crlf, lf, cr, last_char in ("\n", "\r"))


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


def preview(cell: str, redact: bool) -> str:
    if redact and cell not in ("", "-"):
        return "<redacted>"
    return cell if len(cell) <= CELL_PREVIEW_LIMIT else cell[:CELL_PREVIEW_LIMIT] + "..."


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
    validation = report.get("qsv_validation")
    if validation and validation.get("available") and "ok" in validation and not validation.get("ok"):
        issues.append(
            issue(
                "qsv_validation_failed",
                "qsv could not validate the file as RFC 4180-compatible CSV",
                detail=validation.get("stderr") or validation.get("stdout"),
            )
        )
    return issues


def house_issues(report: dict[str, Any]) -> list[dict[str, Any]]:
    issues: list[dict[str, Any]] = []
    if report["delimiter"] != "\t":
        issues.append(
            issue(
                "house_delimiter_not_tsv",
                "authored spreadsheet files should be TSV",
                delimiter=delimiter_label(report["delimiter"]),
            )
        )
    unsafe_headers = [header for header in report["header"] if not HOUSE_HEADER_RE.fullmatch(header)]
    if unsafe_headers:
        issues.append(
            issue(
                "house_headers_not_snake_case",
                "headers are not lowercase snake_case",
                headers=unsafe_headers,
            )
        )
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


def parse_rows_python(
    path: Path,
    encoding: str,
    skip_bytes: int,
    delimiter: str,
    sample_rows: int,
    redact_samples: bool,
) -> dict[str, Any]:
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
                    sample.append([preview(cell, redact_samples) for cell in record])
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


def strip_record_ending(raw_line: bytes) -> tuple[bytes, str | None]:
    if raw_line.endswith(b"\r\n"):
        return raw_line[:-2], "crlf"
    if raw_line.endswith(b"\n"):
        return raw_line[:-1], "lf"
    if raw_line.endswith(b"\r"):
        return raw_line[:-1], "cr"
    return raw_line, None


def decode_fields(raw_fields: list[bytes]) -> list[str] | None:
    try:
        return [field.decode("utf-8") for field in raw_fields]
    except UnicodeDecodeError:
        return None


def parse_rows_unquoted_fast(
    path: Path,
    skip_bytes: int,
    delimiter: str,
    sample_rows: int,
    redact_samples: bool,
) -> tuple[dict[str, Any], dict[str, Any]] | None:
    """Fast parser for simple delimited files without quotes or embedded newlines."""
    delimiter_bytes = delimiter.encode("utf-8")
    if len(delimiter_bytes) != 1:
        return None

    crlf = 0
    lf = 0
    cr = 0
    trailing_newline = False
    header: list[str] | None = None
    width = 0
    duplicate_headers: list[str] = []
    ragged_count = 0
    ragged_first: list[dict[str, int]] = []
    empty = 0
    dash_nulls = 0
    data_rows = 0
    sample: list[list[str]] = []

    with path.open("rb") as handle:
        if skip_bytes:
            handle.seek(skip_bytes)
        for raw_record_number, raw_line in enumerate(handle, start=1):
            record_bytes, ending = strip_record_ending(raw_line)
            trailing_newline = ending is not None
            if ending == "crlf":
                crlf += 1
            elif ending == "lf":
                lf += 1
            elif ending == "cr":
                cr += 1
            if b'"' in record_bytes or b"\r" in record_bytes:
                return None

            if raw_record_number == 1:
                header = decode_fields(record_bytes.split(delimiter_bytes))
                if header is None:
                    return None
                width = len(header)
                duplicate_headers = sorted(name for name, count in Counter(header).items() if count > 1)
                continue

            data_rows += 1
            if record_bytes == b"":
                empty += 1
                if len(sample) < sample_rows:
                    sample.append([])
                continue

            record = decode_fields(record_bytes.split(delimiter_bytes))
            if record is None:
                return None
            if len(sample) < sample_rows:
                sample.append([preview(cell, redact_samples) for cell in record])
            if all(cell.strip() == "" for cell in record):
                empty += 1
                continue
            if len(record) != width:
                ragged_count += 1
                if len(ragged_first) < 10:
                    ragged_first.append({"record": raw_record_number, "columns": len(record)})
            dash_nulls += sum(1 for cell in record if cell == "-")

    if header is None:
        fail(f"{path.name} is empty")

    row_report = {
        "columns": width,
        "header": header,
        "duplicate_headers": duplicate_headers,
        "data_rows": data_rows,
        "empty_rows": empty,
        "ragged_rows": {"count": ragged_count, "first": ragged_first},
        "dash_null_cells": dash_nulls,
        "sample": sample,
    }
    return row_report, summarize_newlines(crlf, lf, cr, trailing_newline)


def qsv_validation(path: Path, delimiter: str, encoding: str) -> dict[str, Any]:
    qsv = shutil.which("qsv")
    if qsv is None:
        return {"available": False}
    if encoding != "utf-8":
        return {"available": True, "skipped": "non-utf8"}
    try:
        result = subprocess.run(
            [qsv, "validate", "-d", delimiter, str(path)],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"available": True, "ok": False, "error": str(exc)}
    return {
        "available": True,
        "ok": result.returncode == 0,
        "returncode": result.returncode,
        "stdout": result.stdout.strip(),
        "stderr": result.stderr.strip(),
    }


def inspect_path(
    path: Path,
    *,
    rows: int = 5,
    strict: bool = False,
    expect_like: Path | None = None,
    expect_columns: int | None = None,
    engine: str = "auto",
    house: bool = False,
    redact_samples: bool = False,
) -> tuple[dict[str, Any], bool]:
    if rows < 0:
        fail("--rows must be >= 0")
    if expect_columns is not None and expect_columns < 1:
        fail("--expect-columns must be >= 1")
    if not path.is_file():
        fail(f"{path} is not a file")

    size = path.stat().st_size
    with path.open("rb") as handle:
        raw = handle.read(SNIFF_BYTES)

    if path.suffix.lower() in BINARY_SUFFIXES or raw[:4] == b"PK\x03\x04":
        fail(
            f"{path.name} is a binary spreadsheet, not delimited text",
            "follow references/xlsx.md (openpyxl, qsv excel, or DuckDB read_xlsx) instead of peek.py",
        )
    if b"\x00" in raw[:SNIFF_BYTES] and not raw.startswith((b"\xff\xfe", b"\xfe\xff")):
        fail(f"{path.name} looks binary (NUL bytes found)")

    text, encoding, bom, skip_bytes = decode_sample(raw)
    codec = stream_codec(encoding)
    delimiter, delimiter_source = pick_delimiter(path, text)

    validation = qsv_validation(path, delimiter, encoding) if engine == "auto" else {"available": False}
    engine_used = "python-csv"
    row_report: dict[str, Any]
    newlines: dict[str, Any]

    fast_report = None
    if engine == "auto" and encoding == "utf-8":
        fast_report = parse_rows_unquoted_fast(path, skip_bytes, delimiter, rows, redact_samples)
    if fast_report is not None:
        row_report, newlines = fast_report
        engine_used = "python-fast-unquoted"
    else:
        row_report = parse_rows_python(path, codec, skip_bytes, delimiter, rows, redact_samples)
        newlines = newline_report(path, codec, skip_bytes)

    report: dict[str, Any] = {
        "file": str(path),
        "size_bytes": size,
        "analysis_truncated": False,
        "engine_requested": engine,
        "engine": engine_used,
        "encoding": encoding,
        "bom": bom,
        "newlines": newlines,
        "delimiter": delimiter,
        "delimiter_source": delimiter_source,
        "qsv_validation": validation,
    }
    report.update(row_report)

    reported_issues = structural_issues(report)
    if house:
        reported_issues.extend(house_issues(report))
    should_fail = (strict or house) and bool(reported_issues)

    if expect_columns is not None and report["columns"] != expect_columns:
        reported_issues.append(
            issue(
                "expected_columns_mismatch",
                "column count does not match --expect-columns",
                expected=expect_columns,
                actual=report["columns"],
            )
        )
        should_fail = True

    if expect_like:
        expected = load_expected_report(expect_like)
        expected_issues = compare_expected_like(report, expected)
        reported_issues.extend(expected_issues)
        if expected_issues:
            should_fail = True

    report["status"] = "issues_found" if reported_issues else "ok"
    report["issues"] = reported_issues
    return report, should_fail


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("file", type=Path)
    parser.add_argument("--rows", type=int, default=5, help="sample rows to include (default: 5)")
    parser.add_argument("--strict", action="store_true", help="exit 1 on structural issues")
    parser.add_argument("--house", action="store_true", help="also enforce authored TSV house conventions")
    parser.add_argument("--redact-samples", action="store_true", help="redact non-null sample cell values")
    parser.add_argument("--expect-like", type=Path, help="exit 1 if shape/format drift from a previous peek report")
    parser.add_argument("--expect-columns", type=int, help="exit 1 unless the file has this many columns")
    parser.add_argument(
        "--engine",
        choices=("auto", "python"),
        default="auto",
        help="auto uses the fast unquoted parser and qsv validation when safe; python forces csv.reader",
    )
    args = parser.parse_args()

    report, should_fail = inspect_path(
        args.file,
        rows=args.rows,
        strict=args.strict,
        expect_like=args.expect_like,
        expect_columns=args.expect_columns,
        engine=args.engine,
        house=args.house,
        redact_samples=args.redact_samples,
    )
    print(json.dumps(report, indent=2, ensure_ascii=False))
    if should_fail:
        sys.exit(1)


if __name__ == "__main__":
    main()
