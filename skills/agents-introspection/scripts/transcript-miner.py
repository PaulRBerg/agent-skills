#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Mine Codex and Claude Code transcript metadata without emitting transcript excerpts.

Usage:
  uv run /path/to/agents-introspection/scripts/transcript-miner.py
      [--project PATH ...] [--keyword TEXT ...] [--format text|json]
      [--max-sessions N] [--include-archived]
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from collections import Counter, defaultdict, deque
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Iterable


EMAIL_RE = re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")
API_KEY_RE = re.compile(r"\b(?:sk|pk|rk|ghp|github_pat|xox[baprs])-[A-Za-z0-9_\-]{16,}\b")
GENERIC_SECRET_RE = re.compile(
    r"(?i)\b(?:api[_-]?key|access[_-]?token|secret(?:[_-]?key)?|private[_-]?key|password)\b"
    r"\s*[:=]\s*[\"']?(?![$<{[])([A-Za-z0-9_./+=-]{16,})"
)
EVM_ADDRESS_RE = re.compile(r"(?<![0-9a-fA-F])0x[0-9a-fA-F]{40}(?![0-9a-fA-F])")
HEX_64_RE = re.compile(r"(?<![0-9a-fA-F])0x[0-9a-fA-F]{64}(?![0-9a-fA-F])")
LONG_SECRETISH_RE = re.compile(r"(?<![A-Za-z0-9_/-])[A-Za-z0-9_+=/-]{48,}(?![A-Za-z0-9_/-])")

CORRECTION_PATTERNS = {
    "user-correction": re.compile(r"(?i)\b(actually|wrong|instead|i asked|not what|do not|don't|stop|you should)\b"),
    "instruction-reminder": re.compile(r"(?i)\b(AGENTS\.md|instructions?|follow .*rules?|violat(?:e|ed|ion))\b"),
}
FAILURE_PATTERNS = {
    "command-failure": re.compile(r"(?i)\b(exit code|non-zero|failed|failure|error|traceback|exception)\b"),
    "shell-failure": re.compile(r"(?i)\b(zsh:|bash:|command not found|permission denied|no such file|not a directory)\b"),
    "path-cwd-failure": re.compile(r"(?i)\b(wrong cwd|wrong directory|path .*not found|could not find|missing file)\b"),
}
VERIFICATION_PATTERNS = {
    "tests": re.compile(r"(?i)\b(test|pytest|vitest|unit tests?|integration tests?)\b"),
    "lint-format": re.compile(r"(?i)\b(lint|format|mdformat|prettier|ruff|eslint)\b"),
    "repo-check": re.compile(r"(?i)\b(just |uv run|cargo check|npm run|pnpm |bun test|verified|verification|passes?|passed)\b"),
}
THEME_PATTERNS = {
    "agent-skills": re.compile(r"(?i)\b(SKILL\.md|openai\.yaml|frontmatter|allow_implicit|agent skills?|skill catalog)\b"),
    "transcripts": re.compile(r"(?i)\b(transcript|session_index|sessions/|claude/projects|history\.jsonl)\b"),
    "markdown": re.compile(r"(?i)\b(markdown|mdformat|README\.md|AGENTS\.md)\b"),
    "git": re.compile(r"(?i)\b(git status|git diff|commit|branch|worktree|staged)\b"),
    "shell-tooling": re.compile(r"(?i)\b(zsh|bash|just|uv run|rg |fd |jq )\b"),
    "privacy": re.compile(r"(?i)\b(secret|redact|private key|api key|token|wallet|address)\b"),
}
MAX_FULL_SESSION_BYTES = 2_000_000
SESSION_HEAD_RECORDS = 250
SESSION_TAIL_RECORDS = 750
MAX_SIGNAL_STRINGS_PER_RECORD = 25
MAX_SIGNAL_STRINGS_PER_SESSION = 3_000
MAX_TOOL_RESULT_FILES = 200


