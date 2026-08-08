#!/usr/bin/env python3
"""End-to-end acceptance tests for the agents-introspection transcript miner."""

from __future__ import annotations

import datetime as dt
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
MINER = REPO_ROOT / "skills/agents-introspection/scripts/transcript-miner.py"


class MinerFixture:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.codex_home = root / "codex"
        self.claude_home = root / "claude"
        self.projects = root / "projects"
        self.codex_sessions = self.codex_home / "sessions" / "2026" / "08" / "08"
        self.codex_sessions.mkdir(parents=True)
        (self.claude_home / "projects").mkdir(parents=True)
        self.projects.mkdir()

    def project(self, name: str, parent: Path | None = None) -> Path:
        project = (parent or self.projects) / name
        project.mkdir(parents=True)
        return project.resolve()

    def codex_session(
        self,
        session_id: str,
        cwd: Path,
        *,
        user: str | None = None,
        assistant: str | None = None,
        context: str | None = None,
        extra: list[dict[str, Any]] | None = None,
        session_meta: bool = True,
        turn_cwds: list[Path] | None = None,
        session_dir: Path | None = None,
    ) -> Path:
        records: list[dict[str, Any]] = []
        if session_meta:
            records.append(
                {
                    "timestamp": "2026-08-08T12:00:00Z",
                    "type": "session_meta",
                    "payload": {"id": session_id, "cwd": str(cwd)},
                }
            )
        for turn_cwd in turn_cwds or []:
            records.append({"type": "turn_context", "payload": {"cwd": str(turn_cwd)}})
        if context is not None:
            records.append(message_record("developer", context))
        if user is not None:
            records.append(message_record("user", user))
        if assistant is not None:
            records.append(message_record("assistant", assistant))
        records.extend(extra or [])
        path = (session_dir or self.codex_sessions) / f"rollout-{session_id}.jsonl"
        write_jsonl(path, records)
        return path

    def claude_session(
        self,
        directory_project: Path,
        session_id: str,
        cwd: Path,
        *,
        user: str | None = None,
        assistant: str | None = None,
    ) -> Path:
        project_dir = self.claude_home / "projects" / encode_claude_project(directory_project)
        project_dir.mkdir(parents=True, exist_ok=True)
        records: list[dict[str, Any]] = []
        if user is not None:
            records.append(claude_message("user", session_id, cwd, user))
        if assistant is not None:
            records.append(claude_message("assistant", session_id, cwd, assistant))
        if not records:
            records.append(claude_message("user", session_id, cwd, "placeholder"))
        path = project_dir / f"{session_id}.jsonl"
        write_jsonl(path, records)
        return path

    def history(self, records: list[dict[str, Any]]) -> None:
        write_jsonl(self.claude_home / "history.jsonl", records)

    def run(
        self,
        projects: list[Path],
        keywords: list[str],
        *,
        include_current: bool = False,
        env_updates: dict[str, str] | None = None,
        output_format: str = "json",
        since: str | None = None,
        excerpts: bool = False,
    ) -> Any:
        command = [sys.executable, str(MINER)]
        for project in projects:
            command.extend(["--project", str(project)])
        for keyword in keywords:
            command.extend(["--keyword", keyword])
        command.extend(["--max-sessions", "100", "--format", output_format])
        if include_current:
            command.append("--include-current")
        if since is not None:
            command.extend(["--since", since])
        if excerpts:
            command.append("--excerpts")
        env = os.environ.copy()
        env.update(
            {
                "CODEX_HOME": str(self.codex_home),
                "CLAUDE_CONFIG_DIR": str(self.claude_home),
            }
        )
        env.pop("CODEX_THREAD_ID", None)
        env.pop("CLAUDE_SESSION_ID", None)
        env.update(env_updates or {})
        result = subprocess.run(command, text=True, capture_output=True, check=False, env=env)
        if result.returncode != 0:
            raise AssertionError(f"miner failed ({result.returncode}):\n{result.stderr}\n{result.stdout}")
        return json.loads(result.stdout) if output_format == "json" else result.stdout


class TranscriptMinerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.fixture = MinerFixture(Path(self.temp_dir.name))

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_content_mentions_never_assign_foreign_projects(self) -> None:
        project_a = self.fixture.project("alpha")
        project_b = self.fixture.project("beta")
        project_c = self.fixture.project("gamma")
        extra = [
            {
                "type": "response_item",
                "payload": {"type": "function_call", "name": "exec", "arguments": json.dumps({"cwd": str(project_b)})},
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "output": json.dumps({"exit_code": 0, "output": f"read {project_c}"}),
                },
            },
        ]
        self.fixture.codex_session(
            "owned-alpha",
            project_a,
            user=f"needle; compare {project_b} and {project_c}",
            extra=extra,
        )

        report = self.fixture.run([project_a, project_b, project_c], ["needle"])

        self.assertEqual([session["project"] for session in report["candidate_sessions"]], [str(project_a)])
        coverage = coverage_by_project(report)
        self.assertEqual(coverage[project_b]["content_only_project_mentions_ignored"], 1)
        self.assertEqual(coverage[project_c]["content_only_project_mentions_ignored"], 1)
        self.assertEqual(coverage[project_b]["structurally_matched"], 0)

    def test_descendants_match_and_overlapping_roots_choose_most_specific(self) -> None:
        outer = self.fixture.project("outer")
        inner = self.fixture.project("inner", outer)
        working_dir = self.fixture.project("src", inner)
        self.fixture.codex_session("nested", working_dir, user="needle")

        report = self.fixture.run([outer, inner], ["needle"])

        self.assertEqual(len(report["candidate_sessions"]), 1)
        session = report["candidate_sessions"][0]
        self.assertEqual(session["project"], str(inner))
        self.assertEqual(session["ownership"]["cwd"], str(working_dir))

    def test_context_envelopes_do_not_supply_relevance(self) -> None:
        project = self.fixture.project("context")
        self.fixture.codex_session(
            "context-only",
            project,
            user="# AGENTS.md instructions for /tmp\nneedle pytest error transcript",
        )
        self.fixture.codex_session("real-user", project, user="Please fix needle")

        report = self.fixture.run([project], ["needle"])

        self.assertEqual([Path(session["path"]).stem for session in report["candidate_sessions"]], ["rollout-real-user"])
        self.assertEqual(report["candidate_sessions"][0]["signal_channels"]["eligible_user_messages"], 1)

    def test_keyword_hits_are_deduplicated_and_failures_cannot_qualify(self) -> None:
        project = self.fixture.project("dedupe")
        self.fixture.codex_session("repeat", project, user=" ".join(["needle"] * 5_000))
        self.fixture.codex_session(
            "generic-failure",
            project,
            user="unrelated request",
            extra=[
                {
                    "type": "response_item",
                    "payload": {"type": "function_call_output", "output": "failed error needle-free"},
                }
            ],
        )

        report = self.fixture.run([project], ["needle"])

        self.assertEqual(len(report["candidate_sessions"]), 1)
        session = report["candidate_sessions"][0]
        self.assertEqual(session["keyword_hits"], {"needle": 1})
        self.assertEqual(session["score"], 24)
        self.assertEqual(session["signal_channels"]["structured_tool_failures"], 0)

    def test_current_codex_and_claude_sessions_are_excluded_by_default(self) -> None:
        project = self.fixture.project("current")
        self.fixture.codex_session("codex-live", project, user="needle")
        self.fixture.claude_session(project, "claude-live", project, user="needle")
        self.fixture.history([{"project": str(project), "sessionId": "claude-live", "display": "needle"}])
        env = {"CODEX_THREAD_ID": "codex-live", "CLAUDE_SESSION_ID": "claude-live"}

        default_report = self.fixture.run([project], ["needle"], env_updates=env)
        included_report = self.fixture.run([project], ["needle"], include_current=True, env_updates=env)

        self.assertEqual(default_report["candidate_sessions"], [])
        self.assertEqual(coverage_by_project(default_report)[project]["current_sessions_excluded"], 2)
        self.assertEqual({session["source"] for session in included_report["candidate_sessions"]}, {"codex", "claude"})

    def test_claude_history_finds_old_session_behind_newer_irrelevant_files(self) -> None:
        project = self.fixture.project("history")
        history = [{"project": str(project), "sessionId": "old", "display": "needle task"}]
        self.fixture.claude_session(project, "old", project, user="needle task")
        for index in range(70):
            session_id = f"new-{index:02d}"
            self.fixture.claude_session(project, session_id, project, user="unrelated")
            history.append({"project": str(project), "sessionId": session_id, "display": "unrelated"})
        self.fixture.history(history)

        report = self.fixture.run([project], ["needle"])

        claude_sessions = [session for session in report["candidate_sessions"] if session["source"] == "claude"]
        self.assertEqual([Path(session["path"]).stem for session in claude_sessions], ["old"])
        self.assertEqual(coverage_by_project(report)[project]["claude_scanned"], 71)

    def test_conflicting_claude_ownership_is_ambiguous(self) -> None:
        project_a = self.fixture.project("claude-a")
        project_b = self.fixture.project("claude-b")
        self.fixture.claude_session(project_a, "conflict", project_a, user="needle")
        self.fixture.history([{"project": str(project_b), "sessionId": "conflict", "display": "needle"}])

        report = self.fixture.run([project_a, project_b], ["needle"])

        self.assertEqual(report["candidate_sessions"], [])
        coverage = coverage_by_project(report)
        self.assertEqual(coverage[project_a]["ambiguous_ownership_excluded"], 1)
        self.assertEqual(coverage[project_b]["ambiguous_ownership_excluded"], 1)

    def test_json_and_text_expose_diagnostics_without_excerpts(self) -> None:
        project = self.fixture.project("schema")
        secret_excerpt = "needle private excerpt that must not be printed"
        self.fixture.codex_session(
            "schema",
            project,
            user=secret_excerpt,
            assistant="tests passed",
            context="<environment_context>needle</environment_context>",
            extra=[
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call_output",
                        "output": json.dumps({"exit_code": 2, "output": "needle"}),
                    },
                }
            ],
        )

        report = self.fixture.run([project], ["needle"])
        text = self.fixture.run([project], ["needle"], output_format="text")

        for key in (
            "projects",
            "keywords",
            "candidate_sessions",
            "task_themes",
            "correction_signals",
            "failure_signals",
            "verification_signals",
            "tool_calls",
            "privacy_gaps",
        ):
            self.assertIn(key, report)
        coverage = coverage_by_project(report)[project]
        for key in (
            "codex_candidates",
            "claude_candidates",
            "selected_sessions",
            "codex_scanned",
            "claude_scanned",
            "structurally_matched",
            "relevance_matched",
            "current_sessions_excluded",
            "content_only_project_mentions_ignored",
            "ambiguous_ownership_excluded",
        ):
            self.assertIn(key, coverage)
        session = report["candidate_sessions"][0]
        for key in (
            "source",
            "project",
            "path",
            "timestamp",
            "title",
            "score",
            "keyword_hits",
            "task_themes",
            "correction_signals",
            "failure_signals",
            "verification_signals",
            "tool_calls",
            "privacy_gaps",
            "ownership",
            "signal_channels",
            "modified",
            "excerpts",
        ):
            self.assertIn(key, session)
        self.assertEqual(session["excerpts"], [])
        self.assertEqual(
            set(session["ownership"]),
            {"matched_via", "cwd", "project"},
        )
        self.assertEqual(
            set(session["signal_channels"]),
            {
                "eligible_user_messages",
                "eligible_assistant_messages",
                "ignored_context_messages",
                "structured_tool_failures",
            },
        )
        self.assertIn("ownership:", text)
        self.assertIn("channels:", text)
        self.assertIn("exclusions:", text)
        self.assertEqual(session["signal_channels"]["structured_tool_failures"], 1)
        self.assertNotIn(secret_excerpt, text)

    def test_excerpts_are_redacted_and_gated_by_flag(self) -> None:
        project = self.fixture.project("excerpts")
        secret_user = "please look into needle issue for user@example.com"
        self.fixture.codex_session(
            "excerpt-session",
            project,
            user=secret_user,
            assistant="needle handled, tests passed",
        )

        without_flag = self.fixture.run([project], ["needle"])
        with_flag_json = self.fixture.run([project], ["needle"], excerpts=True)
        with_flag_text = self.fixture.run([project], ["needle"], excerpts=True, output_format="text")

        self.assertEqual(without_flag["candidate_sessions"][0]["excerpts"], [])

        session = with_flag_json["candidate_sessions"][0]
        self.assertTrue(session["excerpts"])
        self.assertEqual(session["excerpts"][0]["channel"], "user")
        self.assertIn("<email>", session["excerpts"][0]["text"])
        self.assertNotIn("user@example.com", session["excerpts"][0]["text"])
        self.assertIn("excerpt[user]:", with_flag_text)
        self.assertNotIn("user@example.com", with_flag_text)

    def test_since_prunes_old_codex_dirs_and_claude_files(self) -> None:
        project = self.fixture.project("since")
        self.fixture.codex_session("recent", project, user="needle recent")
        old_dir = self.fixture.codex_home / "sessions" / "2020" / "01" / "01"
        old_codex = self.fixture.codex_session("old", project, user="needle old", session_dir=old_dir)
        old_mtime = (dt.datetime.now(dt.timezone.utc) - dt.timedelta(days=400)).timestamp()
        os.utime(old_codex, (old_mtime, old_mtime))
        revived_dir = self.fixture.codex_home / "sessions" / "2020" / "02" / "02"
        self.fixture.codex_session("revived", project, user="needle revived recently", session_dir=revived_dir)
        self.fixture.codex_session("flat", project, user="needle flat layout", session_dir=self.fixture.codex_home / "sessions")

        self.fixture.claude_session(project, "recent-claude", project, user="needle recent claude")
        old_claude = self.fixture.claude_session(project, "old-claude", project, user="needle old claude")
        os.utime(old_claude, (old_mtime, old_mtime))

        no_since = self.fixture.run([project], ["needle"])
        self.assertIsNone(no_since["since"])

        since_report = self.fixture.run([project], ["needle"], since="60d")
        self.assertIsNotNone(since_report["since"])
        self.assertGreaterEqual(since_report["since"]["codex_dirs_pruned"], 1)
        self.assertGreaterEqual(since_report["since"]["codex_files_pruned"], 1)
        self.assertGreaterEqual(since_report["since"]["claude_files_pruned"], 1)

        stems = {Path(session["path"]).stem for session in since_report["candidate_sessions"]}
        self.assertIn("rollout-recent", stems)
        self.assertIn("rollout-revived", stems)
        self.assertIn("rollout-flat", stems)
        self.assertNotIn("rollout-old", stems)
        self.assertIn("recent-claude", stems)
        self.assertNotIn("old-claude", stems)

    def test_or_group_keyword_counts_one_hit_per_message(self) -> None:
        project = self.fixture.project("or-group")
        self.fixture.codex_session(
            "or-group",
            project,
            user="the transcript-miner needs mining improvements",
        )

        report = self.fixture.run([project], ["miner|mining|transcript-miner"])

        session = report["candidate_sessions"][0]
        self.assertEqual(session["keyword_hits"], {"miner|mining|transcript-miner": 1})

    def test_prescan_keeps_user_matches_and_gate_still_excludes_body_only_matches(self) -> None:
        project = self.fixture.project("prescan")
        self.fixture.codex_session(
            "user-only",
            project,
            user="please handle sentinelword now",
        )
        self.fixture.codex_session(
            "body-only",
            project,
            user="unrelated request",
            extra=[
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call_output",
                        "output": json.dumps({"exit_code": 0, "output": "sentinelword appears only here"}),
                    },
                }
            ],
        )

        report = self.fixture.run([project], ["sentinelword"])

        stems = {Path(session["path"]).stem for session in report["candidate_sessions"]}
        self.assertIn("rollout-user-only", stems)
        self.assertNotIn("rollout-body-only", stems)
        coverage = coverage_by_project(report)[project]
        self.assertEqual(coverage["structurally_matched"], 2)
        self.assertEqual(coverage["relevance_matched"], 1)

    def test_candidates_expose_modified_timestamp(self) -> None:
        project = self.fixture.project("modified")
        self.fixture.codex_session("modified", project, user="needle")

        report = self.fixture.run([project], ["needle"])

        session = report["candidate_sessions"][0]
        self.assertRegex(session["modified"], r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")


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


def claude_message(role: str, session_id: str, cwd: Path, text: str) -> dict[str, Any]:
    return {
        "type": role,
        "sessionId": session_id,
        "cwd": str(cwd),
        "timestamp": "2026-08-08T12:00:00Z",
        "message": {"role": role, "content": [{"type": "text", "text": text}]},
    }


def encode_claude_project(project: Path) -> str:
    return re.sub(r"[^A-Za-z0-9]", "-", str(project))


def write_jsonl(path: Path, records: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record) + "\n")


def coverage_by_project(report: dict[str, Any]) -> dict[Path, dict[str, Any]]:
    return {Path(project["path"]): project["coverage"] for project in report["projects"]}


if __name__ == "__main__":
    unittest.main()
