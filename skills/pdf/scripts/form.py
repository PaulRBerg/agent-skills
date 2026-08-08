#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["pypdf>=6.15,<7", "reportlab>=5,<6"]
# ///
"""Inspect, fill, flatten, and overlay local PDF forms."""

from __future__ import annotations

import argparse
import io
import json
import os
import shutil
import subprocess
import tempfile
from collections import defaultdict
from pathlib import Path
from typing import Any

from pypdf import PdfReader, PdfWriter
from pypdf.errors import PdfReadError
from pypdf.generic import ArrayObject, NameObject
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas

SCHEMA_VERSION = 1
DEFAULT_FONT = Path("/System/Library/Fonts/Supplemental/Arial.ttf")
FONT_NAME = "PDFSkillArial"


class UserError(Exception):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def emit(payload: dict[str, Any], returncode: int = 0) -> int:
    print(json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True))
    return returncode


def pdf_object(value: Any) -> Any:
    if hasattr(value, "get_object"):
        value = value.get_object()
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, dict):
        return {str(key): pdf_object(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [pdf_object(item) for item in value]
    return str(value)


def resolved_dictionary(value: Any) -> Any:
    return value.get_object() if hasattr(value, "get_object") else value


def acroform(reader: PdfReader) -> Any | None:
    root = resolved_dictionary(reader.trailer["/Root"])
    value = root.get("/AcroForm")
    return resolved_dictionary(value) if value is not None else None


def ensure_readable(reader: PdfReader) -> None:
    if reader.is_encrypted:
        raise UserError("password_required", "encrypted PDFs are not supported")


def qualified_field_name(annotation: Any) -> str | None:
    parts: list[str] = []
    node = resolved_dictionary(annotation)
    seen: set[int] = set()
    while node is not None and id(node) not in seen:
        seen.add(id(node))
        if node.get("/T") is not None:
            parts.append(str(node["/T"]))
        parent = node.get("/Parent")
        node = resolved_dictionary(parent) if parent is not None else None
    return ".".join(reversed(parts)) or None


def widget_locations(reader: PdfReader) -> dict[str, list[dict[str, Any]]]:
    locations: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for page_number, page in enumerate(reader.pages, 1):
        for reference in page.get("/Annots", []):
            annotation = resolved_dictionary(reference)
            if str(annotation.get("/Subtype")) != "/Widget":
                continue
            name = qualified_field_name(annotation)
            if name is None:
                continue
            rectangle = annotation.get("/Rect")
            locations[name].append(
                {
                    "page": page_number,
                    "rectangle": [float(value) for value in rectangle] if rectangle else None,
                }
            )
    return locations


def inspect_payload(input_path: Path) -> dict[str, Any]:
    reader = PdfReader(str(input_path))
    ensure_readable(reader)
    form = acroform(reader)
    fields = reader.get_fields() or {}
    locations = widget_locations(reader)
    items: list[dict[str, Any]] = []
    for name in sorted(fields):
        field = resolved_dictionary(fields[name])
        location = locations.get(name, [{}])[0]
        items.append(
            {
                "name": name,
                "type": str(field.get("/FT")) if field.get("/FT") is not None else None,
                "value": pdf_object(field.get("/V")),
                "default_value": pdf_object(field.get("/DV")),
                "options": pdf_object(field.get("/Opt")),
                "flags": int(field.get("/Ff", 0)),
                "page": location.get("page"),
                "rectangle": location.get("rectangle"),
            }
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "status": "ok",
        "operation": "inspect",
        "input": str(input_path),
        "page_count": len(reader.pages),
        "acroform": form is not None,
        "xfa": bool(form is not None and form.get("/XFA") is not None),
        "fields": items,
    }


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise UserError("missing_json", "JSON input does not exist") from error
    except json.JSONDecodeError as error:
        raise UserError("invalid_json", f"invalid JSON at line {error.lineno}") from error


def prepare_paths(input_value: str, output_value: str, force: bool) -> tuple[Path, Path]:
    input_path = Path(input_value).expanduser().resolve()
    output_path = Path(output_value).expanduser().resolve()
    if not input_path.is_file():
        raise UserError("missing_input", "input PDF does not exist")
    if input_path == output_path:
        raise UserError("input_output_alias", "input and output must be different paths")
    if output_path.exists() and not force:
        raise UserError("output_exists", "output exists; pass --force to replace it")
    if not output_path.parent.is_dir():
        raise UserError("missing_output_directory", "output directory does not exist")
    return input_path, output_path


def validate_writable_form(reader: PdfReader) -> dict[str, Any]:
    ensure_readable(reader)
    form = acroform(reader)
    fields = reader.get_fields() or {}
    if form is None or not fields:
        raise UserError("no_acroform", "PDF has no fillable AcroForm fields")
    if form.get("/XFA") is not None:
        raise UserError("unsupported_xfa", "XFA forms are not supported")
    if any(str(resolved_dictionary(field).get("/FT")) == "/Sig" for field in fields.values()):
        raise UserError("unsupported_signature", "signature fields are not supported")
    return fields


def atomic_write(writer: PdfWriter, output_path: Path) -> None:
    if shutil.which("qpdf") is None:
        raise UserError("missing_qpdf", "qpdf is required to validate written PDFs")
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{output_path.name}.", suffix=".tmp.pdf", dir=output_path.parent
    )
    os.close(descriptor)
    temporary_path = Path(temporary_name)
    try:
        writer.write(str(temporary_path))
        check = subprocess.run(
            ["qpdf", "--check", str(temporary_path)],
            capture_output=True,
            check=False,
            text=True,
        )
        if check.returncode not in (0, 3):
            raise UserError("integrity_failed", "qpdf rejected the generated PDF")
        os.replace(temporary_path, output_path)
    finally:
        temporary_path.unlink(missing_ok=True)


def command_inspect(args: argparse.Namespace) -> int:
    input_path = Path(args.input).expanduser().resolve()
    if not input_path.is_file():
        raise UserError("missing_input", "input PDF does not exist")
    return emit(inspect_payload(input_path))


def command_fill(args: argparse.Namespace) -> int:
    input_path, output_path = prepare_paths(args.input, args.output, args.force)
    values_path = Path(args.values).expanduser().resolve()
    values = load_json(values_path)
    if not isinstance(values, dict) or not all(isinstance(key, str) for key in values):
        raise UserError("invalid_values", "values JSON must be an object keyed by field name")
    for value in values.values():
        if isinstance(value, str):
            continue
        if isinstance(value, list) and all(isinstance(item, str) for item in value):
            continue
        raise UserError("invalid_values", "field values must be strings or lists of strings")

    reader = PdfReader(str(input_path))
    fields = validate_writable_form(reader)
    unknown = sorted(set(values) - set(fields))
    if unknown:
        raise UserError("unknown_fields", f"unknown field names: {', '.join(unknown)}")

    writer = PdfWriter(clone_from=str(input_path))
    for page in writer.pages:
        writer.update_page_form_field_values(
            page,
            values,
            auto_regenerate=False,
            flatten=args.flatten,
        )
    if args.flatten:
        for page in writer.pages:
            annotations = page.get("/Annots")
            if annotations is None:
                continue
            retained = ArrayObject(
                [
                    reference
                    for reference in annotations
                    if str(resolved_dictionary(reference).get("/Subtype")) != "/Widget"
                ]
            )
            if retained:
                page[NameObject("/Annots")] = retained
            else:
                page.pop(NameObject("/Annots"), None)
        writer.root_object.pop(NameObject("/AcroForm"), None)
    atomic_write(writer, output_path)
    return emit(
        {
            "schema_version": SCHEMA_VERSION,
            "status": "ok",
            "operation": "fill",
            "output": str(output_path),
            "fields_written": sorted(values),
            "flattened": args.flatten,
        }
    )


def positive_number(value: Any, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise UserError("invalid_placements", f"{name} must be a number")
    result = float(value)
    if name == "font_size" and result <= 0:
        raise UserError("invalid_placements", "font_size must be greater than zero")
    return result


def command_overlay(args: argparse.Namespace) -> int:
    input_path, output_path = prepare_paths(args.input, args.output, args.force)
    placements = load_json(Path(args.placements).expanduser().resolve())
    if not isinstance(placements, list):
        raise UserError("invalid_placements", "placements JSON must be an array")
    reader = PdfReader(str(input_path))
    ensure_readable(reader)
    grouped: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for index, placement in enumerate(placements):
        if not isinstance(placement, dict):
            raise UserError("invalid_placements", f"placement {index} must be an object")
        page = placement.get("page")
        text = placement.get("text")
        if isinstance(page, bool) or not isinstance(page, int) or not 1 <= page <= len(reader.pages):
            raise UserError("invalid_placements", f"placement {index} has an invalid page")
        if not isinstance(text, str) or "\n" in text or "\r" in text:
            raise UserError("invalid_placements", f"placement {index} text must be one line")
        grouped[page].append(
            {
                "x": positive_number(placement.get("x"), "x"),
                "y": positive_number(placement.get("y"), "y"),
                "text": text,
                "font_size": positive_number(placement.get("font_size", 10), "font_size"),
            }
        )

    if not DEFAULT_FONT.is_file():
        raise UserError("missing_font", f"default font not found: {DEFAULT_FONT}")
    pdfmetrics.registerFont(TTFont(FONT_NAME, str(DEFAULT_FONT)))
    writer = PdfWriter(clone_from=str(input_path))
    input_dimensions = [
        (float(page.mediabox.width), float(page.mediabox.height)) for page in reader.pages
    ]
    for page_number, page_placements in grouped.items():
        width, height = input_dimensions[page_number - 1]
        buffer = io.BytesIO()
        overlay_canvas = canvas.Canvas(buffer, pagesize=(width, height))
        for placement in page_placements:
            overlay_canvas.setFont(FONT_NAME, placement["font_size"])
            overlay_canvas.drawString(placement["x"], placement["y"], placement["text"])
        overlay_canvas.save()
        buffer.seek(0)
        overlay_page = PdfReader(buffer).pages[0]
        writer.pages[page_number - 1].merge_page(overlay_page)

    output_dimensions = [
        (float(page.mediabox.width), float(page.mediabox.height)) for page in writer.pages
    ]
    if output_dimensions != input_dimensions:
        raise UserError("geometry_changed", "overlay changed page dimensions")
    atomic_write(writer, output_path)
    return emit(
        {
            "schema_version": SCHEMA_VERSION,
            "status": "ok",
            "operation": "overlay",
            "output": str(output_path),
            "placements_written": len(placements),
            "pages_affected": sorted(grouped),
        }
    )


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)

    inspect_parser = commands.add_parser("inspect")
    inspect_parser.add_argument("input")
    inspect_parser.set_defaults(handler=command_inspect)

    fill_parser = commands.add_parser("fill")
    fill_parser.add_argument("input")
    fill_parser.add_argument("values")
    fill_parser.add_argument("output")
    fill_parser.add_argument("--flatten", action="store_true")
    fill_parser.add_argument("--force", action="store_true")
    fill_parser.set_defaults(handler=command_fill)

    overlay_parser = commands.add_parser("overlay")
    overlay_parser.add_argument("input")
    overlay_parser.add_argument("placements")
    overlay_parser.add_argument("output")
    overlay_parser.add_argument("--force", action="store_true")
    overlay_parser.set_defaults(handler=command_overlay)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        return args.handler(args)
    except UserError as error:
        return emit(
            {
                "schema_version": SCHEMA_VERSION,
                "status": "error",
                "error": {"code": error.code, "message": str(error)},
            },
            1,
        )
    except (KeyError, OSError, PdfReadError, TypeError, ValueError) as error:
        return emit(
            {
                "schema_version": SCHEMA_VERSION,
                "status": "error",
                "error": {"code": "unexpected_error", "message": str(error)},
            },
            1,
        )


if __name__ == "__main__":
    raise SystemExit(main())
