from __future__ import annotations

import sqlite3
import stat
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from threading import Barrier

import pytest

from ai_coord.identity import Identity, ProcessReference
from ai_coord.schema import SchemaVersionError
from ai_coord.store import (
    CODEX_ORPHAN_GRACE,
    MAX_INBOX_MESSAGES,
    MESSAGE_TTL,
    SCHEMA_VERSION,
    Store,
)


def test_new_store_uses_schema_v8(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")

    version = int(store.connection.execute("PRAGMA user_version").fetchone()[0])
    message_columns = {
        str(row["name"])
        for row in store.connection.execute("PRAGMA table_info(messages)").fetchall()
    }
    session_columns = {
        str(row["name"])
        for row in store.connection.execute("PRAGMA table_info(sessions)").fetchall()
    }

    tables = {
        str(row["name"])
        for row in store.connection.execute(
            "SELECT name FROM sqlite_master WHERE type = 'table'"
        ).fetchall()
    }

    assert version == SCHEMA_VERSION == 8
    assert "notified_at" in message_columns
    assert {"sender_callsign", "recipient_callsign"} <= message_columns
    assert "process_started_at" in session_columns
    assert "callsign" in session_columns
    assert "permission_mode" in session_columns
    assert "imports" not in tables
    assert {
        "claim_baselines",
        "dirt_observations",
        "provider_cache",
        "residual_owners",
    } <= tables


@pytest.mark.parametrize("version", [1, SCHEMA_VERSION - 1, SCHEMA_VERSION + 1])
def test_store_rejects_incompatible_schema_without_modifying_it(
    tmp_path: Path, version: int
) -> None:
    path = tmp_path / "state.db"
    connection = sqlite3.connect(path)
    connection.execute("CREATE TABLE sentinel(value TEXT NOT NULL)")
    connection.execute("INSERT INTO sentinel VALUES ('preserved')")
    connection.execute(f"PRAGMA user_version = {version}")
    connection.commit()
    connection.close()

    with pytest.raises(SchemaVersionError) as raised:
        Store(path)

    assert (raised.value.found, raised.value.required, raised.value.path) == (
        version,
        SCHEMA_VERSION,
        path,
    )
    assert str(raised.value) == (
        f"state schema {version} is incompatible with required schema {SCHEMA_VERSION} at {path}; "
        "close all agents and explicitly replace the ledger before retrying"
    )
    connection = sqlite3.connect(path)
    assert int(connection.execute("PRAGMA user_version").fetchone()[0]) == version
    assert connection.execute("SELECT value FROM sentinel").fetchone()[0] == "preserved"
    assert connection.execute("PRAGMA journal_mode").fetchone()[0] == "delete"
    connection.close()


def test_callsigns_are_unique_machine_wide_and_idempotent(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")
    first = Identity("codex", "first")
    second = Identity("claude", "second")
    for identity in (first, second):
        store.upsert_session(
            identity,
            cwd="/repo",
            repo_root="/repo",
            state="working",
            source="test",
        )

    store.set_session_callsign(first, "✈️ Night Owl")
    generation = store.generation()
    store.set_session_callsign(first, "✈️ Night Owl")
    assert store.generation() == generation
    with pytest.raises(ValueError, match="already in use"):
        store.set_session_callsign(second, "✈ night owl")

    store.set_session_callsign(first, "🌙 Lunar One")
    store.set_session_callsign(second, "✈ night owl")
    assert store.session(second)["callsign"] == "✈ night owl"  # type: ignore[index]


def test_permission_mode_changes_alone_bump_generation(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")
    identity = Identity("codex", "mode-session")

    def upsert(permission_mode: str | None = None, *, update: bool = False) -> None:
        store.upsert_session(
            identity,
            cwd="/repo",
            repo_root="/repo",
            state="working",
            source="test",
            permission_mode=permission_mode,
            update_permission_mode=update,
        )

    upsert()
    generation = store.generation()

    upsert()
    assert store.generation() == generation

    upsert("plan", update=True)
    assert store.generation() == generation + 1
    generation = store.generation()

    upsert("plan", update=True)
    assert store.generation() == generation

    upsert()
    assert store.session(identity)["permission_mode"] == "plan"  # type: ignore[index]
    assert store.generation() == generation

    upsert(update=True)
    assert store.session(identity)["permission_mode"] is None  # type: ignore[index]
    assert store.generation() == generation + 1


def test_message_callsigns_are_snapshotted_across_rename_and_exit(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")
    sender = Identity("codex", "sender")
    recipient = Identity("claude", "recipient")
    for identity in (sender, recipient):
        store.upsert_session(
            identity,
            cwd="/repo",
            repo_root="/repo",
            state="working",
            source="test",
        )
    store.set_session_callsign(sender, "🦊 Fox One")
    store.set_session_callsign(recipient, "🐙 Octo Two")
    store.send_message(sender, [recipient], "before", "/repo", current=1)
    store.set_session_callsign(sender, "🦝 Raccoon One")
    store.set_session_callsign(recipient, "🦑 Squid Two")
    store.send_message(sender, [recipient], "after", "/repo", current=2)
    store.end_session(sender)
    store.end_session(recipient)

    messages = store.inbox(recipient)

    assert [
        (row["text"], row["sender_callsign"], row["recipient_callsign"]) for row in messages
    ] == [
        ("before", "🦊 Fox One", "🐙 Octo Two"),
        ("after", "🦝 Raccoon One", "🦑 Squid Two"),
    ]


def test_concurrent_callsign_claims_have_one_winner(tmp_path: Path) -> None:
    path = tmp_path / "state.db"
    Store(path).close()
    gate = Barrier(2)

    def claim_callsign(index: int) -> bool:
        store = Store(path)
        identity = Identity("codex", f"session-{index}")
        store.upsert_session(
            identity,
            cwd=f"/repo-{index}",
            repo_root=f"/repo-{index}",
            state="working",
            source="test",
        )
        gate.wait()
        try:
            store.set_session_callsign(identity, "🤖 Shared Bot")
        except ValueError:
            return False
        finally:
            store.close()
        return True

    with ThreadPoolExecutor(max_workers=2) as executor:
        results = list(executor.map(claim_callsign, range(2)))

    assert sorted(results) == [False, True]


def test_save_claim_atomically_replaces_scope_age_baselines_and_residuals(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")
    identity = Identity("codex", "owner")
    store.upsert_session(
        identity,
        cwd="/repo",
        repo_root="/repo",
        state="working",
        source="test",
    )
    store.observe_dirt("/repo", {"docs/readme.md": "dirty-oid"}, current=1)
    with store.transaction() as connection:
        store.save_claim(
            connection,
            identity,
            repo_root="/repo",
            label="broad",
            state="active",
            paths=("docs/readme.md", "src/app.py"),
            blocked_reason=None,
            created_at=1,
            updated_at=1,
            baselines={"docs/readme.md": "docs-oid", "src/app.py": "src-oid"},
        )

    with pytest.raises(RuntimeError, match="rollback"), store.transaction() as connection:
        store.save_claim(
            connection,
            identity,
            repo_root="/repo",
            label="rolled back",
            state="active",
            paths=("src/app.py",),
            blocked_reason=None,
            created_at=2,
            updated_at=2,
            baselines={"src/app.py": "new-src-oid"},
            residual_paths=("docs/readme.md",),
        )
        raise RuntimeError("rollback")

    claim = store.claim(identity)
    assert claim is not None
    assert (claim["label"], claim["paths"], claim["created_at"]) == (
        "broad",
        ("docs/readme.md", "src/app.py"),
        1,
    )
    assert store.baselines(identity) == [
        {"path": "docs/readme.md", "oid": "docs-oid"},
        {"path": "src/app.py", "oid": "src-oid"},
    ]
    assert store.residual_owners("/repo") == {}

    with store.transaction() as connection:
        store.save_claim(
            connection,
            identity,
            repo_root="/repo",
            label="narrow",
            state="active",
            paths=("src/app.py",),
            blocked_reason=None,
            created_at=3,
            updated_at=3,
            baselines={"src/app.py": "new-src-oid"},
            residual_paths=("docs/readme.md",),
        )

    claim = store.claim(identity)
    assert claim is not None
    assert (claim["label"], claim["paths"], claim["created_at"]) == (
        "narrow",
        ("src/app.py",),
        3,
    )
    assert store.baselines(identity) == [{"path": "src/app.py", "oid": "new-src-oid"}]
    residual = store.residual_owners("/repo")["docs/readme.md"]
    assert (residual["client"], residual["session_id"], residual["released_at"]) == (
        identity.client,
        identity.session_id,
        3,
    )


def test_store_permissions_and_message_cap(tmp_path: Path) -> None:
    path = tmp_path / "private" / "state.db"
    store = Store(path)
    assert stat.S_IMODE(path.stat().st_mode) == 0o600
    assert stat.S_IMODE(path.parent.stat().st_mode) == 0o700
    sender = Identity("codex", "sender")
    recipient = Identity("claude", "recipient")
    for index in range(MAX_INBOX_MESSAGES + 5):
        store.send_message(sender, [recipient], f"message {index}", None, current=float(index + 1))
    inbox = store.inbox(recipient)
    assert len(inbox) == MAX_INBOX_MESSAGES
    assert inbox[0]["text"] == "message 5"


def test_store_prunes_expired_messages(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")
    sender = Identity("codex", "sender")
    recipient = Identity("claude", "recipient")
    store.send_message(sender, [recipient], "old", None, current=1)
    store.prune(current=MESSAGE_TTL + 2)
    assert store.inbox(recipient) == []


def test_store_prunes_confirmed_orphans_after_grace(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")
    orphan = Identity("codex", "orphan")
    recent = Identity("codex", "recent")
    unconfirmed = Identity("codex", "unconfirmed")
    for identity, pid, current in (
        (orphan, 101, 0),
        (recent, 102, CODEX_ORPHAN_GRACE),
        (unconfirmed, 103, 0),
    ):
        store.upsert_session(
            identity,
            cwd="/repo",
            repo_root="/repo",
            state="working",
            source="test",
            pid=pid,
            current=current,
        )
    with store.transaction() as connection:
        store.save_claim(
            connection,
            orphan,
            repo_root="/repo",
            label="orphaned work",
            state="active",
            paths=("src",),
            blocked_reason=None,
            created_at=0,
            updated_at=0,
        )
    store.update_delegate(orphan, "child", "explorer", "active")
    generation = store.generation()

    store.prune(
        current=CODEX_ORPHAN_GRACE + 1,
        dead_codex_sessions=(orphan, recent),
    )

    assert store.session(orphan) is None
    assert store.claim(orphan) is None
    assert store.delegates() == []
    assert store.session(recent) is not None
    assert store.session(unconfirmed) is not None
    assert store.generation() == generation + 1


def test_store_prunes_only_the_exact_dead_session_when_pids_are_reused(
    tmp_path: Path,
) -> None:
    store = Store(tmp_path / "state.db")
    dead = Identity("codex", "dead")
    live = Identity("codex", "live")
    for identity, process_started_at in ((dead, 1.0), (live, 2.0)):
        store.upsert_session(
            identity,
            cwd="/repo",
            repo_root="/repo",
            state="working",
            source="test",
            pid=101,
            process_started_at=process_started_at,
            current=0,
        )

    store.prune(
        current=CODEX_ORPHAN_GRACE + 1,
        dead_codex_sessions=(dead,),
    )

    assert store.session(dead) is None
    assert store.session(live) is not None


def test_process_identity_prefers_exact_fingerprints_then_pid_only_records(
    tmp_path: Path,
) -> None:
    store = Store(tmp_path / "state.db")
    pid_only = Identity("codex", "pid-only")
    exact = Identity("codex", "exact")
    store.upsert_session(
        pid_only,
        cwd="/repo",
        repo_root="/repo",
        state="working",
        source="test",
        pid=42,
    )
    store.upsert_session(
        exact,
        cwd="/repo",
        repo_root="/repo",
        state="working",
        source="test",
        pid=42,
        process_started_at=10.0,
    )

    assert store.identities_for_processes((ProcessReference(42, 10.0),)) == [exact]
    assert store.identities_for_processes((ProcessReference(42, 11.0),)) == [pid_only]
    assert store.identities_for_processes((ProcessReference(42, None),)) == [pid_only]


def test_session_process_reference_is_replaced_as_an_atomic_pair(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")
    identity = Identity("codex", "session")
    store.upsert_session(
        identity,
        cwd="/repo",
        repo_root="/repo",
        state="working",
        source="test",
        pid=41,
        process_started_at=1.0,
    )
    store.upsert_session(
        identity,
        cwd="/repo",
        repo_root="/repo",
        state="working",
        source="hook",
        pid=42,
        process_started_at=2.0,
    )
    store.upsert_session(
        identity,
        cwd="/repo",
        repo_root="/repo",
        state="working",
        source="cli",
    )

    session = store.session(identity)
    assert session is not None
    assert (session["pid"], session["process_started_at"]) == (42, 2.0)


def test_claude_inventory_replaces_pid_and_creation_time_as_an_atomic_pair(
    tmp_path: Path,
) -> None:
    store = Store(tmp_path / "state.db")
    identity = Identity("claude", "session")
    base = {
        "session_id": identity.session_id,
        "cwd": "/repo",
        "repo_root": "/repo",
        "state": "working",
        "started_at": 1.0,
    }
    store.replace_claude_sessions([{**base, "pid": 41, "process_started_at": 1.0}], current=2.0)
    store.replace_claude_sessions([{**base, "pid": 42, "process_started_at": None}], current=3.0)

    session = store.session(identity)
    assert session is not None
    assert (session["pid"], session["process_started_at"]) == (42, None)


def test_store_honors_explicit_epoch_timestamps(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")
    sender = Identity("codex", "sender")
    recipient = Identity("claude", "recipient")
    store.upsert_session(
        sender,
        cwd="/tmp",
        repo_root=None,
        state="working",
        source="test",
        started_at=0,
        current=0,
    )
    store.send_message(sender, [recipient], "epoch", None, current=0)
    store.prune(current=0)

    session = store.session(sender)
    assert session is not None
    assert session["started_at"] == 0
    assert session["last_seen"] == 0
    assert store.inbox(recipient)[0]["created_at"] == 0


def test_store_initialization_is_concurrency_safe(tmp_path: Path) -> None:
    path = tmp_path / "state.db"
    gate = Barrier(8)

    def open_store(_: int) -> None:
        gate.wait()
        Store(path).close()

    with ThreadPoolExecutor(max_workers=8) as executor:
        list(executor.map(open_store, range(8)))

    store = Store(path)
    assert store.generation() == 0
