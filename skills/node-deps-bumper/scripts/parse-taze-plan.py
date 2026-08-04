#!/usr/bin/env -S uv run --script
"""Convert stable, no-color Taze output into a machine-readable update plan."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


ANSI_RE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
VERSION_RE = re.compile(r"^(?:workspace:)?[~^<>=v]*([0-9]+)\.([0-9]+)\.([0-9]+)(?:[-+][0-9A-Za-z.-]+)?$")


def version_parts(value: str) -> tuple[int, int, int] | None:
    match = VERSION_RE.match(value)
    return tuple(map(int, match.groups())) if match else None


def classify(current: str, available: str) -> str:
    old = version_parts(current)
    new = version_parts(available)
    if old is None or new is None:
        return "unknown"
    if old[0] != new[0]:
        return "major"
    if old[1] != new[1]:
        return "minor"
    return "patch"


def parse(text: str) -> list[dict[str, str]]:
    entries: list[dict[str, str]] = []
    for raw_line in ANSI_RE.sub("", text).splitlines():
        if "→" not in raw_line:
            continue
        left, right = raw_line.split("→", 1)
        left_tokens = left.split()
        right_tokens = right.split()
        if not left_tokens or not right_tokens:
            continue
        available = right_tokens[0]
        current_index = next(
            (index for index in range(len(left_tokens) - 1, -1, -1) if version_parts(left_tokens[index]) is not None),
            None,
        )
        if current_index is None or version_parts(available) is None:
            continue
        current = left_tokens[current_index]
        package = left_tokens[0]
        update_type = classify(current, available)
        fixed = not current.startswith(("^", "~", ">", "<", "="))
        if fixed:
            action = "skip-fixed"
        elif update_type == "major":
            action = "review-major"
        elif update_type in {"minor", "patch"}:
            action = "apply"
        else:
            action = "review"
        entries.append(
            {
                "package": package,
                "current": current,
                "available": available,
                "type": update_type,
                "action": action,
            }
        )
    priority = {"review-major": 0, "review": 1, "apply": 2, "skip-fixed": 3}
    return sorted(entries, key=lambda item: (priority[item["action"]], item["package"]))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    args = parser.parse_args()
    entries = parse(args.input.read_text(encoding="utf-8", errors="replace"))
    counts = {key: sum(item["type"] == key for item in entries) for key in ("major", "minor", "patch", "unknown")}
    print(json.dumps({"updates": entries, "counts": counts, "total": len(entries)}, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
