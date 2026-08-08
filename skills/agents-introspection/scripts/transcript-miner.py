#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Mine project-owned Codex and Claude Code transcripts, optionally with redacted excerpts."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
from collections import Counter, defaultdict
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Iterable

from transcript_common import (
    CORRECTION_PATTERNS,
    THEME_PATTERNS,
    VERIFICATION_PATTERNS,
    count_keywords,
    count_patterns,
    count_privacy_gaps,
    deduplicate_messages,
    extract_record_channels,
    extract_strings,
    first_string_shallow,
    is_within,
    keyword_alternatives,
    normalize_path,
    read_jsonl,
    read_jsonl_head,
    read_jsonl_sampled,
    read_raw_lines_sampled,
    redact_text,
    source_title,
    truncate,
)


DATE_YEAR_RE = re.compile(r"^\d{4}$")
DATE_COMPONENT_RE = re.compile(r"^\d{2}$")
SINCE_DAYS_RE = re.compile(r"^(\d+)d$")


@dataclass
class Ownership:
    matched_via: str
    cwd: str
    project: str


@dataclass
class SignalChannels:
    eligible_user_messages: int = 0
    eligible_assistant_messages: int = 0
    ignored_context_messages: int = 0
    structured_tool_failures: int = 0


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
    ownership: Ownership | None = None
    signal_channels: SignalChannels = field(default_factory=SignalChannels)
    modified: str | None = None
    excerpts: list[dict[str, str]] = field(default_factory=list)


@dataclass
class SinceFilter:
    value: str
    cutoff: dt.datetime
    codex_dirs_pruned: int = 0
    codex_files_pruned: int = 0
    claude_files_pruned: int = 0

    def to_json(self) -> dict[str, Any]:
        return {
            "value": self.value,
            "cutoff": format_utc_iso(self.cutoff),
            "codex_dirs_pruned": self.codex_dirs_pruned,
            "codex_files_pruned": self.codex_files_pruned,
            "claude_files_pruned": self.claude_files_pruned,
        }


def main() -> int:
    parser = argparse.ArgumentParser(description="Mine project-owned Codex and Claude Code transcript signals.")
    parser.add_argument("--project", action="append", default=[], help="Project path to mine. Repeatable. Default: pwd -P")
    parser.add_argument(
        "--keyword",
        action="append",
        default=[],
        help="Task keyword to score. Repeatable. Use 'a|b|c' for an OR-group of alternatives",
    )
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--max-sessions", type=int, default=20, help="Maximum selected sessions per project")
    parser.add_argument("--include-archived", action="store_true", help="Include ~/.codex/archived_sessions")
    parser.add_argument(
        "--include-current",
        action="store_true",
        help="Include the live CODEX_THREAD_ID or CLAUDE_SESSION_ID transcript (diagnostics only)",
    )
    parser.add_argument("--since", default=None, help="Only mine sessions modified since YYYY-MM-DD or Nd (days back)")
    parser.add_argument(
        "--excerpts", action="store_true", help="Include up to 3 redacted message excerpts per candidate session"
    )
    args = parser.parse_args()

    if args.max_sessions < 1:
        print("transcript-miner: --max-sessions must be positive", file=sys.stderr)
        return 2
    try:
        projects = normalize_projects(args.project)
    except ValueError as error:
        print(f"transcript-miner: {error}", file=sys.stderr)
        return 2

    since_filter: SinceFilter | None = None
    if args.since:
        try:
            since_filter = build_since_filter(args.since)
        except ValueError as error:
            print(f"transcript-miner: {error}", file=sys.stderr)
            return 2

    report = mine_transcripts(
        projects,
        keywords=[keyword for keyword in args.keyword if keyword.strip()],
        max_sessions=args.max_sessions,
        include_archived=args.include_archived,
        include_current=args.include_current,
        since=since_filter,
        include_excerpts=args.excerpts,
    )
    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_text_report(report)
    return 0


