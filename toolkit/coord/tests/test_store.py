from __future__ import annotations

import stat
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from threading import Barrier

from ai_coord.identity import Identity
from ai_coord.store import MAX_INBOX_MESSAGES, MESSAGE_TTL, Store


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
