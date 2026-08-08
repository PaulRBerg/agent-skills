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
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

import yaml


class FormError(ValueError):
    pass


FIELD_ID_RE = re.compile(r"^[A-Za-z0-9_-]+$")
PROJECT_RE = re.compile(r"^[A-Za-z0-9_.-]+/[1-9][0-9]*$")
RENDER_RE = re.compile(r"^[A-Za-z0-9_+.-]+$")


def normalize_metadata(value: Any, name: str) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        values = value.split(",")
    elif isinstance(value, list) and all(isinstance(item, str) for item in value):
        values = value
    else:
        raise FormError(f"top-level {name} must be a string or string array")
    return list(dict.fromkeys(item.strip() for item in values if item.strip()))


def normalize_labels(value: Any) -> list[str]:
    return normalize_metadata(value, "labels")


def normalize_projects(value: Any) -> list[str]:
    projects = normalize_metadata(value, "projects")
    invalid = [project for project in projects if not PROJECT_RE.fullmatch(project)]
    if invalid:
        raise FormError(f"top-level projects must use OWNER/NUMBER: {', '.join(invalid)}")
    return projects


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
    try:
        result = subprocess.run(
            ["gh", "api", f"repos/{args.repo}/contents/.github/ISSUE_TEMPLATE/{args.template}"],
            text=True,
            capture_output=True,
        )
    except OSError as exc:
        if isinstance(exc, FileNotFoundError):
            raise FormError("gh executable not found") from exc
        raise FormError(f"unable to run gh: {exc}") from exc
    if result.returncode:
        raise FormError(result.stderr.strip() or "gh api failed")
    try:
        response = json.loads(result.stdout)
    except (TypeError, ValueError) as exc:
        raise FormError(f"invalid GitHub contents response: {exc}") from exc
    if not isinstance(response, dict):
        raise FormError("invalid GitHub contents response: expected an object")
    if response.get("type") not in (None, "file"):
        raise FormError("invalid GitHub contents response: type must be file")
    if response.get("encoding") not in (None, "base64"):
        raise FormError("invalid GitHub contents response: encoding must be base64")
    content = response.get("content")
    if not isinstance(content, str):
        raise FormError("invalid GitHub contents response: content must be a string")
    try:
        # GitHub commonly includes a trailing newline in this field. Whitespace is
        # harmless in base64, but other malformed characters must be rejected.
        encoded = "".join(content.split())
        decoded = base64.b64decode(encoded, validate=True)
        return decoded.decode("utf-8")
    except (TypeError, ValueError, UnicodeDecodeError, UnicodeEncodeError) as exc:
        raise FormError(f"invalid GitHub contents response: invalid base64 content ({exc})") from exc


def _mapping(raw_field: dict[str, Any], key: str, field_id: str) -> dict[str, Any]:
    if key not in raw_field:
        return {}
    value = raw_field[key]
    if not isinstance(value, dict):
        raise FormError(f"field {field_id} {key} must be an object")
    return value


def _optional_bool(mapping: dict[str, Any], key: str, description: str) -> bool:
    if key not in mapping:
        return False
    value = mapping[key]
    if type(value) is not bool:
        raise FormError(f"{description} {key} must be a boolean")
    return value


def _validate_description(attributes: dict[str, Any], field_id: str) -> str | None:
    if "description" not in attributes:
        return None
    description = attributes["description"]
    if not isinstance(description, str):
        raise FormError(f"field {field_id} description must be a string")
    return description