def normalize_projects(raw_projects: list[str]) -> list[Path]:
    projects: list[Path] = []
    for raw_project in raw_projects or [os.curdir]:
        project = Path(os.path.expanduser(raw_project)).resolve(strict=False)
        if not project.exists():
            raise ValueError(f"project does not exist: {project}")
        if not project.is_dir():
            raise ValueError(f"project is not a directory: {project}")
        if project not in projects:
            projects.append(project)
    return projects


def build_since_filter(value: str) -> SinceFilter:
    match = SINCE_DAYS_RE.fullmatch(value)
    if match:
        cutoff = dt.datetime.now(dt.timezone.utc) - dt.timedelta(days=int(match.group(1)))
        return SinceFilter(value=value, cutoff=cutoff)
    try:
        parsed = dt.datetime.strptime(value, "%Y-%m-%d")
    except ValueError:
        raise ValueError(f"invalid --since value: {value!r} (expected YYYY-MM-DD or Nd)") from None
    return SinceFilter(value=value, cutoff=parsed.replace(tzinfo=dt.timezone.utc))


def mine_transcripts(
    projects: list[Path],
    *,
    keywords: list[str],
    max_sessions: int,
    include_archived: bool,
    include_current: bool = False,
    since: SinceFilter | None = None,
    include_excerpts: bool = False,
) -> dict[str, Any]:
    codex_home = Path(os.path.expanduser(os.environ.get("CODEX_HOME", "~/.codex"))).resolve(strict=False)
    claude_home = claude_config_dir()
    codex_index = load_codex_index(codex_home / "session_index.jsonl")
    claude_history = load_claude_history(claude_home / "history.jsonl")
    coverage = {project: new_coverage(codex_index["records"]) for project in projects}
    now = dt.datetime.now(dt.timezone.utc)

    codex_sessions = mine_codex_sessions(
        projects,
        keywords,
        codex_home,
        codex_index,
        include_archived,
        include_current,
        coverage,
        since,
        now,
        include_excerpts,
    )
    claude_sessions = mine_claude_sessions(
        projects, keywords, claude_home, claude_history, include_current, coverage, since, now, include_excerpts
    )

    selected_sessions: list[SessionSummary] = []
    for project in projects:
        project_sessions = [
            session for session in [*codex_sessions, *claude_sessions] if session.project == str(project)
        ]
        selected = select_sessions(project_sessions, max_sessions)
        selected_sessions.extend(selected)
        coverage[project]["selected_sessions"] = len(selected)

    project_reports = [
        {
            "path": str(project),
            "encoded_claude_project": encode_claude_project(project),
            "coverage": coverage[project],
        }
        for project in projects
    ]
    totals = aggregate_sessions(selected_sessions)
    return {
        "projects": project_reports,
        "keywords": [redact_text(keyword) for keyword in keywords],
        "since": since.to_json() if since else None,
        "candidate_sessions": [session_to_json(session) for session in selected_sessions],
        "task_themes": dict(totals["task_themes"]),
        "correction_signals": dict(totals["correction_signals"]),
        "failure_signals": dict(totals["failure_signals"]),
        "verification_signals": dict(totals["verification_signals"]),
        "tool_calls": dict(totals["tool_calls"]),
        "privacy_gaps": dict(totals["privacy_gaps"]),
    }


def new_coverage(codex_index_records: int) -> dict[str, Any]:
    return {
        "codex_candidates": 0,
        "claude_candidates": 0,
        "selected_sessions": 0,
        "claude_tool_result_files": 0,
        "claude_tool_result_failures": {},
        "codex_index_records": codex_index_records,
        "codex_scanned": 0,
        "claude_scanned": 0,
        "structurally_matched": 0,
        "relevance_matched": 0,
        "current_sessions_excluded": 0,
        "content_only_project_mentions_ignored": 0,
        "ambiguous_ownership_excluded": 0,
    }