@dataclass
class SessionSummary:
    source: str
    project: str
    path: str
    timestamp: str | None = None
    title: str | None = None
    score: int = 0
    keyword_hits: dict[str, int] = field(default_factory=dict)
    task_themes: dict[str, int] = field(default_factory=dict)
    correction_signals: dict[str, int] = field(default_factory=dict)
    failure_signals: dict[str, int] = field(default_factory=dict)
    verification_signals: dict[str, int] = field(default_factory=dict)
    tool_calls: dict[str, int] = field(default_factory=dict)
    privacy_gaps: dict[str, int] = field(default_factory=dict)


def main() -> int:
    parser = argparse.ArgumentParser(description="Mine project-scoped Codex and Claude Code transcript signals.")
    parser.add_argument("--project", action="append", default=[], help="Project path to mine. Repeatable. Default: pwd -P")
    parser.add_argument("--keyword", action="append", default=[], help="Task keyword to score. Repeatable")
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--max-sessions", type=int, default=20, help="Maximum selected sessions per project")
    parser.add_argument("--include-archived", action="store_true", help="Include ~/.codex/archived_sessions")
    args = parser.parse_args()

    if args.max_sessions < 1:
        print("transcript-miner: --max-sessions must be positive", file=sys.stderr)
        return 2

    try:
        projects = normalize_projects(args.project)
    except ValueError as error:
        print(f"transcript-miner: {error}", file=sys.stderr)
        return 2

    report = mine_transcripts(
        projects,
        keywords=[keyword for keyword in args.keyword if keyword.strip()],
        max_sessions=args.max_sessions,
        include_archived=args.include_archived,
    )

    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_text_report(report)
    return 0


def normalize_projects(raw_projects: list[str]) -> list[Path]:
    candidates = raw_projects or [os.curdir]
    projects: list[Path] = []
    for raw_project in candidates:
        project = Path(os.path.expanduser(raw_project)).resolve()
        if not project.exists():
            raise ValueError(f"project does not exist: {project}")
        if not project.is_dir():
            raise ValueError(f"project is not a directory: {project}")
        projects.append(project)
    return projects


def mine_transcripts(
    projects: list[Path],
    *,
    keywords: list[str],
    max_sessions: int,
    include_archived: bool,
) -> dict[str, Any]:
    codex_home = Path(os.path.expanduser(os.environ.get("CODEX_HOME", "~/.codex"))).resolve()
    claude_home = claude_config_dir()
    codex_index = load_codex_index(codex_home / "session_index.jsonl")
    project_reports: list[dict[str, Any]] = []
    selected_sessions: list[SessionSummary] = []

    for project in projects:
        project_keywords = list(dict.fromkeys([*keywords, project.name]))
        codex_sessions = mine_codex_project(
            project,
            project_keywords,
            codex_home,
            codex_index,
            include_archived,
            max_sessions=max_sessions,
        )
        claude_sessions, claude_tool_failures, claude_paths = mine_claude_project(
            project,
            project_keywords,
            claude_home,
            max_sessions=max_sessions,
        )
        project_sessions = select_sessions([*codex_sessions, *claude_sessions], max_sessions)
        selected_sessions.extend(project_sessions)

        project_reports.append(
            {
                "path": str(project),
                "encoded_claude_project": encode_claude_project(project),
                "coverage": {
                    "codex_candidates": len(codex_sessions),
                    "claude_candidates": len(claude_sessions),
                    "selected_sessions": len(project_sessions),
                    "claude_tool_result_files": len(claude_paths),
                    "claude_tool_result_failures": dict(claude_tool_failures),
                    "codex_index_records": codex_index["records"],
                },
            }
        )

    totals = aggregate_sessions(selected_sessions)
    return {
        "projects": project_reports,
        "keywords": [redact_text(keyword) for keyword in keywords],
        "candidate_sessions": [session_to_json(session) for session in selected_sessions],
        "task_themes": dict(totals["task_themes"]),
        "correction_signals": dict(totals["correction_signals"]),
        "failure_signals": dict(totals["failure_signals"]),
        "verification_signals": dict(totals["verification_signals"]),
        "tool_calls": dict(totals["tool_calls"]),
        "privacy_gaps": dict(totals["privacy_gaps"]),
    }