def inspect_form(text: str, repo: str | None = None, template: str | None = None) -> dict[str, Any]:
    try:
        document = yaml.safe_load(text)
    except (TypeError, yaml.YAMLError) as exc:
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
            if "id" in raw_field:
                raise FormError(f"body item {index} markdown fields cannot have an id")
            attributes = _mapping(raw_field, "attributes", f"__markdown_{index + 1}")
            if "value" in attributes and not isinstance(attributes["value"], str):
                raise FormError(f"markdown field {index} value must be a string")
            continue
        if not isinstance(field_type, str) or field_type not in {"input", "textarea", "dropdown", "checkboxes", "upload"}:
            raise FormError(f"body item {index} has unsupported type: {field_type}")
        source_id = raw_field.get("id")
        if source_id is not None and (
            not isinstance(source_id, str) or not source_id or not FIELD_ID_RE.fullmatch(source_id)
        ):
            raise FormError(f"body item {index} has an invalid id")
        field_id = source_id or f"__field_{index + 1}"
        if field_id in ids:
            raise FormError(f"duplicate field id: {field_id}")
        ids.add(field_id)
        attributes = _mapping(raw_field, "attributes", field_id)
        validations = _mapping(raw_field, "validations", field_id)
        label = attributes.get("label")
        if not isinstance(label, str) or not label.strip() or "\n" in label or "\r" in label:
            raise FormError(f"field {field_id} must have a label")
        description = _validate_description(attributes, field_id)
        if "required" in validations and type(validations["required"]) is not bool:
            raise FormError(f"field {field_id} required must be a boolean")
        required = validations.get("required", False)
        if "multiple" in attributes and field_type != "dropdown":
            raise FormError(f"field {field_id} multiple is only valid for dropdowns")
        multiple = _optional_bool(attributes, "multiple", f"field {field_id}") if field_type == "dropdown" else False
        options = attributes.get("options")
        normalized_options: list[str] = []
        attestations: list[dict[str, Any]] = []
        default: int | None = None
        if field_type == "dropdown":
            if not isinstance(options, list) or not options or not all(
                isinstance(option, str) and option.strip() for option in options
            ):
                raise FormError(f"dropdown {field_id} options must be strings")
            if len(set(options)) != len(options):
                raise FormError(f"dropdown {field_id} options must be distinct")
            normalized_options = options
            if "default" in attributes:
                value = attributes["default"]
                if type(value) is not int:
                    raise FormError(f"dropdown {field_id} default must be an integer index")
                if value < 0 or value >= len(options):
                    raise FormError(f"dropdown {field_id} default index is out of bounds")
                default = value
        elif "default" in attributes:
            raise FormError(f"field {field_id} default is only valid for dropdowns")
        elif field_type == "checkboxes":
            if not isinstance(options, list) or not options:
                raise FormError(f"checkboxes {field_id} options must be an array")
            seen_labels: set[str] = set()
            for option in options:
                if not isinstance(option, dict) or not isinstance(option.get("label"), str):
                    raise FormError(f"checkboxes {field_id} has an invalid option")
                option_label = option["label"]
                if not option_label.strip() or option_label in seen_labels:
                    raise FormError(f"checkboxes {field_id} has duplicate or empty labels")
                seen_labels.add(option_label)
                option_required = _optional_bool(option, "required", f"checkbox {field_id} option")
                attestations.append({"label": option_label, "required": option_required})
        elif options is not None:
            raise FormError(f"field {field_id} options are only valid for dropdowns and checkboxes")
        if field_type in {"input", "textarea"} and "value" in attributes and not isinstance(attributes["value"], str):
            raise FormError(f"field {field_id} value must be a string")
        if "render" in attributes and field_type != "textarea":
            raise FormError(f"field {field_id} has an invalid render mode")
        render = attributes.get("render")
        if render is not None and (
            field_type != "textarea" or not isinstance(render, str) or not RENDER_RE.fullmatch(render)
        ):
            raise FormError(f"field {field_id} has an invalid render mode")
        if "accept" in validations and field_type != "upload":
            raise FormError(f"field {field_id} accept is only valid for upload fields")
        accept = validations.get("accept")
        if field_type == "upload" and "accept" in validations and (
            not isinstance(accept, str) or not accept.strip()
        ):
            raise FormError(f"upload {field_id} accept must be a non-empty string")
        fields.append(
            {
                "id": field_id,
                "sourceId": source_id,
                "type": field_type,
                "label": label,
                "description": description,
                "required": required,
                "render": render,
                "multiple": multiple,
                "options": normalized_options,
                "default": default,
                "value": attributes.get("value") if field_type in {"input", "textarea"} else None,
                "accept": accept if field_type == "upload" else None,
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
        "assignees": normalize_metadata(document.get("assignees"), "assignees"),
        "projects": normalize_projects(document.get("projects")),
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


def _validate_render_form(form: Any) -> list[dict[str, Any]]:
    if not isinstance(form, dict):
        raise FormError("form must be an object")
    if type(form.get("schemaVersion")) is not int or form.get("schemaVersion") != 1:
        raise FormError("form schemaVersion must be 1")
    fields = form.get("fields")
    if not isinstance(fields, list):
        raise FormError("form fields must be an array")
    for name in ("repository", "template", "name", "description", "titlePrefix", "issueType"):
        value = form.get(name)
        if value is not None and not isinstance(value, str):
            raise FormError(f"form {name} must be a string")
    ids: set[str] = set()
    for index, field in enumerate(fields):
        if not isinstance(field, dict):
            raise FormError(f"form field {index} must be an object")
        field_id = field.get("id")
        if not isinstance(field_id, str) or not field_id or not FIELD_ID_RE.fullmatch(field_id):
            raise FormError(f"form field {index} has an invalid id")
        if field_id in ids:
            raise FormError(f"duplicate field id: {field_id}")
        ids.add(field_id)
        source_id = field.get("sourceId")
        if source_id is not None and (not isinstance(source_id, str) or not FIELD_ID_RE.fullmatch(source_id)):
            raise FormError(f"form field {field_id} sourceId is invalid")
        field_type = field.get("type")
        if not isinstance(field_type, str) or field_type not in {"input", "textarea", "dropdown", "checkboxes", "upload"}:
            raise FormError(f"form field {field_id} has an unsupported type")
        if (
            not isinstance(field.get("label"), str)
            or not field["label"].strip()
            or "\n" in field["label"]
            or "\r" in field["label"]
        ):
            raise FormError(f"form field {field_id} must have a label")
        if type(field.get("required")) is not bool:
            raise FormError(f"form field {field_id} required must be a boolean")
        description = field.get("description")
        if description is not None and not isinstance(description, str):
            raise FormError(f"form field {field_id} description must be a string")
        render = field.get("render")
        if render is not None and (
            field_type != "textarea" or not isinstance(render, str) or not RENDER_RE.fullmatch(render)
        ):
            raise FormError(f"form field {field_id} has an invalid render mode")
        multiple = field.get("multiple")
        if type(multiple) is not bool:
            raise FormError(f"form field {field_id} multiple must be a boolean")
        options = field.get("options")
        if not isinstance(options, list) or not all(isinstance(option, str) for option in options):
            raise FormError(f"form field {field_id} options must be an array of strings")
        if field_type == "dropdown":
            if not options or len(set(options)) != len(options) or any(not option.strip() for option in options):
                raise FormError(f"dropdown {field_id} options must be non-empty and distinct")
            default = field.get("default")
            if default is not None:
                if type(default) is not int:
                    raise FormError(f"dropdown {field_id} default must be an integer index")
                if default < 0 or default >= len(options):
                    raise FormError(f"dropdown {field_id} default index is out of bounds")
        elif field.get("default") is not None:
            raise FormError(f"form field {field_id} has an invalid default")
        if field_type in {"input", "textarea"} and field.get("value") is not None and not isinstance(field["value"], str):
            raise FormError(f"form field {field_id} value must be a string")
        if field_type == "upload":
            accept = field.get("accept")
            if accept is not None and (not isinstance(accept, str) or not accept.strip()):
                raise FormError(f"upload {field_id} accept must be a non-empty string")
        elif field.get("accept") is not None:
            raise FormError(f"form field {field_id} has an invalid accept value")
        attestations = field.get("checkboxAttestations")
        if not isinstance(attestations, list):
            raise FormError(f"form field {field_id} checkbox attestations must be an array")
        seen_labels: set[str] = set()
        for option in attestations:
            if not isinstance(option, dict) or not isinstance(option.get("label"), str) or not option["label"].strip():
                raise FormError(f"checkboxes {field_id} has an invalid option")
            if option["label"] in seen_labels:
                raise FormError(f"checkboxes {field_id} has duplicate labels")
            seen_labels.add(option["label"])
            if type(option.get("required")) is not bool:
                raise FormError(f"checkbox {field_id} option required must be a boolean")
        if field_type == "checkboxes" and not attestations:
            raise FormError(f"checkboxes {field_id} options must be non-empty")
        if field_type != "checkboxes" and attestations:
            raise FormError(f"form field {field_id} has unexpected checkbox attestations")
    for name in ("labels", "assignees", "projects"):
        value = form.get(name)
        if value is not None and (not isinstance(value, list) or not all(isinstance(item, str) for item in value)):
            raise FormError(f"form {name} must be a string array")
    invalid_projects = [project for project in form.get("projects", []) if not PROJECT_RE.fullmatch(project)]
    if invalid_projects:
        raise FormError(f"form projects must use OWNER/NUMBER: {', '.join(invalid_projects)}")
    return fields


def _fence_for(value: str) -> str:
    runs = [len(run) for run in re.findall(r"`+", value)]
    return "`" * max(3, (max(runs) + 1) if runs else 3)


def render_form(form: dict[str, Any], answers: dict[str, Any]) -> dict[str, Any]:
    fields = _validate_render_form(form)
    if not isinstance(answers, dict):
        raise FormError("answers must be an object keyed by field ID")
    if not all(isinstance(field_id, str) for field_id in answers):
        raise FormError("answers must be keyed by string field IDs")
    known_ids = {field["id"] for field in fields}
    unknown = sorted(set(answers) - known_ids)
    if unknown:
        raise FormError(f"answers contain unknown field IDs: {', '.join(unknown)}")
    sections: list[str] = []
    for field in fields:
        field_id = field["id"]
        value = answers.get(field_id)
        content: str
        if field["type"] in {"input", "textarea", "upload"}:
            if value is None:
                value = field.get("value") or ""
            if field["type"] == "upload" and isinstance(value, list) and all(isinstance(item, str) for item in value):
                value = "\n".join(value)
            if not isinstance(value, str):
                raise FormError(f"answer for {field_id} must be a string")
            if field["required"] and not value.strip():
                raise FormError(f"missing required answer: {field_id}")
            if field.get("render") and value:
                fence = _fence_for(value)
                content = f"{fence}{field['render']}\n{value.rstrip()}\n{fence}"
            else:
                content = value
        elif field["type"] == "dropdown":
            if value is None and field.get("default") is not None:
                selected = [field["options"][field["default"]]]
            else:
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
            invalid_verified = [item for item in verified if item not in labels]
            if invalid_verified:
                raise FormError(f"invalid verified checkbox value for {field_id}: {', '.join(invalid_verified)}")
            not_selected_verified = [item for item in verified if item not in selected]
            if not_selected_verified:
                raise FormError(
                    f"verified checkbox values for {field_id} must be selected: "
                    f"{', '.join(dict.fromkeys(not_selected_verified))}"
                )
            required_labels = [option["label"] for option in field["checkboxAttestations"] if option["required"]]
            unselected = [label for label in required_labels if label not in selected]
            if unselected:
                raise FormError(f"required checkbox attestations are not selected for {field_id}: {', '.join(unselected)}")
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
            "assignees": form.get("assignees") or [],
            "projects": form.get("projects") or [],
            "issueType": form.get("issueType"),
        },
    }


def read_json(path: Path, name: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
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