def mine_codex_sessions(
    projects: list[Path],
    keywords: list[str],
    codex_home: Path,
    codex_index: dict[str, Any],
    include_archived: bool,
    include_current: bool,
    coverage: dict[Path, dict[str, Any]],
    since: SinceFilter | None,
    now: dt.datetime,
    include_excerpts: bool,
) -> list[SessionSummary]:
    roots = [codex_home / "sessions"]
    if include_archived:
        roots.append(codex_home / "archived_sessions")
    paths = collect_codex_paths(roots, since)
    for project in projects:
        coverage[project]["codex_scanned"] = len(paths)

    sessions: list[SessionSummary] = []
    current_id = os.environ.get("CODEX_THREAD_ID", "").strip()
    for path in paths:
        metadata = list(read_jsonl_head(path))
        ownership, ambiguous_projects = codex_ownership(metadata, path, projects)
        mark_ambiguous(coverage, ambiguous_projects)
        if ownership is None:
            continue
        owner = Path(ownership.project)
        if current_id and current_id in codex_session_ids(metadata, path) and not include_current:
            coverage[owner]["current_sessions_excluded"] += 1
            continue
        coverage[owner]["codex_candidates"] += 1
        coverage[owner]["structurally_matched"] += 1
        title_hint = title_for_codex_path(path, codex_index)
        if raw_prescan_eligible(keywords) and not title_already_matches(title_hint, keywords):
            raw_lines = list(read_raw_lines_sampled(path))
            mark_content_only_mentions_raw(raw_lines, owner, projects, coverage)
            if not keywords_present_in_lines(raw_lines, keywords):
                continue
            records = list(read_jsonl_sampled(path))
        else:
            records = list(read_jsonl_sampled(path))
            mark_content_only_mentions(records, owner, projects, coverage)
        summary = summarize_records(
            path,
            source="codex",
            keywords=keywords,
            ownership=ownership,
            records=records,
            title_hint=title_hint,
            now=now,
            include_excerpts=include_excerpts,
        )
        if summary is not None:
            coverage[owner]["relevance_matched"] += 1
            sessions.append(summary)
    return sessions


def collect_codex_paths(roots: list[Path], since: SinceFilter | None) -> list[Path]:
    if since is None:
        paths = [path for root in roots if root.is_dir() for path in root.rglob("*.jsonl")]
        return sorted(paths, key=str)
    slack_date = since.cutoff.date() - dt.timedelta(days=1)
    paths: list[Path] = []
    for root in roots:
        if root.is_dir():
            paths.extend(walk_codex_root(root, slack_date, since))
    kept = [path for path in paths if not prune_if_stale(path, since, "codex_files_pruned")]
    return sorted(kept, key=str)


def walk_codex_root(root: Path, slack_date: dt.date, since: SinceFilter) -> Iterable[Path]:
    yield from root.glob("*.jsonl")
    for year_dir in root.iterdir():
        if not (year_dir.is_dir() and DATE_YEAR_RE.fullmatch(year_dir.name)):
            if year_dir.is_dir():
                yield from year_dir.rglob("*.jsonl")
            continue
        yield from year_dir.glob("*.jsonl")
        for month_dir in year_dir.iterdir():
            if not (month_dir.is_dir() and DATE_COMPONENT_RE.fullmatch(month_dir.name)):
                if month_dir.is_dir():
                    yield from month_dir.rglob("*.jsonl")
                continue
            yield from month_dir.glob("*.jsonl")
            for day_dir in month_dir.iterdir():
                if not (day_dir.is_dir() and DATE_COMPONENT_RE.fullmatch(day_dir.name)):
                    if day_dir.is_dir():
                        yield from day_dir.rglob("*.jsonl")
                    continue
                try:
                    dir_date = dt.date(int(year_dir.name), int(month_dir.name), int(day_dir.name))
                except ValueError:
                    yield from day_dir.rglob("*.jsonl")
                    continue
                if dir_date < slack_date:
                    # An old date directory can still hold sessions resumed recently; mtime decides.
                    survivors = [
                        path
                        for path in day_dir.rglob("*.jsonl")
                        if not prune_if_stale(path, since, "codex_files_pruned")
                    ]
                    if survivors:
                        yield from survivors
                    else:
                        since.codex_dirs_pruned += 1
                    continue
                yield from day_dir.rglob("*.jsonl")


