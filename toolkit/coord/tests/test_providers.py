from __future__ import annotations

from pathlib import Path

import psutil
import pytest

from ai_coord import providers
from ai_coord.identity import Identity, ProcessReference
from ai_coord.providers import HostInventory, ProviderReport, normalize_claude_sessions
from ai_coord.store import CODEX_ORPHAN_GRACE, Store


def test_host_inventory_reconciles_only_stale_dead_codex_sessions(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    store = Store(tmp_path / "state.db")
    stale_dead = Identity("codex", "stale-dead")
    stale_live = Identity("codex", "stale-live")
    recent_dead = Identity("codex", "recent-dead")
    claude = Identity("claude", "claude-dead")
    for identity, pid, process_started_at, current in (
        (stale_dead, 101, 1.0, 0),
        (stale_live, 102, 2.0, 0),
        (recent_dead, 103, 3.0, CODEX_ORPHAN_GRACE),
        (claude, 104, 4.0, 0),
    ):
        store.upsert_session(
            identity,
            cwd="/repo",
            repo_root="/repo",
            state="working",
            source="test",
            pid=pid,
            process_started_at=process_started_at,
            current=current,
        )
    checked: list[ProcessReference] = []

    def process_exists(reference: ProcessReference) -> bool:
        checked.append(reference)
        return reference.pid != 101

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
    assert checked == [ProcessReference(101, 1.0), ProcessReference(102, 2.0)]
    assert store.session(stale_dead) is None
    assert store.session(stale_live) is not None
    assert store.session(recent_dead) is not None
    assert store.session(claude) is not None


class _Process:
    def __init__(
        self,
        pid: int,
        *,
        status: str = psutil.STATUS_RUNNING,
        started_at: float = 1.0,
        denied_status: bool = False,
        denied_creation: bool = False,
    ) -> None:
        self.pid = pid
        self._status = status
        self.started_at = started_at
        self.denied_status = denied_status
        self.denied_creation = denied_creation

    def status(self) -> str:
        if self.denied_status:
            raise psutil.AccessDenied(self.pid)
        return self._status

    def create_time(self) -> float:
        if self.denied_creation:
            raise psutil.AccessDenied(self.pid)
        return self.started_at


def test_process_liveness_rejects_missing_reused_and_zombie_processes(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    processes = {
        102: _Process(102, started_at=2.0),
        103: _Process(103, status=psutil.STATUS_ZOMBIE),
    }

    def process(pid: int) -> _Process:
        if pid == 101:
            raise psutil.NoSuchProcess(pid)
        return processes[pid]

    monkeypatch.setattr(providers.psutil, "Process", process)

    assert not providers._process_exists(ProcessReference(101, 1.0))
    assert not providers._process_exists(ProcessReference(102, 1.0))
    assert not providers._process_exists(ProcessReference(103, 1.0))


def test_process_liveness_is_conservative_when_details_are_unavailable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    processes = {
        101: _Process(101, denied_status=True),
        102: _Process(102, denied_creation=True),
        103: _Process(103),
    }
    monkeypatch.setattr(providers.psutil, "Process", processes.__getitem__)

    assert providers._process_exists(ProcessReference(101, 1.0))
    assert providers._process_exists(ProcessReference(102, 1.0))
    assert providers._process_exists(ProcessReference(103, None))


def test_normalize_claude_sessions_reports_dropped_unknown_and_process_fingerprint(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(
        providers,
        "process_reference",
        lambda pid: ProcessReference(pid, 42.5),
    )
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
    assert rows[0]["process_started_at"] == 42.5
