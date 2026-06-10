#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["openpyxl>=3.1"]
# ///
"""Recalculate all formulas in an .xlsx/.xlsm with headless LibreOffice, then audit it.

Saves the workbook in place and prints a JSON report whose `status` is one of:
success | errors_found | recalc_incomplete | error. Exits non-zero only when
the recalculation itself could not run.

Usage: uv run scripts/recalc.py <workbook> [timeout-seconds]
"""

from __future__ import annotations

import json
import platform
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import NoReturn

EXCEL_ERRORS = {"#VALUE!", "#DIV/0!", "#REF!", "#NAME?", "#NULL!", "#NUM!", "#N/A", "#SPILL!", "#CALC!"}
LIBREOFFICE_ERROR = re.compile(r"Err:\d{3}")
MAX_CELLS_PER_ERROR = 20
DEFAULT_TIMEOUT = 60

MACRO_SUB = "RecalcAndSaveClose"
MACRO_XML = f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE script:module PUBLIC "-//OpenOffice.org//DTD OfficeDocument 1.0//EN" "module.dtd">
<script:module xmlns:script="http://openoffice.org/2000/script" script:name="Module1" script:language="StarBasic">
Sub {MACRO_SUB}()
    ThisComponent.calculateAll()
    ThisComponent.store()
    ThisComponent.close(True)
End Sub
</script:module>
"""


def report(payload: dict, code: int = 0) -> NoReturn:
    print(json.dumps(payload, indent=2))
    sys.exit(code)


def find_soffice() -> str | None:
    on_path = shutil.which("soffice")
    if on_path:
        return on_path
    for candidate in (
        Path("/Applications/LibreOffice.app/Contents/MacOS/soffice"),
        Path.home() / "Applications/LibreOffice.app/Contents/MacOS/soffice",
    ):
        if candidate.is_file():
            return str(candidate)
    return None


def macro_file() -> Path:
    if platform.system() == "Darwin":
        profile = Path.home() / "Library/Application Support/LibreOffice/4/user"
    else:
        profile = Path.home() / ".config/libreoffice/4/user"
    return profile / "basic/Standard/Module1.xba"


def ensure_macro(soffice: str) -> None:
    target = macro_file()
    if target.is_file() and MACRO_SUB in target.read_text():
        return
    if not target.parent.is_dir():
        # First run: let LibreOffice create its user profile.
        subprocess.run([soffice, "--headless", "--terminate_after_init"], capture_output=True, timeout=60)
        target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(MACRO_XML)


def recalculate(soffice: str, workbook: Path, timeout: int) -> None:
    uri = f"vnd.sun.star.script:Standard.Module1.{MACRO_SUB}?language=Basic&location=application"
    subprocess.run(
        [soffice, "--headless", "--norestore", uri, str(workbook)],
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def audit(workbook: Path) -> dict:
    from openpyxl import load_workbook

    with_values = load_workbook(workbook, data_only=True)
    with_formulas = load_workbook(workbook, data_only=False)

    errors: dict[str, list[str]] = defaultdict(list)
    total_errors = 0
    total_formulas = 0
    uncached = 0

    for name in with_formulas.sheetnames:
        formula_ws = with_formulas[name]
        value_ws = with_values[name]
        for row in formula_ws.iter_rows():
            for cell in row:
                if isinstance(cell.value, str) and cell.value.startswith("="):
                    total_formulas += 1
                    if value_ws[cell.coordinate].value is None:
                        uncached += 1
        for row in value_ws.iter_rows():
            for cell in row:
                if isinstance(cell.value, str):
                    token = cell.value.strip()
                    if token in EXCEL_ERRORS or LIBREOFFICE_ERROR.fullmatch(token):
                        errors[token].append(f"{name}!{cell.coordinate}")
                        total_errors += 1
    with_values.close()
    with_formulas.close()

    if total_errors:
        status = "errors_found"
    elif total_formulas and uncached == total_formulas:
        status = "recalc_incomplete"
    else:
        status = "success"

    return {
        "file": str(workbook),
        "status": status,
        "total_formulas": total_formulas,
        "uncached_formulas": uncached,
        "total_errors": total_errors,
        "errors": {
            kind: {"count": len(cells), "cells": cells[:MAX_CELLS_PER_ERROR]}
            for kind, cells in sorted(errors.items())
        },
    }


def main() -> None:
    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        print(__doc__.strip())
        sys.exit(1 if len(sys.argv) < 2 else 0)

    workbook = Path(sys.argv[1]).resolve()
    timeout = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_TIMEOUT
    if not workbook.is_file():
        report({"status": "error", "error": f"{workbook} does not exist"}, code=1)

    soffice = find_soffice()
    if soffice is None:
        report(
            {
                "status": "error",
                "error": "LibreOffice not found: no soffice on PATH and no LibreOffice.app in /Applications or ~/Applications",
                "hint": "brew install --cask libreoffice",
            },
            code=1,
        )

    try:
        ensure_macro(soffice)
        recalculate(soffice, workbook, timeout)
    except subprocess.TimeoutExpired:
        report(
            {
                "status": "error",
                "error": f"LibreOffice timed out after {timeout}s",
                "hint": f"rerun with a larger timeout: uv run scripts/recalc.py {workbook.name} {timeout * 3}",
            },
            code=1,
        )

    result = audit(workbook)
    if result["status"] == "recalc_incomplete":
        result["hint"] = "no cached values were written; quit any running LibreOffice instance and rerun"
    report(result)


if __name__ == "__main__":
    main()