def prune_if_stale(path: Path, since: SinceFilter, counter: str) -> bool:
    try:
        mtime = path.stat().st_mtime
    except OSError:
        return False
    if mtime < since.cutoff.timestamp():
        setattr(since, counter, getattr(since, counter) + 1)
        return True
    return False


def raw_prescan_eligible(keywords: list[str]) -> bool:
    if not keywords:
        return False
    for keyword in keywords:
        alternatives = keyword_alternatives(keyword)
        if not alternatives:
            return False
        for alternative in alternatives:
            if not alternative.isascii() or '"' in alternative or "\\" in alternative:
                return False
    return True


def title_already_matches(title: str | None, keywords: list[str]) -> bool:
    if not title or not keywords:
        return False
    return bool(count_keywords([title], keywords))


def keywords_present_in_lines(lines: list[str], keywords: list[str]) -> bool:
    alternatives = [alternative.lower() for keyword in keywords for alternative in keyword_alternatives(keyword)]
    if not alternatives:
        return False
    for line in lines:
        lowered = line.lower()
        if any(alternative in lowered for alternative in alternatives):
            return True
    return False


def codex_ownership(
    metadata: list[Any], path: Path, projects: list[Path]
) -> tuple[Ownership | None, set[Path]]:
    session_cwds: list[Path] = []
    turn_cwds: list[Path] = []
    for item in metadata:
        if not isinstance(item, dict):
            continue
        payload = item.get("payload") if isinstance(item.get("payload"), dict) else {}
        item_type = item.get("type")
        cwd = payload.get("cwd")
        if item_type == "session_meta" and isinstance(cwd, str) and cwd.strip():
            session_cwds.append(normalize_path(cwd))
        elif item_type == "turn_context" and isinstance(cwd, str) and cwd.strip():
            turn_cwds.append(normalize_path(cwd))

    if session_cwds:
        unique = set(session_cwds)
        owners = {owner for cwd in unique if (owner := most_specific_project(cwd, projects)) is not None}
        if len(unique) != 1 or len(owners) > 1:
            return None, owners
        cwd = session_cwds[0]
        owner = most_specific_project(cwd, projects)
        if owner is None:
            return None, set()
        return Ownership("session_meta.payload.cwd", str(cwd), str(owner)), set()

    if not turn_cwds:
        sampled = list(read_jsonl_sampled(path))
        for item in sampled:
            if not isinstance(item, dict) or item.get("type") != "turn_context":
                continue
            payload = item.get("payload")
            cwd = payload.get("cwd") if isinstance(payload, dict) else None
            if isinstance(cwd, str) and cwd.strip():
                turn_cwds.append(normalize_path(cwd))
    owners = {owner for cwd in turn_cwds if (owner := most_specific_project(cwd, projects)) is not None}
    if not turn_cwds or len(owners) != 1 or any(most_specific_project(cwd, projects) not in owners for cwd in turn_cwds):
        return None, owners
    owner = next(iter(owners))
    canonical_cwds = set(turn_cwds)
    if any(not is_within(cwd, owner) for cwd in canonical_cwds):
        return None, owners
    return Ownership("turn_context.cwd", str(sorted(canonical_cwds, key=str)[0]), str(owner)), set()


def codex_session_ids(metadata: list[Any], path: Path) -> set[str]:
    ids = {path.stem}
    for item in metadata:
        if isinstance(item, dict) and item.get("type") == "session_meta":
            payload = item.get("payload")
            if isinstance(payload, dict):
                for key in ("id", "session_id", "conversation_id"):
                    value = payload.get(key)
                    if isinstance(value, str):
                        ids.add(value)
    return ids


