#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Bounded, always-redacted digest of Codex/Claude Code transcript JSONL files."""

from __future__ import annotations

import argparse
import json
import os
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from transcript_common import (
    CORRECTION_PATTERNS,
    MAX_FULL_SESSION_BYTES,
    VERIFICATION_PATTERNS,
    extract_record_channels,
    extract_tool_failures,
    first_string_shallow,
    keyword_alternatives,
    read_jsonl_sampled_with_lines,
    redact_text,
    truncate,
)


CODEX_RECORD_TYPES = {"session_meta", "turn_context", "response_item"}
ENTRY_TEXT_LIMIT = 240
DEFAULT_MAX_ENTRIES = 120


@dataclass
class Header:
    path: str
    source: str
    session_id: str
    cwd: str | None
    first_timestamp: str | None
    last_timestamp: str | None
    user_messages: int
    assistant_messages: int
    ignored_context_messages: int
    tool_failures: int
    sampled: bool


@dataclass
class Entry:
    line: int
    channel: str
    tool: str | None
    status: str | None
    text: str


@dataclass
class FileDigest:
    path: str
    error: str | None
    header: Header | None
    entries: list[Entry] = field(default_factory=list)
    omitted_entries: int = 0

    def to_json(self) -> dict[str, Any]:
        return {
            "path": self.path,
            "error": self.error,
            "header": asdict(self.header) if self.header else None,
            "entries": [asdict(entry) for entry in self.entries],
            "omitted_entries": self.omitted_entries,
        }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Inspect Codex/Claude Code transcript JSONL files with bounded, redacted digests."
    )
    parser.add_argument("paths", nargs="+", help="Transcript JSONL file paths")
    parser.add_argument(
        "--keyword",
        action="append",
        default=[],
        help="Keyword to match. Repeatable. Use 'a|b|c' for an OR-group of alternatives",
    )
    parser.add_argument("--max-entries", type=int, default=DEFAULT_MAX_ENTRIES, help="Maximum emitted entries per file")
    parser.add_argument("--format", choices=("text", "json"), default="text")
    args = parser.parse_args()

    if args.max_entries < 1:
        print("transcript-inspect: --max-entries must be positive", file=sys.stderr)
        return 2

    keywords = [keyword for keyword in args.keyword if keyword.strip()]
    digests = [inspect_file(raw_path, keywords, args.max_entries) for raw_path in args.paths]

    if args.format == "json":
        print(json.dumps({"files": [digest.to_json() for digest in digests]}, indent=2, sort_keys=True))
    else:
        print_text_report(digests)

    return 1 if digests and all(digest.error is not None for digest in digests) else 0


def inspect_file(raw_path: str, keywords: list[str], max_entries: int) -> FileDigest:
    path = Path(os.path.expanduser(raw_path))
    if not path.exists():
        return FileDigest(raw_path, f"path does not exist: {path}", None)
    if not path.is_file():
        return FileDigest(raw_path, f"not a file: {path}", None)
    try:
        sampled = path.stat().st_size > MAX_FULL_SESSION_BYTES
    except OSError as error:
        return FileDigest(raw_path, f"cannot stat file: {error}", None)

    records = list(read_jsonl_sampled_with_lines(path))
    if not records:
        return FileDigest(raw_path, "no parsable JSONL records (missing, empty, or unreadable)", None)

    items = [item for _, item in records]
    source = guess_source(items)
    session_id = extract_session_id(items, path)
    cwd = extract_cwd(items)

    timestamps: list[str] = []
    user_all: list[tuple[int, str]] = []
    assistant_all: list[tuple[int, str]] = []
    failure_all: list[tuple[int, dict[str, str | None]]] = []
    context_count = 0

    for line, item in records:
        timestamp = first_string_shallow(item, ("timestamp", "created_at", "updated_at"))
        if timestamp:
            timestamps.append(timestamp)
        channels = extract_record_channels(item)
        user_all.extend((line, text) for text in channels["user"])
        assistant_all.extend((line, text) for text in channels["assistant"])
        context_count += len(channels["context"])
        failure_all.extend((line, detail) for detail in extract_tool_failures(item))

    remaining_budget = max(0, max_entries - len(user_all) - len(failure_all))
    if not keywords and len(assistant_all) <= remaining_budget:
        assistant_selected = assistant_all
    else:
        assistant_selected = [
            (line, text) for line, text in assistant_all if assistant_message_qualifies(text, keywords)
        ]

    combined: list[tuple[int, int, Entry]] = []
    for line, text in user_all:
        combined.append((line, 0, Entry(line, "user", None, None, truncate(redact_text(text), ENTRY_TEXT_LIMIT))))
    for line, text in assistant_selected:
        combined.append((line, 1, Entry(line, "assistant", None, None, truncate(redact_text(text), ENTRY_TEXT_LIMIT))))
    for line, detail in failure_all:
        combined.append(
            (
                line,
                2,
                Entry(
                    line,
                    "tool_failure",
                    detail.get("tool"),
                    redact_text(detail["status"]) if detail.get("status") else None,
                    truncate(redact_text(detail.get("text") or ""), ENTRY_TEXT_LIMIT),
                ),
            )
        )
    combined.sort(key=lambda entry: (entry[0], entry[1]))

    entries = [entry for _, _, entry in combined[:max_entries]]
    omitted = max(0, len(combined) - max_entries)

    header = Header(
        path=str(path),
        source=source,
        session_id=session_id,
        cwd=cwd,
        first_timestamp=timestamps[0] if timestamps else None,
        last_timestamp=timestamps[-1] if timestamps else None,
        user_messages=len(user_all),
        assistant_messages=len(assistant_all),
        ignored_context_messages=context_count,
        tool_failures=len(failure_all),
        sampled=sampled,
    )
    return FileDigest(raw_path, None, header, entries, omitted)


