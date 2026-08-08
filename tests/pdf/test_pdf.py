#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow>=12,<13", "pypdf>=6.15,<7", "reportlab>=5,<6"]
# ///
"""Smoke tests for the PDF skill helpers and macOS OCR route."""

from __future__ import annotations

import json
import shlex
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFont
from pypdf import PdfReader
from reportlab.lib.pagesizes import A4
from reportlab.pdfgen import canvas

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "skills" / "pdf" / "scripts"
PROFILE = SCRIPTS / "profile.py"
FORM = SCRIPTS / "form.py"
ARIAL = Path("/System/Library/Fonts/Supplemental/Arial.ttf")
REQUIRED_TOOLS = ("ocrmypdf", "pdfimages", "pdfinfo", "pdftocairo", "pdftotext", "qpdf", "tesseract")


def run(args: list[str], check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(args, capture_output=True, check=False, text=True)
    if check and result.returncode != 0:
        raise AssertionError(
            f"command failed: {shlex.join(args)}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def run_json(args: list[str], check: bool = True) -> tuple[subprocess.CompletedProcess[str], dict]:
    result = run(args, check=check)
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise AssertionError(f"command did not emit JSON: {shlex.join(args)}\n{result.stdout}") from error
    return result, payload


def require_tools() -> None:
    missing = [tool for tool in REQUIRED_TOOLS if shutil.which(tool) is None]
    if missing:
        raise AssertionError(f"required PDF tools not found: {', '.join(missing)}")
    if not ARIAL.is_file():
        raise AssertionError(f"required macOS font not found: {ARIAL}")


def make_digital(path: Path) -> None:
    document = canvas.Canvas(str(path), pagesize=A4)
    document.drawString(72, 780, "Statement page 1")
    document.drawString(72, 756, "Opening balance 100.00")
    document.drawString(72, 732, "Credit 25.50")
    document.showPage()
    document.drawString(72, 780, "Statement page 2")
    document.drawString(72, 756, "Closing balance 125.50")
    document.save()


def make_scan(path: Path) -> None:
    image = Image.new("RGB", (1654, 2339), "white")
    font = ImageFont.truetype(str(ARIAL), 72)
    drawing = ImageDraw.Draw(image)
    drawing.text((140, 320), "OCR TEST 12345", fill="black", font=font)
    drawing.text((140, 450), "Romanian English scan", fill="black", font=font)
    image.save(path, "PDF", resolution=200)


def make_acroform(path: Path) -> None:
    document = canvas.Canvas(str(path), pagesize=A4)
    document.drawString(72, 790, "Name")
    document.acroForm.textfield(
        name="person.name",
        x=120,
        y=775,
        width=300,
        height=24,
        borderStyle="underlined",
        forceBorder=True,
    )
    document.save()


def make_flat_form(path: Path) -> None:
    document = canvas.Canvas(str(path), pagesize=A4)
    document.drawString(72, 790, "City")
    document.line(120, 775, 360, 775)
    document.save()


def render_first_page(path: Path, prefix: Path) -> Path:
    run(["pdftocairo", "-f", "1", "-l", "1", "-singlefile", "-png", "-r", "144", str(path), str(prefix)])
    rendered = prefix.with_suffix(".png")
    assert rendered.is_file()
    return rendered


def assert_render_changed(before: Path, after: Path, tmp: Path, label: str) -> None:
    before_image = Image.open(render_first_page(before, tmp / f"{label}-before")).convert("RGB")
    after_image = Image.open(render_first_page(after, tmp / f"{label}-after")).convert("RGB")
    assert before_image.size == after_image.size
    assert ImageChops.difference(before_image, after_image).getbbox() is not None


def test_profile(tmp: Path) -> tuple[Path, Path]:
    digital = tmp / "digital statement with spaces.pdf"
    scan = tmp / "image-only scan.pdf"
    make_digital(digital)
    make_scan(scan)

    _, digital_payload = run_json([sys.executable, str(PROFILE), str(digital)])
    assert digital_payload["schema_version"] == 1
    assert digital_payload["status"] == "ok"
    assert digital_payload["page_count"] == 2
    assert digital_payload["text"]["pages_with_text"] == [1, 2]
    assert digital_payload["text"]["pages_without_text"] == []
    assert all(page["width_points"] and page["height_points"] for page in digital_payload["pages"])

    _, scan_payload = run_json([sys.executable, str(PROFILE), str(scan)])
    assert scan_payload["status"] == "ok"
    assert scan_payload["page_count"] == 1
    assert scan_payload["text"]["pages_without_text"] == [1]
    assert scan_payload["images"]["total"] >= 1

    corrupt = tmp / "corrupt.pdf"
    corrupt.write_text("not a PDF", encoding="utf-8")
    result, corrupt_payload = run_json([sys.executable, str(PROFILE), str(corrupt)], check=False)
    assert result.returncode == 1
    assert corrupt_payload["status"] == "error"
    assert corrupt_payload["error"]["code"] == "unreadable_pdf"
    return digital, scan


def test_forms(tmp: Path) -> None:
    source = tmp / "source form.pdf"
    flat = tmp / "flat form.pdf"
    make_acroform(source)
    make_flat_form(flat)

    _, inspection = run_json([sys.executable, str(FORM), "inspect", str(source)])
    assert inspection["acroform"] is True
    assert inspection["xfa"] is False
    assert inspection["fields"][0]["name"] == "person.name"
    assert inspection["fields"][0]["page"] == 1

    values = tmp / "values.json"
    values.write_text(json.dumps({"person.name": "Ada Lovelace"}), encoding="utf-8")
    filled = tmp / "filled.pdf"
    _, filled_payload = run_json(
        [sys.executable, str(FORM), "fill", str(source), str(values), str(filled)]
    )
    assert filled_payload["fields_written"] == ["person.name"]
    _, filled_inspection = run_json([sys.executable, str(FORM), "inspect", str(filled)])
    assert filled_inspection["fields"][0]["value"] == "Ada Lovelace"

    unknown_values = tmp / "unknown.json"
    unknown_values.write_text(json.dumps({"missing": "value"}), encoding="utf-8")
    unknown_output = tmp / "unknown-output.pdf"
    result, payload = run_json(
        [sys.executable, str(FORM), "fill", str(source), str(unknown_values), str(unknown_output)],
        check=False,
    )
    assert result.returncode == 1
    assert payload["error"]["code"] == "unknown_fields"
    assert not unknown_output.exists()

    original_filled = filled.read_bytes()
    result, payload = run_json(
        [sys.executable, str(FORM), "fill", str(source), str(values), str(filled)], check=False
    )
    assert result.returncode == 1
    assert payload["error"]["code"] == "output_exists"
    assert filled.read_bytes() == original_filled

    flattened = tmp / "flattened.pdf"
    run_json(
        [sys.executable, str(FORM), "fill", str(source), str(values), str(flattened), "--flatten"]
    )
    flattened_reader = PdfReader(str(flattened))
    flattened_root = flattened_reader.trailer["/Root"]
    assert "/AcroForm" not in flattened_root
    for page in flattened_reader.pages:
        assert all(
            str(reference.get_object().get("/Subtype")) != "/Widget"
            for reference in page.get("/Annots", [])
        )
    assert_render_changed(source, flattened, tmp, "flattened")

    placements = tmp / "placements.json"
    placements.write_text(
        json.dumps([{"page": 1, "x": 126, "y": 780, "text": "București", "font_size": 12}]),
        encoding="utf-8",
    )
    overlaid = tmp / "overlaid.pdf"
    _, overlay_payload = run_json(
        [sys.executable, str(FORM), "overlay", str(flat), str(placements), str(overlaid)]
    )
    assert overlay_payload["placements_written"] == 1
    source_reader = PdfReader(str(flat))
    overlay_reader = PdfReader(str(overlaid))
    assert len(source_reader.pages) == len(overlay_reader.pages) == 1
    assert float(source_reader.pages[0].mediabox.width) == float(overlay_reader.pages[0].mediabox.width)
    assert float(source_reader.pages[0].mediabox.height) == float(overlay_reader.pages[0].mediabox.height)
    assert "Bucure" in (overlay_reader.pages[0].extract_text() or "")
    assert_render_changed(flat, overlaid, tmp, "overlay")
    assert not list(tmp.glob(".*.tmp.pdf"))


def test_ocr(tmp: Path, scan: Path) -> None:
    languages = run(["tesseract", "--list-langs"]).stdout.splitlines()
    assert "ron" in languages
    output = tmp / "ocr.pdf"
    sidecar = tmp / "ocr.txt"
    run(
        [
            "ocrmypdf",
            "--output-type",
            "pdf",
            "--skip-text",
            "-l",
            "eng+ron",
            "--sidecar",
            str(sidecar),
            str(scan),
            str(output),
        ]
    )
    run(["qpdf", "--check", str(output)])
    extracted = run(["pdftotext", "-layout", str(output), "-"]).stdout
    assert "12345" in extracted
    assert "12345" in sidecar.read_text(encoding="utf-8")


def main() -> None:
    require_tools()
    with tempfile.TemporaryDirectory(prefix="pdf-skill-tests-") as temporary_directory:
        tmp = Path(temporary_directory)
        _, scan = test_profile(tmp)
        test_forms(tmp)
        test_ocr(tmp, scan)
    print("PDF helper tests passed")


if __name__ == "__main__":
    main()