def load_claude_history(path: Path) -> dict[str, list[dict[str, str]]]:
    records: dict[str, list[dict[str, str]]] = defaultdict(list)
    for item in read_jsonl(path):
        if not isinstance(item, dict):
            continue
        session_id = item.get("sessionId") or item.get("session_id")
        project = item.get("project")
        display = item.get("display")
        if not isinstance(session_id, str) or not isinstance(project, str):
            continue
        records[session_id].append(
            {
                "project": str(normalize_path(project)),
                "display": display.strip() if isinstance(display, str) else "",
            }
        )
    return records


def mine_claude_sessions(
    projects: list[Path],
    keywords: list[str],
    claude_home: Path,
    history: dict[str, list[dict[str, str]]],
    include_current: bool,
    coverage: dict[Path, dict[str, Any]],
    since: SinceFilter | None,
    now: dt.datetime,
    include_excerpts: bool,
) -> list[SessionSummary]:
    directory_projects: dict[Path, set[Path]] = defaultdict(set)
    for project in projects:
        for key in claude_project_keys(project):
            project_dir = claude_home / "projects" / key
            if project_dir.is_dir():
                directory_projects[project_dir.resolve(strict=False)].add(project)

    path_directory_projects: dict[Path, set[Path]] = defaultdict(set)
    for project_dir, possible_projects in directory_projects.items():
        paths = list(project_dir.glob("*.jsonl"))
        if since is not None:
            paths = [path for path in paths if not prune_if_stale(path, since, "claude_files_pruned")]
        for project in possible_projects:
            coverage[project]["claude_scanned"] += len(paths)
            tool_results_dir = project_dir / "tool-results"
            if tool_results_dir.is_dir():
                coverage[project]["claude_tool_result_files"] += sum(1 for path in tool_results_dir.rglob("*") if path.is_file())
        for path in paths:
            path_directory_projects[path.resolve(strict=False)].update(possible_projects)

    sessions: list[SessionSummary] = []
    current_id = os.environ.get("CLAUDE_SESSION_ID", "").strip()
    for path, directory_candidates in sorted(path_directory_projects.items(), key=lambda item: str(item[0])):
        metadata = list(read_jsonl_head(path))
        session_id = claude_session_id(metadata, path)
        history_records = history.get(session_id, [])
        ownership, ambiguous_projects = claude_ownership(
            metadata, directory_candidates, history_records, projects
        )
        if ownership is None:
            mark_ambiguous(coverage, ambiguous_projects or directory_candidates)
            continue
        owner = Path(ownership.project)
        if current_id and session_id == current_id and not include_current:
            coverage[owner]["current_sessions_excluded"] += 1
            continue
        coverage[owner]["claude_candidates"] += 1
        coverage[owner]["structurally_matched"] += 1

        history_messages = deduplicate_messages(
            record["display"] for record in history_records if record["display"]
        )
        if keywords and history_records and not count_keywords(history_messages, keywords):
            continue
        if not history_records and raw_prescan_eligible(keywords):
            raw_lines = list(read_raw_lines_sampled(path))
            mark_content_only_mentions_raw(raw_lines, owner, projects, coverage)
            if not keywords_present_in_lines(raw_lines, keywords):
                continue
            records = list(read_jsonl_sampled(path))
        else:
            records = list(read_jsonl_sampled(path))
            mark_content_only_mentions(records, owner, projects, coverage)
        summary = summarize_records(
            path,
            source="claude",
            keywords=keywords,
            ownership=ownership,
            records=records,
            title_hint=None,
            preferred_user_messages=history_messages if history_records else None,
            now=now,
            include_excerpts=include_excerpts,
        )
        if summary is not None:
            coverage[owner]["relevance_matched"] += 1
            sessions.append(summary)
    return sessions


