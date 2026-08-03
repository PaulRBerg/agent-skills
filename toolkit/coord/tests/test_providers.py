from __future__ import annotations

from pathlib import Path

import psutil
import pytest
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

from ai_coord import providers
from ai_coord.identity import Identity, ProcessReference
from ai_coord.providers import HostInventory, ProviderReport, normalize_claude_sessions
from ai_coord.store import CODEX_ORPHAN_GRACE, Store


@st.composite
def _claude_record(draw: st.DrawFn) -> tuple[dict[str, object], dict[str, object] | None, int]:
    kind = draw(st.sampled_from(("live", "terminal", "invalid-state", "invalid-timestamp")))
    session_id = draw(st.text(alphabet="abcdefghijklmnopqrstuvwxyz", min_size=1, max_size=12))
    cwd = draw(st.sampled_from(("/tmp", "/repo", "/workspace/project")))
    timestamp = draw(st.integers(min_value=-1_000_000, max_value=10_000_000_000))
    pid = draw(st.one_of(st.none(), st.booleans(), st.integers(min_value=-5, max_value=100)))
    raw: dict[str, object] = {
        "id": session_id,
        "cwd": cwd,
        "startedAt": timestamp,
        "pid": pid,
    }
    if kind == "invalid-state":
        raw["state"] = draw(st.one_of(st.none(), st.booleans(), st.integers()))
        return raw, None, 1
    if kind == "invalid-timestamp":
        raw["state"] = "working"
        raw["startedAt"] = draw(
            st.sampled_from((None, True, float("nan"), float("inf"), "", "not-a-time"))
        )
        return raw, None, 1
    if kind == "terminal":
        raw["state"] = draw(st.sampled_from(tuple(providers.CLAUDE_TERMINAL_STATES)))
        return raw, None, 0
    state = draw(st.sampled_from((*providers.CLAUDE_LIVE_STATES, "novel")))
    raw["state"] = state
    normalized_pid = pid if isinstance(pid, int) and not isinstance(pid, bool) and pid > 0 else None
    return (
        raw,
        {
            "session_id": session_id,
            "cwd": cwd,
            "state": providers.CLAUDE_LIVE_STATES.get(state, "unknown"),
            "pid": normalized_pid,
            "started_at": float(timestamp),
        },
        0,
    )


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


def test_process_liveness_rejects_missing_reused_zombie_and_dead_processes(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    processes = {
        102: _Process(102, started_at=2.0),
        103: _Process(103, status=psutil.STATUS_ZOMBIE),
        104: _Process(104, status=psutil.STATUS_DEAD),
    }

    def process(pid: int) -> _Process:
        if pid == 101:
            raise psutil.NoSuchProcess(pid)
        return processes[pid]

    monkeypatch.setattr(providers.psutil, "Process", process)

    assert not providers._process_exists(ProcessReference(101, 1.0))
    assert not providers._process_exists(ProcessReference(102, 1.0))
    assert not providers._process_exists(ProcessReference(103, 1.0))
    assert not providers._process_exists(ProcessReference(104, 1.0))


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


def test_normalize_claude_sessions_drops_unusable_working_directory() -> None:
    rows, dropped = normalize_claude_sessions(
        [{"id": "invalid-cwd", "cwd": "bad\0path", "state": "working", "startedAt": 1}]
    )

    assert rows == []
    assert dropped == 1


@settings(
    max_examples=100,
    suppress_health_check=[HealthCheck.function_scoped_fixture],
)
@given(records=st.lists(_claude_record(), max_size=10))
def test_normalize_claude_sessions_matches_generated_record_model(
    monkeypatch: pytest.MonkeyPatch,
    records: list[tuple[dict[str, object], dict[str, object] | None, int]],
) -> None:
    monkeypatch.setattr(providers, "git_root", lambda path: path)
    monkeypatch.setattr(
        providers,
        "process_reference",
        lambda pid: ProcessReference(pid, float(pid) + 0.5),
    )
    payload = [raw for raw, _, _ in records]
    expected_rows = [expected for _, expected, _ in records if expected is not None]

    rows, dropped = normalize_claude_sessions(payload)

    assert dropped == sum(invalid for _, _, invalid in records)
    assert len(rows) == len(expected_rows)
    for row, expected in zip(rows, expected_rows, strict=True):
        assert {key: row[key] for key in expected} == expected
        expected_pid = expected["pid"]
        assert row["process_started_at"] == (
            float(expected_pid) + 0.5 if isinstance(expected_pid, int) else None
        )
