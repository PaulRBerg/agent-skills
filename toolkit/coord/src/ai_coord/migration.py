"""Idempotent importer for the legacy AgentSessionStatus JSON registry."""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

from ai_coord.identity import Identity
from ai_coord.store import Store
from ai_coord.util import git_root, now_ts, sanitize


@dataclass(frozen=True, slots=True)
class MigrationReport:
    scanned: int
    imported: int
    skipped: int
    invalid: int
    sessions: int
    claims: int
    messages: int
    notes: int

    def as_dict(self) -> dict[str, int]:
        return {
            "scanned": self.scanned,
            "imported": self.imported,
            "skipped": self.skipped,
            "invalid": self.invalid,
            "sessions": self.sessions,
            "claims": self.claims,
            "messages": self.messages,
            "notes": self.notes,
        }


def migrate_legacy(store: Store, source: Path, *, dry_run: bool = False) -> MigrationReport:
    counts = {
        "scanned": 0,
        "imported": 0,
        "skipped": 0,
        "invalid": 0,
        "sessions": 0,
        "claims": 0,
        "messages": 0,
        "notes": 0,
    }
    paths = sorted(source.glob("*.json"))
    paths += sorted((source / "claims").glob("*.json")) if (source / "claims").is_dir() else []
    paths += sorted((source / "notes").glob("*.json")) if (source / "notes").is_dir() else []
    paths += sorted((source / "inbox").glob("*.json")) if (source / "inbox").is_dir() else []
    for path in paths:
        counts["scanned"] += 1
        try:
            content = path.read_bytes()
        except OSError:
            counts["invalid"] += 1
            continue
        source_key = str(path.resolve())
        if store.imported(source_key, content):
            counts["skipped"] += 1
            continue
        try:
            payload = json.loads(content)
            kind, amount = _import_payload(store, source, path, payload, dry_run=dry_run)
        except (ValueError, TypeError, KeyError, json.JSONDecodeError):
            counts["invalid"] += 1
            continue
        counts[kind] += amount
        counts["imported"] += 1
        if not dry_run:
            store.mark_imported(source_key, content)
    return MigrationReport(**counts)


def _import_payload(
    store: Store,
    source: Path,
    path: Path,
    payload: Any,
    *,
    dry_run: bool,
) -> tuple[str, int]:
    if not isinstance(payload, dict):
        raise TypeError("record is not an object")
    parent = path.parent.name
    if path.parent == source:
        return "sessions", _import_session(store, payload, dry_run)
    if parent == "claims":
        return "claims", _import_claim(store, payload, dry_run)
    if parent == "notes":
        return "notes", _import_notes(store, payload, dry_run)
    if parent == "inbox":
        return "messages", _import_inbox(store, payload, dry_run)
    raise ValueError("unknown record kind")


def _import_session(store: Store, row: dict[str, Any], dry_run: bool) -> int:
    session_id = _string(row, "session_id")
    cwd = _string(row, "cwd")
    state = _string(row, "state")
    if state not in {"in_flight", "working", "idle"}:
        raise ValueError("invalid session state")
    root = git_root(Path(cwd))
    if not dry_run:
        store.upsert_session(
            Identity("codex", session_id),
            cwd=cwd,
            repo_root=str(root) if root else None,
            state="working" if state == "in_flight" else state,
            source="legacy",
            pid=row.get("pid") if isinstance(row.get("pid"), int) else None,
            started_at=_timestamp(row.get("started_at")),
            current=_timestamp(row.get("updated_at")),
        )
    return 1


def _import_claim(store: Store, row: dict[str, Any], dry_run: bool) -> int:
    client = _string(row, "client")
    session_id = _string(row, "session_id")
    label = sanitize(_string(row, "label"), 80)
    cwd = _string(row, "cwd")
    repo_root = row.get("repo_root")
    if not isinstance(repo_root, str) or not repo_root:
        root = git_root(Path(cwd))
        repo_root = str(root or Path(cwd).resolve())
    raw_paths = row.get("paths", [])
    if not isinstance(raw_paths, list) or any(not isinstance(value, str) for value in raw_paths):
        raise ValueError("invalid claim paths")
    paths = tuple(value for value in raw_paths if value)
    state = (
        "queued"
        if any(any(char in value for char in "*?[]") for value in paths)
        else ("active" if paths else "intent")
    )
    timestamp = _timestamp(row.get("created_at")) or now_ts()
    if not dry_run:
        identity = Identity(client, session_id)
        if store.session(identity) is None:
            store.upsert_session(
                identity,
                cwd=cwd,
                repo_root=repo_root,
                state="unknown",
                source="legacy",
                current=timestamp,
                started_at=timestamp,
            )
        with store.transaction() as connection:
            store.save_claim(
                connection,
                identity,
                repo_root=repo_root,
                label=label,
                state=state,
                paths=paths,
                blocked_reason="legacy-pattern" if state == "queued" else None,
                created_at=timestamp,
                updated_at=timestamp,
            )
    return 1


def _import_notes(store: Store, row: dict[str, Any], dry_run: bool) -> int:
    repo_root = _string(row, "repo_root")
    entries = row.get("notes")
    if not isinstance(entries, list):
        raise TypeError("invalid notes")
    imported = 0
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        note_id = entry.get("id")
        text = entry.get("text")
        created_at = _timestamp(entry.get("created_at"), required=False)
        if not isinstance(note_id, str) or not note_id or not isinstance(text, str) or not text:
            continue
        imported += 1
        if dry_run:
            continue
        store.connection.execute(
            """
            INSERT OR IGNORE INTO notes(
                id, repo_root, author_client, author_session_id, text, created_at
            ) VALUES (?, ?, ?, ?, ?, ?)
            """,
            (
                note_id,
                repo_root,
                entry.get("client") if isinstance(entry.get("client"), str) else None,
                entry.get("session_id") if isinstance(entry.get("session_id"), str) else None,
                sanitize(text, 240),
                created_at or now_ts(),
            ),
        )
    return imported


def _import_inbox(store: Store, row: dict[str, Any], dry_run: bool) -> int:
    client = _string(row, "client")
    session_id = _string(row, "session_id")
    entries = row.get("messages")
    if not isinstance(entries, list):
        raise TypeError("invalid inbox")
    imported = 0
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        required = ("id", "from_client", "from_session_id", "text", "created_at")
        if any(not isinstance(entry.get(key), str) or not entry[key] for key in required):
            continue
        imported += 1
        if dry_run:
            continue
        store.connection.execute(
            """
            INSERT OR IGNORE INTO messages(
                id, sender_client, sender_session_id, recipient_client,
                recipient_session_id, repo_root, text, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                entry["id"],
                entry["from_client"],
                entry["from_session_id"],
                client,
                session_id,
                entry.get("repo_root") if isinstance(entry.get("repo_root"), str) else None,
                sanitize(entry["text"], 240),
                _timestamp(entry["created_at"]),
            ),
        )
    return imported


def _string(row: dict[str, Any], key: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value:
        raise ValueError(f"missing {key}")
    return value


def _timestamp(value: Any, *, required: bool = True) -> float | None:
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return float(value)
    if isinstance(value, str) and value:
        try:
            return datetime.fromisoformat(value).timestamp()
        except ValueError:
            pass
    if required:
        raise ValueError("invalid timestamp")
    return None
