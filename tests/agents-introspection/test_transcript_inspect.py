#!/usr/bin/env python3
"""End-to-end acceptance tests for the agents-introspection transcript inspector."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
INSPECTOR = REPO_ROOT / "skills/agents-introspection/scripts/transcript-inspect.py"


def message_record(role: str, text: str) -> dict[str, Any]:
    content_type = "input_text" if role in {"user", "developer", "system"} else "output_text"
    return {
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": role,
            "content": [{"type": content_type, "text": text}],
        },
    }


def session_meta(session_id: str, cwd: str, timestamp: str = "2026-08-08T12:00:00Z") -> dict[str, Any]:
    return {
        "timestamp": timestamp,
        "type": "session_meta",
        "payload": {"id": session_id, "cwd": cwd},
    }


def function_call_output(exit_code: int, output: str, timestamp: str | None = None) -> dict[str, Any]:
    record: dict[str, Any] = {
        "type": "response_item",
        "payload": {
            "type": "function_call_output",
            "output": json.dumps({"exit_code": exit_code, "output": output}),
        },
    }
    if timestamp:
        record["timestamp"] = timestamp
    return record


def write_jsonl(path: Path, records: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record) + "\n")


def run_inspector(
    paths: list[Path | str],
    *,
    keywords: list[str] | None = None,
    max_entries: int | None = None,
    output_format: str = "json",
    expect_returncode: int | None = 0,
) -> Any:
    command = [sys.executable, str(INSPECTOR), *[str(path) for path in paths]]
    for keyword in keywords or []:
        command.extend(["--keyword", keyword])
    if max_entries is not None:
        command.extend(["--max-entries", str(max_entries)])
    command.extend(["--format", output_format])
    result = subprocess.run(command, text=True, capture_output=True, check=False)
    if expect_returncode is not None and result.returncode != expect_returncode:
        raise AssertionError(f"unexpected exit code {result.returncode}:\n{result.stderr}\n{result.stdout}")
    if output_format == "json":
        return json.loads(result.stdout)
    return result.stdout


class TranscriptInspectTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name)

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_user_messages_have_correct_line_numbers_and_context_is_excluded(self) -> None:
        path = self.root / "session.jsonl"
        records = [
            session_meta("s1", "/repo"),
            message_record("developer", "<environment_context>ignored context</environment_context>"),
            message_record("user", "first user message"),
            message_record("user", "second user message"),
        ]
        write_jsonl(path, records)

        report = run_inspector([path])
        file_digest = report["files"][0]

        self.assertIsNone(file_digest["error"])
        user_entries = [entry for entry in file_digest["entries"] if entry["channel"] == "user"]
        self.assertEqual([entry["line"] for entry in user_entries], [2, 3])
        self.assertEqual([entry["text"] for entry in user_entries], ["first user message", "second user message"])
        self.assertEqual(file_digest["header"]["ignored_context_messages"], 1)
        self.assertEqual(file_digest["header"]["user_messages"], 2)
        self.assertEqual(file_digest["header"]["source"], "codex")

    def test_redaction_hides_email_and_secret_shaped_tokens(self) -> None:
        path = self.root / "session.jsonl"
        secret_token = "AKIAABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKL"
        secret_line = f"contact user@example.com with token {secret_token}"
        records = [
            session_meta("s1", "/repo"),
            message_record("user", secret_line),
        ]
        write_jsonl(path, records)

        json_report = run_inspector([path])
        text_report = run_inspector([path], output_format="text")

        json_text = json_report["files"][0]["entries"][0]["text"]
        self.assertIn("<email>", json_text)
        self.assertNotIn("user@example.com", json_text)
        self.assertNotIn("user@example.com", text_report)
        self.assertNotIn(secret_token, json_text)
        self.assertNotIn(secret_token, text_report)

    def test_keyword_filtering_selects_only_matching_assistant_messages_with_or_group(self) -> None:
        path = self.root / "session.jsonl"
        records = [
            session_meta("s1", "/repo"),
            message_record("user", "please look into this"),
            message_record("assistant", "unrelated reply about something else"),
            message_record("assistant", "found the needle here"),
            message_record("assistant", "also matched via haystack alternative"),
        ]
        write_jsonl(path, records)

        report = run_inspector([path], keywords=["needle|haystack"])
        entries = report["files"][0]["entries"]

        assistant_texts = {entry["text"] for entry in entries if entry["channel"] == "assistant"}
        self.assertEqual(assistant_texts, {"found the needle here", "also matched via haystack alternative"})

    def test_tool_failure_entry_reports_status_and_snippet(self) -> None:
        path = self.root / "session.jsonl"
        records = [
            session_meta("s1", "/repo"),
            message_record("user", "run the build"),
            function_call_output(2, "build failed: missing dependency"),
        ]
        write_jsonl(path, records)

        report = run_inspector([path])
        entries = report["files"][0]["entries"]
        failures = [entry for entry in entries if entry["channel"] == "tool_failure"]

        self.assertEqual(len(failures), 1)
        self.assertIn("exit_code=2", failures[0]["status"])
        self.assertIn("build failed", failures[0]["text"])
        self.assertEqual(report["files"][0]["header"]["tool_failures"], 1)

    def test_large_file_is_sampled_and_tail_line_numbers_are_absolute(self) -> None:
        path = self.root / "big-session.jsonl"
        records = [session_meta("s1", "/repo")]
        filler_text = "x" * 2000
        total_records = 2200
        for index in range(total_records):
            records.append(message_record("assistant", f"filler {index} {filler_text}"))
        marker_line_index = len(records)
        records.append(message_record("user", "final marker message"))
        write_jsonl(path, records)

        self.assertGreater(path.stat().st_size, 2_000_000)

        report = run_inspector([path], max_entries=500)
        file_digest = report["files"][0]

        self.assertTrue(file_digest["header"]["sampled"])
        marker_entries = [entry for entry in file_digest["entries"] if entry["text"] == "final marker message"]
        self.assertEqual(len(marker_entries), 1)
        self.assertEqual(marker_entries[0]["line"], marker_line_index)

    def test_nonexistent_path_among_good_paths_reports_error_but_exits_zero(self) -> None:
        good_path = self.root / "good.jsonl"
        write_jsonl(good_path, [session_meta("s1", "/repo"), message_record("user", "hello")])
        missing_path = self.root / "missing.jsonl"

        report = run_inspector([good_path, missing_path], expect_returncode=0)

        by_path = {entry["path"]: entry for entry in report["files"]}
        self.assertIsNone(by_path[str(good_path)]["error"])
        self.assertIsNotNone(by_path[str(missing_path)]["error"])

    def test_all_paths_bad_exits_one(self) -> None:
        missing_a = self.root / "missing-a.jsonl"
        missing_b = self.root / "missing-b.jsonl"

        report = run_inspector([missing_a, missing_b], expect_returncode=1)

        self.assertTrue(all(entry["error"] for entry in report["files"]))

    def test_max_entries_caps_output_and_reports_omitted_count(self) -> None:
        path = self.root / "session.jsonl"
        records = [session_meta("s1", "/repo")]
        for index in range(10):
            records.append(message_record("user", f"user message {index}"))
        write_jsonl(path, records)

        report = run_inspector([path], max_entries=3)
        file_digest = report["files"][0]

        self.assertEqual(len(file_digest["entries"]), 3)
        self.assertEqual(file_digest["omitted_entries"], 7)

    def test_text_format_smoke(self) -> None:
        path = self.root / "session.jsonl"
        write_jsonl(path, [session_meta("s1", "/repo"), message_record("user", "hello there")])

        text = run_inspector([path], output_format="text")

        self.assertIn(str(path), text)
        self.assertIn("hello there", text)
        self.assertIn("source=codex", text)


if __name__ == "__main__":
    unittest.main()