def load_codex_index(path: Path) -> dict[str, Any]:
    titles_by_id: dict[str, str] = {}
    titles_by_path: dict[str, str] = {}
    records = 0
    if not path.is_file():
        return {"records": 0, "titles_by_id": titles_by_id, "titles_by_path": titles_by_path}

    for item in read_jsonl(path):
        records += 1
        session_id = first_string(item, ("id", "session_id", "conversation_id"))
        transcript_path = first_string(item, ("path", "file", "rollout_path", "transcript_path"))
        title = first_string(item, ("title", "summary", "task", "prompt"))
        if title and session_id:
            titles_by_id[session_id] = redact_text(title)
        if title and transcript_path:
            titles_by_path[str(Path(os.path.expanduser(transcript_path)).resolve())] = redact_text(title)

    return {"records": records, "titles_by_id": titles_by_id, "titles_by_path": titles_by_path}


def mine_codex_project(
    project: Path,
    keywords: list[str],
    codex_home: Path,
    codex_index: dict[str, Any],
    include_archived: bool,
    max_sessions: int,
) -> list[SessionSummary]:
    roots = [codex_home / "sessions"]
    if include_archived:
        roots.append(codex_home / "archived_sessions")

    sessions: list[SessionSummary] = []
    max_candidate_files = max(25, max_sessions * 8)
    for root in roots:
        if not root.is_dir():
            continue
        for path, has_project_match in codex_project_paths(root, project, max_candidate_files):
            summary = summarize_jsonl_session(
                path,
                source="codex",
                project=project,
                keywords=keywords,
                title_hint=title_for_codex_path(path, codex_index),
                require_project_match=not has_project_match,
            )
            if summary is not None:
                sessions.append(summary)
    return sessions