def claude_ownership(
    metadata: list[Any],
    directory_candidates: set[Path],
    history_records: list[dict[str, str]],
    projects: list[Path],
) -> tuple[Ownership | None, set[Path]]:
    cwd_values: set[Path] = set()
    for item in metadata:
        if isinstance(item, dict):
            cwd = item.get("cwd")
            if isinstance(cwd, str) and cwd.strip():
                cwd_values.add(normalize_path(cwd))
    cwd_owners = {owner for cwd in cwd_values if (owner := most_specific_project(cwd, projects)) is not None}
    history_paths = {normalize_path(record["project"]) for record in history_records}
    history_owners = {
        owner for project_path in history_paths if (owner := most_specific_project(project_path, projects)) is not None
    }
    evidence = [set(directory_candidates)]
    if cwd_values:
        evidence.append(cwd_owners)
    if history_records:
        evidence.append(history_owners)
    implicated = set().union(*evidence)
    if any(len(values) != 1 for values in evidence):
        return None, implicated
    owners = set().union(*evidence)
    if len(owners) != 1:
        return None, implicated
    owner = next(iter(owners))
    if cwd_values:
        cwd = sorted(cwd_values, key=str)[0]
        matched_via = "transcript.cwd+encoded-directory"
    else:
        cwd = owner
        matched_via = "encoded-directory"
    if history_records:
        matched_via += "+history.project"
    return Ownership(matched_via, str(cwd), str(owner)), set()


def claude_session_id(metadata: list[Any], path: Path) -> str:
    for item in metadata:
        if not isinstance(item, dict):
            continue
        for key in ("sessionId", "session_id", "conversation_id"):
            value = item.get(key)
            if isinstance(value, str) and value.strip():
                return value.strip()
    return path.stem


def summarize_records(
    path: Path,
    *,
    source: str,
    keywords: list[str],
    ownership: Ownership,
    records: list[Any],
    title_hint: str | None,
    now: dt.datetime,
    include_excerpts: bool,
    preferred_user_messages: list[str] | None = None,
) -> SessionSummary | None:
    user_messages: list[str] = []
    assistant_messages: list[str] = []
    context_messages: list[str] = []
    tool_calls: Counter[str] = Counter()
    structured_failures = 0
    timestamp: str | None = None
    title = title_hint
    for item in records:
        if timestamp is None:
            timestamp = first_string_shallow(item, ("timestamp", "created_at", "updated_at"))
        if title is None:
            title = source_title(item)
        channels = extract_record_channels(item)
        user_messages.extend(channels["user"])
        assistant_messages.extend(channels["assistant"])
        context_messages.extend(channels["context"])
        tool_calls.update(channels["tools"])
        structured_failures += channels["failures"]

    if preferred_user_messages is not None:
        user_messages = preferred_user_messages
    user_messages = deduplicate_messages(user_messages)
    assistant_messages = deduplicate_messages(assistant_messages)
    context_messages = deduplicate_messages(context_messages)
    title = truncate(redact_text(title)) if title else None

    user_hits = count_keywords(user_messages, keywords)
    assistant_hits = count_keywords(assistant_messages, keywords)
    title_hits = count_keywords([title] if title else [], keywords)
    if keywords and not (user_hits or assistant_hits or title_hits):
        return None
    keyword_hits = user_hits + assistant_hits + title_hits
    corrections = count_patterns(user_messages, CORRECTION_PATTERNS)
    verification = count_patterns(assistant_messages, VERIFICATION_PATTERNS)
    eligible_messages = [*user_messages, *assistant_messages]
    themes = count_patterns(eligible_messages, THEME_PATTERNS)
    privacy = count_privacy_gaps(eligible_messages)
    failures = Counter({"command-failure": structured_failures}) if structured_failures else Counter()

    modified_dt = file_modified_time(path)
    age_source = parse_candidate_timestamp(timestamp) or modified_dt
    bonus = recency_bonus(now, age_source) if age_source is not None else 0

    score = (
        10
        + sum(user_hits.values()) * 8
        + sum(title_hits.values()) * 4
        + sum(assistant_hits.values()) * 2
        + min(6, sum(corrections.values()) * 2)
        + min(4, structured_failures)
        + min(4, sum(verification.values()))
        + bonus
    )
    excerpts = build_excerpts(user_messages, assistant_messages, keywords, include_excerpts)
    return SessionSummary(
        source=source,
        project=ownership.project,
        path=str(path),
        timestamp=timestamp,
        title=title,
        score=score,
        keyword_hits=dict(keyword_hits),
        task_themes=dict(themes),
        correction_signals=dict(corrections),
        failure_signals=dict(failures),
        verification_signals=dict(verification),
        tool_calls=dict(tool_calls),
        privacy_gaps=dict(privacy),
        ownership=ownership,
        signal_channels=SignalChannels(
            eligible_user_messages=len(user_messages),
            eligible_assistant_messages=len(assistant_messages),
            ignored_context_messages=len(context_messages),
            structured_tool_failures=structured_failures,
        ),
        modified=format_utc_iso(modified_dt) if modified_dt is not None else None,
        excerpts=excerpts,
    )


