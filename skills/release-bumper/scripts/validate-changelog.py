#!/usr/bin/env python3
"""Validate deterministic Common Changelog structure for one release."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path


CATEGORIES = ("Changed", "Added", "Removed", "Fixed")
RELEASE_RE = re.compile(r"^## (?P<linked>\[)?(?P<version>\d+\.\d+\.\d+)(?(linked)\]) - (?P<date>\d{4}-\d{2}-\d{2})$")
REFERENCE_RE = re.compile(r"^\[(?P<version>\d+\.\d+\.\d+)\]:\s+(?P<url>\S+)\s*$")


def validate(text: str, version: str, date: str, tag: str | None) -> list[str]:
    errors: list[str] = []
    try:
        dt.date.fromisoformat(date)
    except ValueError:
        return [f"invalid expected date: {date}"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        return [f"invalid stable version: {version}"]

    lines = text.splitlines()
    if not lines or lines[0] != "# Changelog":
        errors.append("file must start with '# Changelog'")
    releases = [(index, match) for index, line in enumerate(lines) if (match := RELEASE_RE.fullmatch(line))]
    expected = [(index, match) for index, match in releases if match.group("version") == version]
    if len(expected) != 1:
        errors.append(f"expected exactly one release heading for {version}")
        return errors
    start, heading = expected[0]
    if heading.group("date") != date:
        errors.append(f"release {version} date is {heading.group('date')}, expected {date}")
    if releases and releases[0][0] != start:
        errors.append(f"release {version} is not the first release entry")
    end = next((index for index in range(start + 1, len(lines)) if lines[index].startswith("## ")), len(lines))
    body = lines[start + 1 : end]

    headings: list[tuple[int, str]] = []
    for offset, line in enumerate(body):
        if line.startswith("### "):
            category = line[4:]
            if category not in CATEGORIES:
                errors.append(f"unsupported category: {category}")
            else:
                headings.append((offset, category))
        elif line.startswith("####"):
            errors.append("release entries may not contain fourth-level headings")
    category_names = [category for _, category in headings]
    if not category_names and not any(line.startswith("_") and line.endswith("_") for line in body):
        errors.append("release must contain a notice or change category")
    if len(category_names) != len(set(category_names)):
        errors.append("release categories may not repeat")
    if category_names != sorted(category_names, key=CATEGORIES.index):
        errors.append("release categories are out of order")

    for position, (offset, category) in enumerate(headings):
        group_end = headings[position + 1][0] if position + 1 < len(headings) else len(body)
        content = [
            line
            for line in body[offset + 1 : group_end]
            if line.strip() and not REFERENCE_RE.fullmatch(line)
        ]
        bullets = [line for line in content if line.startswith("- ")]
        if not bullets:
            errors.append(f"category {category} must contain an unnumbered list")
        if any(re.match(r"^\d+[.)]\s", line) for line in content):
            errors.append(f"category {category} contains a numbered list")
        if any(line.startswith(("  ", "\t")) for line in content):
            errors.append(f"category {category} contains a multiline or nested list item")
        if any(not line.startswith("- ") for line in content):
            errors.append(f"category {category} contains non-list content")

    references = {
        match.group("version"): match.group("url")
        for line in lines
        if (match := REFERENCE_RE.fullmatch(line))
    }
    if heading.group("linked"):
        url = references.get(version)
        if not url:
            errors.append(f"linked release {version} has no reference definition")
        else:
            actual_tag = url.rstrip("/").rsplit("/", 1)[-1]
            allowed_tags = {tag} if tag else {version, f"v{version}"}
            if actual_tag not in allowed_tags:
                errors.append(f"release link tag is {actual_tag}, expected {' or '.join(sorted(allowed_tags))}")
    else:
        errors.append(f"release {version} heading must be linked")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--file", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--date", required=True)
    parser.add_argument("--tag")
    args = parser.parse_args()
    try:
        text = args.file.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 64
    errors = validate(text, args.version, args.date, args.tag)
    print(json.dumps({"schemaVersion": 1, "valid": not errors, "errors": errors}, indent=2))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
