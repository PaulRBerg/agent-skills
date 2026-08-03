from __future__ import annotations

import sqlite3
import stat
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from threading import Barrier

from ai_coord.identity import Identity
from ai_coord.store import (
    CODEX_ORPHAN_GRACE,
    MAX_INBOX_MESSAGES,
    MESSAGE_TTL,
    SCHEMA_VERSION,
    Store,
)


def test_new_store_uses_schema_v2(tmp_path: Path) -> None:
    store = Store(tmp_path / "state.db")

    version = int(store.connection.execute("PRAGMA user_version").fetchone()[0])
    columns = {
        str(row["name"])
        for row in store.connection.execute("PRAGMA table_info(messages)").fetchall()
    }

    assert version == SCHEMA_VERSION == 2
    assert "notified_at" in columns


def test_store_migrates_schema_v1_to_v2(tmp_path: Path) -> None:
    path = tmp_path / "state.db"
    connection = sqlite3.connect(path)
    connection.execute(
        """
        CREATE TABLE messages (
            id TEXT PRIMARY KEY,
            sender_client TEXT NOT NULL,
            sender_session_id TEXT NOT NULL,
            recipient_client TEXT NOT NULL,
            recipient_session_id TEXT NOT NULL,
            repo_root TEXT,
            text TEXT NOT NULL,
            created_at REAL NOT NULL,
            acknowledged_at REAL
        )
        """
    )
    connection.execute("PRAGMA user_version = 1")
    connection.commit()
    connection.close()

    store = Store(path)

    version = int(store.connection.execute("PRAGMA user_version").fetchone()[0])
    columns = {
        str(row["name"])
        for row in store.connection.execute("PRAGMA table_info(messages)").fetchall()
    }
    assert version == 2
    assert "notified_at" in columns


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
        dead_codex_pids=(101, 102),
    )

    assert store.session(orphan) is None
    assert store.claim(orphan) is None
    assert store.delegates() == []
    assert store.session(recent) is not None
    assert store.session(unconfirmed) is not None
    assert store.generation() == generation + 1


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