def file_modified_time(path: Path) -> dt.datetime | None:
    try:
        mtime = path.stat().st_mtime
    except OSError:
        return None
    return dt.datetime.fromtimestamp(mtime, tz=dt.timezone.utc)


def parse_candidate_timestamp(value: str | None) -> dt.datetime | None:
    if not value:
        return None
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        parsed = dt.datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=dt.timezone.utc)
    return parsed.astimezone(dt.timezone.utc)


def recency_bonus(now: dt.datetime, candidate: dt.datetime) -> int:
    age_days = (now - candidate).total_seconds() / 86400
    if age_days <= 7:
        return 6
    if age_days <= 30:
        return 3
    return 0


def format_utc_iso(value: dt.datetime) -> str:
    return value.astimezone(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def build_excerpts(
    user_messages: list[str], assistant_messages: list[str], keywords: list[str], include_excerpts: bool
) -> list[dict[str, str]]:
    if not include_excerpts:
        return []
    excerpts: list[dict[str, str]] = []
    seen: set[str] = set()
    if user_messages:
        first = user_messages[0]
        excerpts.append({"channel": "user", "text": truncate(redact_text(first), 240)})
        seen.add(" ".join(first.split()))
    if keywords:
        alternatives = [alternative.lower() for keyword in keywords for alternative in keyword_alternatives(keyword)]
        for channel, messages in (("user", user_messages), ("assistant", assistant_messages)):
            for message in messages:
                if len(excerpts) >= 3:
                    return excerpts
                normalized = " ".join(message.split())
                if normalized in seen:
                    continue
                if any(alternative in message.lower() for alternative in alternatives):
                    excerpts.append({"channel": channel, "text": truncate(redact_text(message), 240)})
                    seen.add(normalized)
    return excerpts


def mark_content_only_mentions(
    records: list[Any], owner: Path, projects: list[Path], coverage: dict[Path, dict[str, Any]]
) -> None:
    strings = list(extract_strings(records))
    for project in projects:
        if project != owner and any(str(project) in text for text in strings):
            coverage[project]["content_only_project_mentions_ignored"] += 1


def mark_content_only_mentions_raw(
    lines: list[str], owner: Path, projects: list[Path], coverage: dict[Path, dict[str, Any]]
) -> None:
    for project in projects:
        if project != owner and any(str(project) in line for line in lines):
            coverage[project]["content_only_project_mentions_ignored"] += 1


def mark_ambiguous(coverage: dict[Path, dict[str, Any]], projects: Iterable[Path]) -> None:
    for project in set(projects):
        coverage[project]["ambiguous_ownership_excluded"] += 1


def most_specific_project(cwd: Path, projects: list[Path]) -> Path | None:
    matches = [project for project in projects if is_within(cwd, project)]
    return max(matches, key=lambda project: len(project.parts), default=None)


def load_codex_index(path: Path) -> dict[str, Any]:
    titles_by_id: dict[str, str] = {}
    titles_by_path: dict[str, str] = {}
    records = 0
    for item in read_jsonl(path):
        records += 1
        session_id = first_string_shallow(item, ("id", "session_id", "conversation_id"))
        transcript_path = first_string_shallow(item, ("path", "file", "rollout_path", "transcript_path"))
        title = first_string_shallow(item, ("title", "summary", "task", "prompt"))
        if title and session_id:
            titles_by_id[session_id] = redact_text(title)
        if title and transcript_path:
            titles_by_path[str(normalize_path(transcript_path))] = redact_text(title)
    return {"records": records, "titles_by_id": titles_by_id, "titles_by_path": titles_by_path}


def title_for_codex_path(path: Path, codex_index: dict[str, Any]) -> str | None:
    title = codex_index["titles_by_path"].get(str(path.resolve(strict=False)))
    if title:
        return title
    for session_id, candidate_title in codex_index["titles_by_id"].items():
        if session_id in path.name:
            return candidate_title
    return None


def claude_config_dir() -> Path:
    config_dir = os.environ.get("CLAUDE_CONFIG_DIR") or os.environ.get("CLAUDE_HOME") or "~/.claude"
    return Path(os.path.expanduser(config_dir)).resolve(strict=False)


def claude_project_keys(project: Path) -> list[str]:
    return list(dict.fromkeys([encode_claude_project(project), legacy_encode_claude_project(project)]))


def encode_claude_project(project: Path) -> str:
    return re.sub(r"[^A-Za-z0-9]", "-", str(project))


def legacy_encode_claude_project(project: Path) -> str:
    return str(project).replace("/", "-")


def select_sessions(sessions: list[SessionSummary], max_sessions: int) -> list[SessionSummary]:
    return sorted(sessions, key=lambda session: (session.score, session.timestamp or "", session.path), reverse=True)[
        :max_sessions
    ]


def aggregate_sessions(sessions: list[SessionSummary]) -> dict[str, Counter[str]]:
    totals: dict[str, Counter[str]] = defaultdict(Counter)
    for session in sessions:
        for key in (
            "task_themes",
            "correction_signals",
            "failure_signals",
            "verification_signals",
            "tool_calls",
            "privacy_gaps",
        ):
            totals[key].update(getattr(session, key))
    return totals


def session_to_json(session: SessionSummary) -> dict[str, Any]:
    return asdict(session)


def print_text_report(report: dict[str, Any]) -> None:
    print("transcript-miner")
    since = report.get("since")
    if since:
        print(
            f"\nSince: {since['value']} (cutoff {since['cutoff']}) — "
            f"codex_dirs_pruned={since['codex_dirs_pruned']}, codex_files_pruned={since['codex_files_pruned']}, "
            f"claude_files_pruned={since['claude_files_pruned']}"
        )
    print("\nProjects:")
    for project in report["projects"]:
        coverage = project["coverage"]
        print(
            f"- {project['path']}: {coverage['codex_scanned']} Codex scanned, "
            f"{coverage['claude_scanned']} Claude scanned, {coverage['structurally_matched']} structurally owned, "
            f"{coverage['relevance_matched']} relevant, {coverage['selected_sessions']} selected"
        )
        print(
            "  exclusions: "
            f"current={coverage['current_sessions_excluded']}, "
            f"content-only={coverage['content_only_project_mentions_ignored']}, "
            f"ambiguous={coverage['ambiguous_ownership_excluded']}"
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
        ownership = session["ownership"]
        channels = session["signal_channels"]
        print(
            f"  ownership: {ownership['matched_via']} cwd={ownership['cwd']} project={ownership['project']}; "
            f"channels: user={channels['eligible_user_messages']}, "
            f"assistant={channels['eligible_assistant_messages']}, "
            f"context-ignored={channels['ignored_context_messages']}, "
            f"tool-failures={channels['structured_tool_failures']}"
        )
        compact = compact_session_line(session)
        if compact:
            print(f"  {compact}")
        for excerpt in session.get("excerpts") or []:
            print(f"  excerpt[{excerpt['channel']}]: {excerpt['text']}")
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