def codex_project_paths(root: Path, project: Path, max_paths: int) -> list[tuple[Path, bool]]:
    if shutil.which("rg"):
        result = subprocess.run(
            ["rg", "-l", "-F", str(project), str(root)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode in (0, 1):
            paths = [Path(line).resolve() for line in result.stdout.splitlines() if line.endswith(".jsonl")]
            return [(path, True) for path in sorted(paths, key=mtime_sort_key, reverse=True)[:max_paths]]

    return [(path, False) for path in sorted(root.rglob("*.jsonl"), key=mtime_sort_key, reverse=True)[:max_paths]]


def mine_claude_project(
    project: Path,
    keywords: list[str],
    claude_home: Path,
    max_sessions: int,
) -> tuple[list[SessionSummary], Counter[str], list[str]]:
    project_dirs = [
        claude_home / "projects" / project_key
        for project_key in claude_project_keys(project)
    ]
    project_dirs = list(dict.fromkeys(project_dirs))
    existing_project_dirs = [project_dir for project_dir in project_dirs if project_dir.is_dir()]
    if not existing_project_dirs:
        return [], Counter(), []

    sessions: list[SessionSummary] = []
    max_candidate_files = max(25, max_sessions * 8)
    candidate_paths = [
        path
        for project_dir in existing_project_dirs
        for path in project_dir.glob("*.jsonl")
    ]
    for path in sorted(candidate_paths, key=mtime_sort_key, reverse=True)[:max_candidate_files]:
        summary = summarize_jsonl_session(
            path,
            source="claude",
            project=project,
            keywords=keywords,
            title_hint=None,
            require_project_match=False,
        )
        if summary is not None:
            sessions.append(summary)

    tool_failures: Counter[str] = Counter()
    tool_paths: list[str] = []
    for project_dir in existing_project_dirs:
        tool_results_dir = project_dir / "tool-results"
        if tool_results_dir.is_dir():
            for path in sorted(tool_results_dir.rglob("*"), key=mtime_sort_key, reverse=True):
                if not path.is_file():
                    continue
                if len(tool_paths) >= MAX_TOOL_RESULT_FILES:
                    break
                tool_paths.append(str(path))
                text = read_text_limited(path)
                if not text:
                    continue
                signals = count_patterns([text], FAILURE_PATTERNS)
                tool_failures.update(signals)

    return sessions, tool_failures, tool_paths


def summarize_jsonl_session(
    path: Path,
    *,
    source: str,
    project: Path,
    keywords: list[str],
    title_hint: str | None,
    require_project_match: bool,
) -> SessionSummary | None:
    project_text = str(project)
    timestamp: str | None = None
    title = title_hint
    tool_calls: Counter[str] = Counter()
    keyword_hits: Counter[str] = Counter()
    corrections: Counter[str] = Counter()
    failures: Counter[str] = Counter()
    verification: Counter[str] = Counter()
    themes: Counter[str] = Counter()
    privacy: Counter[str] = Counter()
    project_match = not require_project_match
    signal_strings_seen = 0

    for item in read_jsonl_sampled(path):
        if timestamp is None:
            timestamp = first_string(item, ("timestamp", "created_at", "updated_at"))
        if title is None:
            title = first_string(item, ("title", "summary", "task"))
            if title:
                title = redact_text(title)
        collect_tool_calls(item, tool_calls)
        signal_strings: list[str] = []
        for text in extract_strings(item):
            if not project_match and project_text in text:
                project_match = True
            if not text or signal_strings_seen >= MAX_SIGNAL_STRINGS_PER_SESSION:
                continue
            if len(signal_strings) >= MAX_SIGNAL_STRINGS_PER_RECORD:
                continue
            signal_strings.append(signal_text(text))
            signal_strings_seen += 1
        keyword_hits.update(count_keywords(signal_strings, keywords))
        corrections.update(count_patterns(signal_strings, CORRECTION_PATTERNS))
        failures.update(count_patterns(signal_strings, FAILURE_PATTERNS))
        verification.update(count_patterns(signal_strings, VERIFICATION_PATTERNS))
        themes.update(count_patterns(signal_strings, THEME_PATTERNS))
        privacy.update(count_privacy_gaps(signal_strings))

    if not project_match:
        return None

    score = (
        10
        + sum(keyword_hits.values()) * 3
        + sum(corrections.values()) * 2
        + sum(failures.values())
        + sum(verification.values())
        + sum(themes.values())
    )

    return SessionSummary(
        source=source,
        project=str(project),
        path=str(path),
        timestamp=timestamp,
        title=truncate(redact_text(title)) if title else None,
        score=score,
        keyword_hits=dict(keyword_hits),
        task_themes=dict(themes),
        correction_signals=dict(corrections),
        failure_signals=dict(failures),
        verification_signals=dict(verification),
        tool_calls=dict(tool_calls),
        privacy_gaps=dict(privacy),
    )


def select_sessions(sessions: list[SessionSummary], max_sessions: int) -> list[SessionSummary]:
    return sorted(sessions, key=lambda session: (session.score, session.timestamp or "", session.path), reverse=True)[:max_sessions]


def aggregate_sessions(sessions: list[SessionSummary]) -> dict[str, Counter[str]]:
    totals: dict[str, Counter[str]] = defaultdict(Counter)
    for session in sessions:
        totals["task_themes"].update(session.task_themes)
        totals["correction_signals"].update(session.correction_signals)
        totals["failure_signals"].update(session.failure_signals)
        totals["verification_signals"].update(session.verification_signals)
        totals["tool_calls"].update(session.tool_calls)
        totals["privacy_gaps"].update(session.privacy_gaps)
    return totals


def session_to_json(session: SessionSummary) -> dict[str, Any]:
    item = asdict(session)
    item["title"] = truncate(redact_text(item["title"])) if item["title"] else None
    return item


def read_jsonl(path: Path) -> Iterable[Any]:
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for line in handle:
                item = parse_jsonl_line(line)
                if item is not None:
                    yield item
    except OSError:
        return


def read_jsonl_sampled(path: Path) -> Iterable[Any]:
    try:
        if path.stat().st_size <= MAX_FULL_SESSION_BYTES:
            yield from read_jsonl(path)
            return
    except OSError:
        return

    tail: deque[str] = deque(maxlen=SESSION_TAIL_RECORDS)
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for line_number, line in enumerate(handle):
                if line_number < SESSION_HEAD_RECORDS:
                    item = parse_jsonl_line(line)
                    if item is not None:
                        yield item
                    continue
                tail.append(line)
    except OSError:
        return

    for line in tail:
        item = parse_jsonl_line(line)
        if item is not None:
            yield item


def parse_jsonl_line(line: str) -> Any | None:
    line = line.strip()
    if not line:
        return None
    try:
        return json.loads(line)
    except json.JSONDecodeError:
        return None


def read_text_limited(path: Path, limit: int = 250_000) -> str:
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            return handle.read(limit)
    except OSError:
        return ""


def extract_strings(value: Any, depth: int = 0) -> Iterable[str]:
    if depth > 8:
        return
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for child in value.values():
            yield from extract_strings(child, depth + 1)
    elif isinstance(value, list):
        for child in value:
            yield from extract_strings(child, depth + 1)


def first_string(item: Any, keys: tuple[str, ...]) -> str | None:
    if not isinstance(item, dict):
        return None
    for key in keys:
        value = item.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    for value in item.values():
        if isinstance(value, dict):
            found = first_string(value, keys)
            if found:
                return found
    return None


def collect_tool_calls(item: Any, counter: Counter[str]) -> None:
    if isinstance(item, dict):
        item_type = str(item.get("type", ""))
        name = item.get("name") or item.get("tool_name")
        if isinstance(name, str) and (
            item_type in {"function_call", "tool_use", "tool_call"}
            or "call" in item_type
            or "tool" in item_type
        ):
            counter[redact_text(name)] += 1
        for value in item.values():
            collect_tool_calls(value, counter)
    elif isinstance(item, list):
        for value in item:
            collect_tool_calls(value, counter)


def count_keywords(strings: list[str], keywords: list[str]) -> Counter[str]:
    counter: Counter[str] = Counter()
    lowered = [text.lower() for text in strings]
    for keyword in keywords:
        normalized = keyword.strip().lower()
        if not normalized:
            continue
        count = sum(text.count(normalized) for text in lowered)
        if count:
            counter[redact_text(keyword)] = count
    return counter


def count_patterns(strings: list[str], patterns: dict[str, re.Pattern[str]]) -> Counter[str]:
    counter: Counter[str] = Counter()
    for text in strings:
        for name, pattern in patterns.items():
            if pattern.search(text):
                counter[name] += 1
    return counter


def count_privacy_gaps(strings: list[str]) -> Counter[str]:
    counter: Counter[str] = Counter()
    for text in strings:
        if EMAIL_RE.search(text):
            counter["email"] += 1
        if API_KEY_RE.search(text) or GENERIC_SECRET_RE.search(text):
            counter["api-key-or-secret"] += 1
        if HEX_64_RE.search(text):
            counter["private-key-or-tx-hash"] += 1
        if EVM_ADDRESS_RE.search(text):
            counter["evm-address"] += 1
        if LONG_SECRETISH_RE.search(text):
            counter["long-secret-like-token"] += 1
    return counter


def redact_text(value: str | None) -> str:
    if not value:
        return ""
    text = EMAIL_RE.sub("<email>", value)
    text = API_KEY_RE.sub("<api-key>", text)
    text = GENERIC_SECRET_RE.sub(lambda match: match.group(0).split(match.group(1), 1)[0] + "<secret>", text)
    text = HEX_64_RE.sub("<tx-or-key-hash>", text)
    text = EVM_ADDRESS_RE.sub("<evm-address>", text)
    text = LONG_SECRETISH_RE.sub("<secret-like-token>", text)
    return text


def signal_text(value: str, limit: int = 4_000) -> str:
    if len(value) <= limit:
        return value
    half = limit // 2
    return f"{value[:half]}\n{value[-half:]}"


def truncate(value: str, limit: int = 160) -> str:
    if len(value) <= limit:
        return value
    return value[: limit - 1].rstrip() + "..."


def claude_config_dir() -> Path:
    config_dir = os.environ.get("CLAUDE_CONFIG_DIR") or os.environ.get("CLAUDE_HOME") or "~/.claude"
    return Path(os.path.expanduser(config_dir)).resolve()


def claude_project_keys(project: Path) -> list[str]:
    return [encode_claude_project(project), legacy_encode_claude_project(project)]


def encode_claude_project(project: Path) -> str:
    return re.sub(r"[^A-Za-z0-9]", "-", str(project))


def legacy_encode_claude_project(project: Path) -> str:
    return str(project).replace("/", "-")


def mtime_sort_key(path: Path) -> float:
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def title_for_codex_path(path: Path, codex_index: dict[str, Any]) -> str | None:
    titles_by_path = codex_index.get("titles_by_path", {})
    title = titles_by_path.get(str(path))
    if title:
        return title
    titles_by_id = codex_index.get("titles_by_id", {})
    for session_id, candidate_title in titles_by_id.items():
        if session_id and session_id in path.name:
            return candidate_title
    return None


def print_text_report(report: dict[str, Any]) -> None:
    print("transcript-miner")
    print("\nProjects:")
    for project in report["projects"]:
        coverage = project["coverage"]
        print(
            f"- {project['path']}: "
            f"{coverage['codex_candidates']} Codex, "
            f"{coverage['claude_candidates']} Claude, "
            f"{coverage['selected_sessions']} selected"
        )

    if report["task_themes"]:
        print("\nTask themes:")
        for name, count in sorted(report["task_themes"].items(), key=lambda item: (-item[1], item[0])):
            print(f"- {name}: {count}")

    print("\nCandidate sessions:")
    if not report["candidate_sessions"]:
        print("- none")
    for session in report["candidate_sessions"]:
        title = f" — {session['title']}" if session.get("title") else ""
        print(f"- {session['source']} score={session['score']} {session['path']}{title}")
        compact = compact_session_line(session)
        if compact:
            print(f"  {compact}")

    print_counter_block("Correction signals", report["correction_signals"])
    print_counter_block("Failure signals", report["failure_signals"])
    print_counter_block("Verification signals", report["verification_signals"])
    print_counter_block("Tool calls", report["tool_calls"])
    print_counter_block("Privacy gaps", report["privacy_gaps"])


def compact_session_line(session: dict[str, Any]) -> str:
    parts = []
    for label, key in (
        ("keywords", "keyword_hits"),
        ("themes", "task_themes"),
        ("failures", "failure_signals"),
        ("verification", "verification_signals"),
    ):
        values = session.get(key) or {}
        if values:
            parts.append(f"{label}: {', '.join(sorted(values)[:4])}")
    return "; ".join(parts)


def print_counter_block(title: str, values: dict[str, int]) -> None:
    print(f"\n{title}:")
    if not values:
        print("- none")
        return
    for name, count in sorted(values.items(), key=lambda item: (-item[1], item[0])):
        print(f"- {name}: {count}")


if __name__ == "__main__":
    raise SystemExit(main())
