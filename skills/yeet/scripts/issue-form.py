#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["pyyaml>=6.0"]
# ///
"""Inspect a live GitHub issue form or render validated field-ID answers."""

from __future__ import annotations

import argparse
import base64
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import yaml


class FormError(ValueError):
    pass


def normalize_labels(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [item.strip() for item in value.split(",") if item.strip()]
    if isinstance(value, list) and all(isinstance(item, str) for item in value):
        return list(dict.fromkeys(value))
    raise FormError("top-level labels must be a string or string array")


def load_yaml_text(args: argparse.Namespace) -> str:
    local = args.input or args.fixture
    if local:
        try:
            return local.read_text(encoding="utf-8")
        except OSError as exc:
            raise FormError(str(exc)) from exc
    if not args.repo or not args.template:
        raise FormError("inspect requires --repo and --template unless --input is used")
    if Path(args.template).name != args.template or not args.template.endswith((".yml", ".yaml")):
        raise FormError("--template must be a YAML filename")
    result = subprocess.run(
        ["gh", "api", f"repos/{args.repo}/contents/.github/ISSUE_TEMPLATE/{args.template}"],
        text=True,
        capture_output=True,
    )
    if result.returncode:
        raise FormError(result.stderr.strip() or "gh api failed")
    try:
        response = json.loads(result.stdout)
        content = response["content"]
        return base64.b64decode(content).decode("utf-8")
    except (KeyError, ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise FormError(f"invalid GitHub contents response: {exc}") from exc


def inspect_form(text: str, repo: str | None = None, template: str | None = None) -> dict[str, Any]:
    try:
        document = yaml.safe_load(text)
    except yaml.YAMLError as exc:
        raise FormError(f"invalid YAML: {exc}") from exc
    if not isinstance(document, dict):
        raise FormError("issue form must be a YAML object")
    body = document.get("body")
    if not isinstance(body, list):
        raise FormError("issue form body must be an array")
    for key in ("name", "description", "title", "type"):
        if document.get(key) is not None and not isinstance(document[key], str):
            raise FormError(f"top-level {key} must be a string")
    fields: list[dict[str, Any]] = []
    ids: set[str] = set()
    for index, raw_field in enumerate(body):
        if not isinstance(raw_field, dict):
            raise FormError(f"body item {index} must be an object")
        field_type = raw_field.get("type")
        if field_type == "markdown":
            continue
        if field_type not in {"input", "textarea", "dropdown", "checkboxes", "upload"}:
            raise FormError(f"body item {index} has unsupported type: {field_type}")
        source_id = raw_field.get("id")
        if source_id is not None and (not isinstance(source_id, str) or not source_id):
            raise FormError(f"body item {index} has an invalid id")
        field_id = source_id or f"__field_{index + 1}"
        if field_id in ids:
            raise FormError(f"duplicate field id: {field_id}")
        ids.add(field_id)
        attributes = raw_field.get("attributes") or {}
        validations = raw_field.get("validations") or {}
        if not isinstance(attributes, dict) or not isinstance(validations, dict):
            raise FormError(f"field {field_id} attributes and validations must be objects")
        label = attributes.get("label")
        if not isinstance(label, str) or not label:
            raise FormError(f"field {field_id} must have a label")
        options = attributes.get("options") or []
        normalized_options: list[str] = []
        attestations: list[dict[str, Any]] = []
        if field_type == "dropdown":
            if not isinstance(options, list) or not all(isinstance(option, str) for option in options):
                raise FormError(f"dropdown {field_id} options must be strings")
            normalized_options = options
        elif field_type == "checkboxes":
            if not isinstance(options, list):
                raise FormError(f"checkboxes {field_id} options must be an array")
            for option in options:
                if not isinstance(option, dict) or not isinstance(option.get("label"), str):
                    raise FormError(f"checkboxes {field_id} has an invalid option")
                attestations.append({"label": option["label"], "required": option.get("required") is True})
        render = attributes.get("render")
        if render is not None and (field_type != "textarea" or not isinstance(render, str) or not render):
            raise FormError(f"field {field_id} has an invalid render mode")
        fields.append(
            {
                "id": field_id,
                "sourceId": source_id,
                "type": field_type,
                "label": label,
                "description": attributes.get("description") if isinstance(attributes.get("description"), str) else None,
                "required": validations.get("required") is True,
                "render": render,
                "multiple": field_type == "dropdown" and attributes.get("multiple") is True,
                "options": normalized_options,
                "checkboxAttestations": attestations,
            }
        )
    return {
        "schemaVersion": 1,
        "repository": repo,
        "template": template,
        "name": document.get("name"),
        "description": document.get("description"),
        "titlePrefix": document.get("title") or "",
        "labels": normalize_labels(document.get("labels")),
        "issueType": document.get("type"),
        "fields": fields,
    }


def checkbox_answer(value: Any, field_id: str) -> tuple[list[str], set[str]]:
    if value is None:
        return [], set()
    if isinstance(value, list) and all(isinstance(item, str) for item in value):
        return value, set()
    if not isinstance(value, dict):
        raise FormError(f"answer for {field_id} must be an object or string array")
    if "selected" in value or "verified" in value:
        selected = value.get("selected", value.get("verified", []))
        verified = value.get("verified", [])
        if not isinstance(selected, list) or not all(isinstance(item, str) for item in selected):
            raise FormError(f"selected checkbox values for {field_id} must be strings")
        if not isinstance(verified, list) or not all(isinstance(item, str) for item in verified):
            raise FormError(f"verified checkbox values for {field_id} must be strings")
        return selected, set(verified)
    if not all(isinstance(label, str) and isinstance(verified, bool) for label, verified in value.items()):
        raise FormError(f"checkbox answer map for {field_id} must contain boolean values")
    verified = {label for label, is_verified in value.items() if is_verified}
    return list(verified), verified


def render_form(form: dict[str, Any], answers: dict[str, Any]) -> dict[str, Any]:
    if form.get("schemaVersion") != 1 or not isinstance(form.get("fields"), list):
        raise FormError("form schemaVersion must be 1")
    if not isinstance(answers, dict):
        raise FormError("answers must be an object keyed by field ID")
    known_ids = {field.get("id") for field in form["fields"]}
    unknown = sorted(set(answers) - known_ids)
    if unknown:
        raise FormError(f"answers contain unknown field IDs: {', '.join(unknown)}")
    sections: list[str] = []
    for field in form["fields"]:
        field_id = field["id"]
        value = answers.get(field_id)
        content: str
        if field["type"] in {"input", "textarea", "upload"}:
            if value is None:
                value = ""
            if field["type"] == "upload" and isinstance(value, list) and all(isinstance(item, str) for item in value):
                value = "\n".join(value)
            if not isinstance(value, str):
                raise FormError(f"answer for {field_id} must be a string")
            if field["required"] and not value.strip():
                raise FormError(f"missing required answer: {field_id}")
            if field.get("render") and value:
                content = f"```{field['render']}\n{value.rstrip()}\n```"
            else:
                content = value
        elif field["type"] == "dropdown":
            selected = value if isinstance(value, list) else ([] if value is None else [value])
            if not all(isinstance(item, str) for item in selected):
                raise FormError(f"dropdown answer for {field_id} must contain strings")
            if not field.get("multiple") and len(selected) > 1:
                raise FormError(f"dropdown {field_id} does not allow multiple values")
            invalid = [item for item in selected if item not in field["options"]]
            if invalid:
                raise FormError(f"invalid dropdown value for {field_id}: {', '.join(invalid)}")
            if field["required"] and not selected:
                raise FormError(f"missing required answer: {field_id}")
            content = ", ".join(selected)
        else:
            selected, verified = checkbox_answer(value, field_id)
            labels = [option["label"] for option in field["checkboxAttestations"]]
            invalid = [item for item in selected if item not in labels]
            if invalid:
                raise FormError(f"invalid checkbox value for {field_id}: {', '.join(invalid)}")
            required_labels = [option["label"] for option in field["checkboxAttestations"] if option["required"]]
            unverified = [label for label in required_labels if label not in verified]
            if unverified:
                raise FormError(f"required checkbox attestations are unverified for {field_id}: {', '.join(unverified)}")
            if field["required"] and not selected:
                raise FormError(f"missing required answer: {field_id}")
            content = "\n".join(f"- [{'x' if label in selected else ' '}] {label}" for label in labels)
        if content or field["required"] or field["type"] == "checkboxes":
            sections.append(f"### {field['label']}\n\n{content}".rstrip())
    return {
        "schemaVersion": 1,
        "body": "\n\n".join(sections) + "\n",
        "posting": {
            "titlePrefix": form.get("titlePrefix") or "",
            "labels": form.get("labels") or [],
            "issueType": form.get("issueType"),
        },
    }


def read_json(path: Path, name: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise FormError(f"cannot read {name}: {exc}") from exc
    if not isinstance(value, dict):
        raise FormError(f"{name} must contain a JSON object")
    return value


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    inspect = subparsers.add_parser("inspect")
    inspect.add_argument("--repo")
    inspect.add_argument("--template")
    inspect.add_argument("--input", type=Path)
    inspect.add_argument("--fixture", type=Path, help=argparse.SUPPRESS)
    render = subparsers.add_parser("render")
    render.add_argument("--form", required=True, type=Path)
    render.add_argument("--answers", required=True, type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        if args.command == "inspect":
            result = inspect_form(load_yaml_text(args), args.repo, args.template)
        else:
            result = render_form(read_json(args.form, "form"), read_json(args.answers, "answers"))
    except FormError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 64
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
