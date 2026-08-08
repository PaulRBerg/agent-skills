#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Emit a compact factual profile for a local PDF."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any

REQUIRED_TOOLS = ("pdfimages", "pdfinfo", "pdftotext", "qpdf")


def emit(payload: dict[str, Any], returncode: int) -> int:
    print(json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True))
    return returncode


def fail(code: str, message: str, pdf_path: Path | None = None) -> int:
    payload: dict[str, Any] = {
        "schema_version": 1,
        "status": "error",
        "error": {"code": code, "message": message},
    }
    if pdf_path is not None:
        payload["path"] = str(pdf_path)
    return emit(payload, 1)


def run(args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        capture_output=True,
        check=False,
        encoding="utf-8",
        errors="replace",
        text=True,
    )


def parse_pdfinfo(output: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in output.splitlines():
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        result[key.strip()] = value.strip()
    return result


def parse_geometry(output: str, page_count: int, fallback: dict[str, str]) -> list[dict[str, Any]]:
    pages: dict[int, dict[str, Any]] = {
        page: {"page": page, "width_points": None, "height_points": None, "rotation": 0}
        for page in range(1, page_count + 1)
    }
    size_pattern = re.compile(
        r"^Page\s+(\d+)\s+size:\s+([0-9.]+)\s+x\s+([0-9.]+)\s+pts",
        re.MULTILINE,
    )
    rotation_pattern = re.compile(r"^Page\s+(\d+)\s+rot:\s+(-?\d+)", re.MULTILINE)
    for match in size_pattern.finditer(output):
        page = int(match.group(1))
        if page in pages:
            pages[page]["width_points"] = float(match.group(2))
            pages[page]["height_points"] = float(match.group(3))
    for match in rotation_pattern.finditer(output):
        page = int(match.group(1))
        if page in pages:
            pages[page]["rotation"] = int(match.group(2))

    fallback_size = re.search(r"([0-9.]+)\s+x\s+([0-9.]+)\s+pts", fallback.get("Page size", ""))
    fallback_rotation = int(fallback.get("Page rot", "0") or 0)
    if fallback_size:
        for page in pages.values():
            if page["width_points"] is None:
                page["width_points"] = float(fallback_size.group(1))
                page["height_points"] = float(fallback_size.group(2))
                page["rotation"] = fallback_rotation
    return list(pages.values())


def text_coverage(output: str, page_count: int) -> dict[str, Any]:
    chunks = output.split("\f")
    if len(chunks) > page_count and not chunks[-1].strip():
        chunks.pop()
    chunks.extend([""] * max(0, page_count - len(chunks)))
    counts = [sum(not character.isspace() for character in chunk) for chunk in chunks[:page_count]]
    return {
        "characters": sum(counts),
        "pages_with_text": [index for index, count in enumerate(counts, 1) if count > 0],
        "pages_without_text": [index for index, count in enumerate(counts, 1) if count == 0],
        "per_page": [
            {"page": index, "characters": count} for index, count in enumerate(counts, 1)
        ],
    }


def image_coverage(output: str, page_count: int) -> dict[str, Any]:
    counts: Counter[int] = Counter()
    for line in output.splitlines():
        match = re.match(r"^\s*(\d+)\s+\d+\s+", line)
        if match:
            counts[int(match.group(1))] += 1
    return {
        "total": sum(counts.values()),
        "per_page": [
            {"page": page, "count": counts.get(page, 0)} for page in range(1, page_count + 1)
        ],
    }


def main() -> int:
    if len(sys.argv) != 2:
        return fail("usage", "usage: profile.py <input.pdf>")

    pdf_path = Path(sys.argv[1]).expanduser().resolve()
    if not pdf_path.is_file():
        return fail("missing_file", "input is not a readable file", pdf_path)

    missing = [tool for tool in REQUIRED_TOOLS if shutil.which(tool) is None]
    if missing:
        return fail("missing_tools", f"required tools not found: {', '.join(missing)}", pdf_path)

    basic = run(["pdfinfo", str(pdf_path)])
    if basic.returncode != 0:
        return fail("unreadable_pdf", "pdfinfo could not read the input", pdf_path)
    info = parse_pdfinfo(basic.stdout)
    try:
        page_count = int(info.get("Pages", "0"))
    except ValueError:
        return fail("invalid_metadata", "pdfinfo returned an invalid page count", pdf_path)
    if page_count < 1:
        return fail("invalid_metadata", "PDF has no readable pages", pdf_path)

    encrypted = info.get("Encrypted", "unknown").lower().startswith("yes")
    base: dict[str, Any] = {
        "schema_version": 1,
        "path": str(pdf_path),
        "size_bytes": pdf_path.stat().st_size,
        "pdf_version": info.get("PDF version"),
        "encrypted": encrypted,
        "page_count": page_count,
    }
    if encrypted:
        base.update({"status": "password_required", "integrity": {"status": "not_checked"}})
        return emit(base, 1)

    integrity = run(["qpdf", "--check", str(pdf_path)])
    if integrity.returncode not in (0, 3):
        return fail("integrity_failed", "qpdf rejected the input", pdf_path)

    detailed = run(["pdfinfo", "-f", "1", "-l", str(page_count), "-box", str(pdf_path)])
    if detailed.returncode != 0:
        return fail("metadata_failed", "pdfinfo could not inspect page geometry", pdf_path)
    text_result = run(["pdftotext", "-layout", str(pdf_path), "-"])
    if text_result.returncode != 0:
        return fail("text_failed", "pdftotext could not inspect text coverage", pdf_path)
    images_result = run(["pdfimages", "-list", str(pdf_path)])
    if images_result.returncode != 0:
        return fail("images_failed", "pdfimages could not inspect image coverage", pdf_path)

    base.update(
        {
            "status": "warnings" if integrity.returncode == 3 else "ok",
            "integrity": {
                "status": "warnings" if integrity.returncode == 3 else "ok",
            },
            "pages": parse_geometry(detailed.stdout, page_count, info),
            "images": image_coverage(images_result.stdout, page_count),
            "text": text_coverage(text_result.stdout, page_count),
        }
    )
    return emit(base, 0)


if __name__ == "__main__":
    raise SystemExit(main())
