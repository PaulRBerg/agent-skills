from __future__ import annotations

import http.client
import json
from dataclasses import dataclass
from datetime import UTC, datetime
from threading import Thread
from typing import Any

from ai_coord.coordinator import Coordinator
from ai_coord.identity import Identity
from ai_coord.providers import StaticInventory
from ai_coord.server import SnapshotService, create_server, sse_snapshot_frame
from ai_coord.store import Store


def test_all_messages_empty_and_ordered(tmp_path: Any) -> None:
    store = Store(tmp_path / "state.db")
    assert store.all_messages() == []

    sender = Identity("codex", "sender")
    recipient = Identity("claude", "recipient")
    store.send_message(sender, [recipient], "later", "/repo", current=2)
    store.send_message(sender, [recipient], "first", "/repo", current=1)

    messages = store.all_messages()

    assert [message["text"] for message in messages] == ["first", "later"]
    assert set(messages[0]) == {
        "id",
        "sender_client",
        "sender_session_id",
        "recipient_client",
        "recipient_session_id",
        "repo_root",
        "text",
        "created_at",
        "acknowledged_at",
    }


def test_snapshot_includes_dashboard_fields(tmp_path: Any) -> None:
    state_path = tmp_path / "state.db"
    store = Store(state_path)
    store.send_message(
        Identity("codex", "sender"), [Identity("claude", "recipient")], "hello", "/repo"
    )
    store.close()
    service = SnapshotService(
        lambda: Coordinator(Store(state_path), StaticInventory()),
        utcnow=lambda: datetime(2026, 8, 4, tzinfo=UTC),
    )

    payload = service.snapshot()

    assert {"messages", "generated_at", "generation"} <= set(payload)
    assert payload["messages"][0]["text"] == "hello"
    assert payload["generated_at"] == "2026-08-04T00:00:00+00:00"
    assert isinstance(payload["generation"], int)


def test_sse_snapshot_frame_format() -> None:
    assert sse_snapshot_frame({"generation": 7}) == b'event: snapshot\ndata: {"generation":7}\n\n'


@dataclass
class _Snapshot:
    value: int

    def as_dict(self) -> dict[str, int]:
        return {"value": self.value}


class _Store:
    def __init__(self) -> None:
        self.generation_value = 3

    def all_messages(self) -> list[dict[str, str]]:
        return []

    def generation(self) -> int:
        return self.generation_value

    def close(self) -> None:
        pass


class _Coordinator:
    def __init__(self, store: _Store, calls: list[bool]) -> None:
        self.store = store
        self._calls = calls

    def snapshot(self, *, machine_wide: bool) -> _Snapshot:
        assert machine_wide
        self._calls.append(True)
        return _Snapshot(len(self._calls))


def test_snapshot_service_reuses_cache_within_rate_limit() -> None:
    current = 0.0
    calls: list[bool] = []
    store = _Store()
    service = SnapshotService(
        lambda: _Coordinator(store, calls),  # type: ignore[arg-type]
        monotonic=lambda: current,
    )

    first = service.snapshot()
    current = 1.9
    cached = service.snapshot()
    current = 2.0
    refreshed = service.snapshot()

    assert first is cached
    assert refreshed is not cached
    assert [payload["value"] for payload in (first, refreshed)] == [1, 2]
    assert len(calls) == 2


class _FixedService:
    def snapshot(self) -> dict[str, bool]:
        return {"ok": True}

    def generation(self) -> int:
        return 0


def test_unknown_path_returns_json_404() -> None:
    server = create_server("127.0.0.1", 0, _FixedService())  # type: ignore[arg-type]
    thread = Thread(target=server.handle_request)
    thread.start()
    connection = http.client.HTTPConnection("127.0.0.1", server.server_port)
    connection.request("GET", "/missing")
    response = connection.getresponse()
    body = json.loads(response.read())
    connection.close()
    thread.join()
    server.server_close()

    assert response.status == 404
    assert body == {"error": "not found"}
