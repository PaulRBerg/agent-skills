from __future__ import annotations

from pathlib import Path
from threading import Event, get_ident

import psutil
import pytest
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

from ai_coord import providers
from ai_coord.identity import Identity, ProcessReference
from ai_coord.integrations import HookCheck, HooksCheck
from ai_coord.providers import (
    INVENTORY_CACHE_SECONDS,
    HostInventory,
    ProviderReport,
    normalize_claude_sessions,
)
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
        raw["state"] = draw(st.sampled_from(tuple(sorted(providers.CLAUDE_TERMINAL_STATES))))
        return raw, None, 0
    state = draw(st.sampled_from((*providers.CLAUDE_LIVE_STATES, "novel")))
    raw["state"] = state
    normalized_pid = pid if isinstance(pid, int) and not isinstance(pid, bool) and pid > 0 else None
    return (
        raw,
        {
            "session_id": session_id,
            "cwd": cwd,
            "repo_root": cwd,
            "state": providers.CLAUDE_LIVE_STATES.get(state, "unknown"),
            "name": None,
            "waiting_for": None,
            "pid": normalized_pid,
            "process_started_at": (
                float(normalized_pid) + 0.5 if normalized_pid is not None else None
            ),
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

    def codex_report(
        _inventory: HostInventory, _store: Store, _executable: str | None
    ) -> ProviderReport:
        return ProviderReport("codex", True, "test")

    def collect_claude(
        _inventory: HostInventory, _executable: str | None
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


def test_host_inventory_reuses_complete_cache_across_store_instances(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "state.db"
    current = 100.0
    calls: list[str] = []

    monkeypatch.setattr(providers, "now_ts", lambda: current)
    monkeypatch.setattr(providers.shutil, "which", lambda client: f"/bin/{client}")

    def codex_report(
        _inventory: HostInventory, _store: Store, _executable: str | None
    ) -> ProviderReport:
        calls.append("codex")
        return ProviderReport("codex", True, "test-codex")

    def collect_claude(
        _inventory: HostInventory, _executable: str | None
    ) -> tuple[ProviderReport, list[dict[str, object]]]:
        calls.append("claude")
        return ProviderReport("claude", True, "test-claude"), []

    monkeypatch.setattr(HostInventory, "_codex_report", codex_report)
    monkeypatch.setattr(HostInventory, "_collect_claude", collect_claude)
    first_store = Store(path)
    first = HostInventory().refresh(first_store, allow_cached=True)
    first_store.close()

    current += INVENTORY_CACHE_SECONDS / 2
    second = HostInventory().refresh(Store(path), allow_cached=True)

    assert calls.count("codex") == calls.count("claude") == 1
    assert second == first
    assert [report.source for report in second.providers] == ["test-codex", "test-claude"]


@pytest.mark.parametrize(
    ("second_time", "change_context"),
    ((99.0, False), (100.0 + INVENTORY_CACHE_SECONDS, False), (101.0, True)),
)
def test_host_inventory_reprobes_expired_backward_or_other_context_cache(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    second_time: float,
    change_context: bool,
) -> None:
    current = 100.0
    calls = 0

    monkeypatch.setattr(providers, "now_ts", lambda: current)
    monkeypatch.setattr(providers.shutil, "which", lambda client: f"/bin/{client}")

    def codex_report(
        _inventory: HostInventory, _store: Store, _executable: str | None
    ) -> ProviderReport:
        nonlocal calls
        calls += 1
        return ProviderReport("codex", True, "test")

    def collect_claude(
        _inventory: HostInventory, _executable: str | None
    ) -> tuple[ProviderReport, list[dict[str, object]]]:
        return ProviderReport("claude", True, "test", enabled=False), []

    monkeypatch.setattr(HostInventory, "_codex_report", codex_report)
    monkeypatch.setattr(HostInventory, "_collect_claude", collect_claude)
    store = Store(tmp_path / "state.db")
    HostInventory().refresh(store, allow_cached=True)

    current = second_time
    if change_context:
        monkeypatch.setenv("CODEX_HOME", str(tmp_path / "other-codex"))
    HostInventory().refresh(store, allow_cached=True)

    assert calls == 2


def test_host_inventory_clears_complete_cache_after_fresh_degradation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    store = Store(tmp_path / "state.db")
    degraded = False
    monkeypatch.setattr(providers, "now_ts", lambda: 100.0)
    monkeypatch.setattr(providers.shutil, "which", lambda client: f"/bin/{client}")

    def codex_report(
        _inventory: HostInventory, _store: Store, _executable: str | None
    ) -> ProviderReport:
        return ProviderReport(
            "codex",
            not degraded,
            "test",
            error="untrusted" if degraded else None,
        )

    def collect_claude(
        _inventory: HostInventory, _executable: str | None
    ) -> tuple[ProviderReport, list[dict[str, object]]]:
        return ProviderReport("claude", True, "test", enabled=False), []

    monkeypatch.setattr(HostInventory, "_codex_report", codex_report)
    monkeypatch.setattr(HostInventory, "_collect_claude", collect_claude)
    assert HostInventory().refresh(store, allow_cached=False).complete
    assert store.connection.execute("SELECT COUNT(*) FROM provider_cache").fetchone()[0] == 2

    degraded = True
    assert not HostInventory().refresh(store, allow_cached=False).complete

    assert store.connection.execute("SELECT COUNT(*) FROM provider_cache").fetchone()[0] == 0


def test_host_inventory_reprobes_cache_rows_that_are_not_complete(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    store = Store(tmp_path / "state.db")
    calls = 0
    monkeypatch.setattr(providers, "now_ts", lambda: 100.0)
    monkeypatch.setattr(providers.shutil, "which", lambda client: f"/bin/{client}")

    def codex_report(
        _inventory: HostInventory, _store: Store, _executable: str | None
    ) -> ProviderReport:
        nonlocal calls
        calls += 1
        return ProviderReport("codex", True, "test")

    def collect_claude(
        _inventory: HostInventory, _executable: str | None
    ) -> tuple[ProviderReport, list[dict[str, object]]]:
        return ProviderReport("claude", True, "test", enabled=False), []

    monkeypatch.setattr(HostInventory, "_codex_report", codex_report)
    monkeypatch.setattr(HostInventory, "_collect_claude", collect_claude)
    inventory = HostInventory()
    assert inventory.refresh(store, allow_cached=True).complete
    with store.transaction() as connection:
        connection.execute("UPDATE provider_cache SET ok = 0 WHERE client = 'codex'")

    assert inventory.refresh(store, allow_cached=True).complete
    assert calls == 2


def test_host_inventory_overlaps_probes_and_reconciles_on_calling_thread(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    store = Store(tmp_path / "state.db")
    calling_thread = get_ident()
    claude_started = Event()
    reconciliation_threads: list[int] = []
    original_replace = store.replace_claude_sessions

    monkeypatch.setattr(providers.shutil, "which", lambda client: f"/bin/{client}")

    def codex_report(
        _inventory: HostInventory, _store: Store, _executable: str | None
    ) -> ProviderReport:
        assert get_ident() == calling_thread
        assert claude_started.wait(timeout=1)
        return ProviderReport("codex", True, "test")

    def collect_claude(
        _inventory: HostInventory, _executable: str | None
    ) -> tuple[ProviderReport, list[dict[str, object]]]:
        assert get_ident() != calling_thread
        claude_started.set()
        return ProviderReport("claude", True, "test"), []

    def replace(rows: list[dict[str, object]], current: float) -> None:
        reconciliation_threads.append(get_ident())
        original_replace(rows, current)

    monkeypatch.setattr(HostInventory, "_codex_report", codex_report)
    monkeypatch.setattr(HostInventory, "_collect_claude", collect_claude)
    monkeypatch.setattr(store, "replace_claude_sessions", replace)

    assert HostInventory().refresh(store).complete
    assert reconciliation_threads == [calling_thread]


def test_codex_provider_reports_trusted_hook_inventory(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    hook_path = tmp_path / "hooks.json"
    store = Store(tmp_path / "state.db")
    inspected_paths: list[Path] = []

    monkeypatch.setattr(
        providers.shutil,
        "which",
        lambda name: "/bin/codex" if name == "codex" else None,
    )
    monkeypatch.setattr(
        providers,
        "inspect_hooks",
        lambda _client, _path: HooksCheck("codex", hook_path, True, ()),
    )

    def inspect_trust(path: Path) -> HookCheck:
        inspected_paths.append(path)
        return HookCheck(True, path, None, {})

    monkeypatch.setattr(providers, "inspect_codex_hook_trust", inspect_trust)

    report = HostInventory()._codex_report(store, "/bin/codex")

    assert report == ProviderReport("codex", True, "hook-ledger")
    assert inspected_paths == [hook_path]


def test_codex_provider_preserves_hook_file_failures(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    hook_path = tmp_path / "hooks.json"
    store = Store(tmp_path / "state.db")

    monkeypatch.setattr(providers.shutil, "which", lambda _name: "/bin/codex")
    monkeypatch.setattr(
        providers,
        "inspect_hooks",
        lambda _client, _path: HooksCheck(
            "codex", hook_path, False, ("SessionStart",), "invalid hook configuration"
        ),
    )
    monkeypatch.setattr(
        providers,
        "inspect_codex_hook_trust",
        lambda _path: pytest.fail("hook trust inspection should not run for invalid hook files"),
    )

    report = HostInventory()._codex_report(store, "/bin/codex")

    assert not report.ok
    assert report.error == "invalid hook configuration; missing or invalid hooks: SessionStart"


@pytest.mark.parametrize(
    ("trust_error", "expected"),
    (
        (
            "owned hook does not exactly match app-server configuration",
            "owned hook does not exactly match app-server configuration",
        ),
        ("Codex app-server unavailable", "Codex app-server unavailable"),
        (None, "app-server hook trust could not be verified"),
    ),
)
def test_codex_provider_fails_closed_when_hook_trust_is_degraded_or_unverifiable(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    trust_error: str | None,
    expected: str,
) -> None:
    hook_path = tmp_path / "hooks.json"
    store = Store(tmp_path / "state.db")

    monkeypatch.setattr(providers.shutil, "which", lambda _name: "/bin/codex")
    monkeypatch.setattr(
        providers,
        "inspect_hooks",
        lambda _client, _path: HooksCheck("codex", hook_path, True, ()),
    )
    monkeypatch.setattr(
        providers,
        "inspect_codex_hook_trust",
        lambda path: HookCheck(False, path, trust_error, {}),
    )

    report = HostInventory()._codex_report(store, "/bin/codex")

    assert not report.ok
    assert report.error == f"hook trust: {expected}"


def test_codex_provider_preserves_hook_health_failures(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    hook_path = tmp_path / "hooks.json"
    store = Store(tmp_path / "state.db")
    store.hook_error("codex", "SessionStart", "hook_error")

    monkeypatch.setattr(providers.shutil, "which", lambda _name: "/bin/codex")
    monkeypatch.setattr(
        providers,
        "inspect_hooks",
        lambda _client, _path: HooksCheck("codex", hook_path, True, ()),
    )
    monkeypatch.setattr(
        providers,
        "inspect_codex_hook_trust",
        lambda _path: pytest.fail("hook trust inspection should not run for unhealthy hooks"),
    )

    report = HostInventory()._codex_report(store, "/bin/codex")

    assert not report.ok
    assert report.error == "last hook error: hook_error"


def test_codex_provider_remains_disabled_without_codex(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    store = Store(tmp_path / "state.db")

    monkeypatch.setattr(providers.shutil, "which", lambda _name: None)
    monkeypatch.setattr(
        providers,
        "inspect_codex_hook_trust",
        lambda _path: pytest.fail("hook trust inspection should not run without Codex"),
    )

    assert HostInventory()._codex_report(store, None) == ProviderReport(
        "codex", True, "hook-ledger", enabled=False
    )


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
    assert rows == expected_rows
