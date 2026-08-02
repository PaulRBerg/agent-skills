from __future__ import annotations

import json
from pathlib import Path

import pytest

from ai_coord.coordinator import Coordinator
from ai_coord.identity import Identity
from ai_coord.migration import migrate_legacy
from ai_coord.providers import StaticInventory
from ai_coord.store import Store


def _write(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload))


def test_migrate_legacy_records_idempotently(tmp_path: Path, git_repo: Path) -> None:
    source = tmp_path / "legacy"
    session = "legacy-session"
    timestamp = "2026-08-02T10:00:00Z"
    _write(
        source / "session.json",
        {
            "schema_version": 1,
            "session_id": session,
            "turn_id": "turn",
            "cwd": str(git_repo),
            "state": "idle",
            "started_at": timestamp,
            "updated_at": timestamp,
            "pid": 123,
            "process_start_fingerprint": "fingerprint",
        },
    )
    _write(
        source / "claims" / "claim.json",
        {
            "schema_version": 2,
            "session_id": session,
            "client": "codex",
            "cwd": str(git_repo),
            "repo_root": str(git_repo),
            "label": "legacy work",
            "paths": ["src"],
            "created_at": timestamp,
        },
    )
    _write(
        source / "notes" / "notes.json",
        {
            "schema_version": 1,
            "repo_root": str(git_repo),
            "notes": [{"id": "note1234", "text": "finding", "created_at": timestamp}],
        },
    )
    _write(
        source / "inbox" / "inbox.json",
        {
            "schema_version": 1,
            "client": "codex",
            "session_id": session,
            "messages": [
                {
                    "id": "message1",
                    "from_client": "claude",
                    "from_session_id": "sender",
                    "text": "ready",
                    "created_at": timestamp,
                    "repo_root": str(git_repo),
                }
            ],
        },
    )
    store = Store(tmp_path / "state.db")
    report = migrate_legacy(store, source)
    assert report.as_dict() == {
        "scanned": 4,
        "imported": 4,
        "skipped": 0,
        "invalid": 0,
        "sessions": 1,
        "claims": 1,
        "messages": 1,
        "notes": 1,
    }
    claim = store.claim(Identity("codex", session))
    assert claim is not None
    assert claim["paths"] == ("src",)
    assert store.inbox(Identity("codex", session))[0]["text"] == "ready"
    assert store.notes(str(git_repo))[0]["text"] == "finding"

    second = migrate_legacy(store, source)
    assert second.imported == 0
    assert second.skipped == 4


def test_migration_dry_run_and_legacy_glob_blocker(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    source = tmp_path / "legacy"
    _write(
        source / "claims" / "claim.json",
        {
            "session_id": "legacy",
            "client": "claude",
            "cwd": str(git_repo),
            "repo_root": str(git_repo),
            "label": "glob",
            "paths": ["src/**"],
            "created_at": "2026-08-02T10:00:00Z",
        },
    )
    store = Store(tmp_path / "state.db")
    preview = migrate_legacy(store, source, dry_run=True)
    assert preview.claims == 1
    assert store.claims() == []
    migrate_legacy(store, source)
    claim = store.claim(Identity("claude", "legacy"))
    assert claim is not None
    assert claim["state"] == "queued"
    assert claim["blocked_reason"] == "legacy-pattern"
    monkeypatch.setenv("AI_COORD_CLIENT", "codex")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "new-session")
    outcome = Coordinator(store, StaticInventory()).start("new work", ("docs",), cwd=git_repo)
    assert outcome.kind == "BLOCKED"


def test_migration_rejects_unknown_client(tmp_path: Path, git_repo: Path) -> None:
    source = tmp_path / "legacy"
    _write(
        source / "claims" / "claim.json",
        {
            "session_id": "legacy",
            "client": "unknown",
            "cwd": str(git_repo),
            "repo_root": str(git_repo),
            "label": "invalid",
            "paths": ["src"],
            "created_at": "2026-08-02T10:00:00Z",
        },
    )
    store = Store(tmp_path / "state.db")

    report = migrate_legacy(store, source)

    assert report.invalid == 1
    assert report.imported == 0
    assert store.claims() == []


def test_migration_rejects_non_finite_timestamp(tmp_path: Path, git_repo: Path) -> None:
    source = tmp_path / "legacy"
    _write(
        source / "claims" / "claim.json",
        {
            "session_id": "legacy",
            "client": "codex",
            "cwd": str(git_repo),
            "repo_root": str(git_repo),
            "label": "invalid",
            "paths": ["src"],
            "created_at": float("nan"),
        },
    )
    store = Store(tmp_path / "state.db")

    report = migrate_legacy(store, source)

    assert report.invalid == 1
    assert store.claims() == []
