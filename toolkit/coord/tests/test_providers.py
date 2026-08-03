from __future__ import annotations

from pathlib import Path

import pytest

from ai_coord import providers
from ai_coord.identity import Identity
from ai_coord.providers import HostInventory, ProviderReport, normalize_claude_sessions
from ai_coord.store import CODEX_ORPHAN_GRACE, Store


def test_host_inventory_reconciles_only_stale_dead_codex_pids(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    store = Store(tmp_path / "state.db")
    stale_dead = Identity("codex", "stale-dead")
    stale_live = Identity("codex", "stale-live")
    recent_dead = Identity("codex", "recent-dead")
    claude = Identity("claude", "claude-dead")
    for identity, pid, current in (
        (stale_dead, 101, 0),
        (stale_live, 102, 0),
        (recent_dead, 103, CODEX_ORPHAN_GRACE),
        (claude, 104, 0),
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
    checked: list[int] = []

    def process_exists(pid: int) -> bool:
        checked.append(pid)
        return pid != 101

    def codex_report(_inventory: HostInventory, _store: Store) -> ProviderReport:
        return ProviderReport("codex", True, "test")

    def collect_claude(
        _inventory: HostInventory,
    ) -> tuple[ProviderReport, list[dict[str, object]]]:
        return ProviderReport("claude", True, "test", enabled=False), []

    monkeypatch.setattr(providers, "now_ts", lambda: CODEX_ORPHAN_GRACE + 1)
    monkeypatch.setattr(providers, "_process_exists", process_exists)
    monkeypatch.setattr(HostInventory, "_codex_report", codex_report)
    monkeypatch.setattr(HostInventory, "_collect_claude", collect_claude)

    assert HostInventory().refresh(store).complete
    assert checked == [101, 102]
    assert store.session(stale_dead) is None
    assert store.session(stale_live) is not None
    assert store.session(recent_dead) is not None
    assert store.session(claude) is not None


def test_process_lookup_errors_are_conservative(monkeypatch: pytest.MonkeyPatch) -> None:
    def kill(pid: int, signal: int) -> None:
        assert signal == 0
        if pid == 101:
            raise ProcessLookupError
        raise PermissionError

    monkeypatch.setattr(providers.os, "kill", kill)

    assert not providers._process_exists(101)
    assert providers._process_exists(102)


def test_normalize_claude_sessions_reports_dropped_and_unknown() -> None:
    rows, dropped = normalize_claude_sessions(
        [
            {
                "sessionId": "one",
                "cwd": "/tmp",
                "state": "busy",
                "startedAt": 1_700_000_000_000,
                "pid": 42,
                "name": "worker",
            },
            {
                "id": "two",
                "cwd": "/tmp",
                "status": "novel",
                "startedAt": "2026-08-02T00:00:00Z",
            },
            {"id": "nan", "cwd": "/tmp", "status": "working", "startedAt": float("nan")},
            {"bad": True},
            {"id": "done", "cwd": "/tmp", "status": "completed", "startedAt": 1},
        ]
    )
    assert dropped == 2
    assert [row["state"] for row in rows] == ["working", "unknown"]
    assert rows[0]["pid"] == 42