def assistant_message_qualifies(text: str, keywords: list[str]) -> bool:
    if keyword_group_matches(text, keywords):
        return True
    return any(pattern.search(text) for pattern in (*CORRECTION_PATTERNS.values(), *VERIFICATION_PATTERNS.values()))


def keyword_group_matches(text: str, keywords: list[str]) -> bool:
    if not keywords:
        return False
    lowered = text.lower()
    for keyword in keywords:
        alternatives = [alternative.lower() for alternative in keyword_alternatives(keyword)]
        if alternatives and any(alternative in lowered for alternative in alternatives):
            return True
    return False


def guess_source(items: list[Any]) -> str:
    for item in items:
        if isinstance(item, dict) and item.get("type") in CODEX_RECORD_TYPES:
            return "codex"
    for item in items:
        if isinstance(item, dict) and ("sessionId" in item or "cwd" in item):
            return "claude"
    return "unknown"


def extract_session_id(items: list[Any], path: Path) -> str:
    for item in items:
        if not isinstance(item, dict):
            continue
        payload = item.get("payload") if isinstance(item.get("payload"), dict) else None
        if item.get("type") == "session_meta" and isinstance(payload, dict):
            value = first_string_shallow(payload, ("id", "session_id", "conversation_id"))
            if value:
                return value
        value = first_string_shallow(item, ("sessionId", "session_id", "conversation_id"))
        if value:
            return value
    return path.stem


def extract_cwd(items: list[Any]) -> str | None:
    for item in items:
        if not isinstance(item, dict):
            continue
        payload = item.get("payload") if isinstance(item.get("payload"), dict) else None
        if isinstance(payload, dict):
            value = first_string_shallow(payload, ("cwd",))
            if value:
                return value
        value = first_string_shallow(item, ("cwd",))
        if value:
            return value
    return None


def print_text_report(digests: list[FileDigest]) -> None:
    for index, digest in enumerate(digests):
        if index:
            print()
        print(f"== {digest.path} ==")
        if digest.error or digest.header is None:
            print(f"ERROR: {digest.error}")
            continue
        header = digest.header
        print(f"source={header.source} session={header.session_id} cwd={header.cwd or '-'} sampled={header.sampled}")
        print(
            f"first={header.first_timestamp or '-'} last={header.last_timestamp or '-'} "
            f"user={header.user_messages} assistant={header.assistant_messages} "
            f"context-ignored={header.ignored_context_messages} tool-failures={header.tool_failures}"
        )
        for entry in digest.entries:
            if entry.channel == "tool_failure":
                tool = entry.tool or "unknown-tool"
                status = entry.status or "unknown-status"
                print(f"[{entry.line}] tool_failure ({tool}, {status}): {entry.text}")
            else:
                print(f"[{entry.line}] {entry.channel}: {entry.text}")
        if digest.omitted_entries:
            print(f"... {digest.omitted_entries} entries omitted")


if __name__ == "__main__":
    raise SystemExit(main())
