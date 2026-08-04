from __future__ import annotations

import json
from pathlib import Path

import pytest
from hypothesis import HealthCheck, example, given, settings
from hypothesis import strategies as st

from ai_coord.coordinator import Coordinator
from ai_coord.identity import Identity
from ai_coord.migration import _timestamp, migrate_legacy
from ai_coord.providers import StaticInventory
from ai_coord.store import Store


def _write(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload))


def _isolated_case(tmp_path: Path) -> Path:
    path = tmp_path / f"case-{sum(1 for _ in tmp_path.iterdir())}"
    path.mkdir()
    return path


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


def test_migration_dry_run_and_legacy_glob_does_not_block_literal_claims(
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
    assert outcome.kind == "READY"


def test_timestamp_treats_naive_legacy_iso_values_as_utc() -> None:
    naive = "2026-08-02T13:02:29.282"

    assert _timestamp(naive) == _timestamp(f"{naive}Z")


@settings(
    max_examples=25,
    deadline=None,
    suppress_health_check=[HealthCheck.function_scoped_fixture],
)
@example(client="unknown")
@given(
    client=st.one_of(
        st.none(),
        st.booleans(),
        st.integers(),
        st.text().filter(lambda value: value not in {"codex", "claude"}),
    )
)
def test_migration_rejects_invalid_generated_clients(
    tmp_path: Path, git_repo: Path, client: object
) -> None:
    case = _isolated_case(tmp_path)
    source = case / "legacy"
    _write(
        source / "claims" / "claim.json",
        {
            "session_id": "legacy",
            "client": client,
            "cwd": str(git_repo),
            "repo_root": str(git_repo),
            "label": "invalid",
            "paths": ["src"],
            "created_at": "2026-08-02T10:00:00Z",
        },
    )
    store = Store(case / "state.db")

    report = migrate_legacy(store, source)

    assert report.invalid == 1
    assert report.imported == 0
    assert store.claims() == []


@settings(
    max_examples=25,
    deadline=None,
    suppress_health_check=[HealthCheck.function_scoped_fixture],
)
@example(timestamp=float("nan"))
@given(
    timestamp=st.sampled_from(
        (None, True, False, float("nan"), float("inf"), float("-inf"), "", "invalid")
    )
)
def test_migration_rejects_invalid_generated_timestamps(
    tmp_path: Path, git_repo: Path, timestamp: object
) -> None:
    case = _isolated_case(tmp_path)
    source = case / "legacy"
    _write(
        source / "claims" / "claim.json",
        {
            "session_id": "legacy",
            "client": "codex",
            "cwd": str(git_repo),
            "repo_root": str(git_repo),
            "label": "invalid",
            "paths": ["src"],
            "created_at": timestamp,
        },
    )
    store = Store(case / "state.db")

    report = migrate_legacy(store, source)

    assert report.invalid == 1
    assert store.claims() == []
