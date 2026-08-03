from __future__ import annotations

import os
import subprocess
import time
from collections.abc import Callable
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import pytest

import ai_coord.coordinator as coordinator_module
from ai_coord.coordinator import Coordinator
from ai_coord.identity import Identity, ProcessReference
from ai_coord.providers import InventoryResult, StaticInventory
from ai_coord.store import CODEX_ORPHAN_GRACE, Store


class _PruningInventory:
    def __init__(self, current: float, dead_codex_sessions: tuple[Identity, ...]) -> None:
        self.current = current
        self.dead_codex_sessions = dead_codex_sessions

    def refresh(self, store: Store) -> InventoryResult:
        store.prune(self.current, dead_codex_sessions=self.dead_codex_sessions)
        return StaticInventory().refresh(store)


class _CountingInventory:
    def __init__(self) -> None:
        self.calls = 0

    def refresh(self, store: Store) -> InventoryResult:
        self.calls += 1
        return StaticInventory().refresh(store)


class _FakeClock:
    def __init__(self) -> None:
        self.current = 0.0
        self.on_sleep: Callable[[], None] | None = None

    def monotonic(self) -> float:
        return self.current

    def sleep(self, seconds: float) -> None:
        self.current += seconds
        if self.on_sleep is not None:
            callback = self.on_sleep
            self.on_sleep = None
            callback()


def _coordinator(db_path: Path, complete: bool = True) -> Coordinator:
    return Coordinator(Store(db_path), StaticInventory(complete))


def _set_identity(monkeypatch: pytest.MonkeyPatch, session_id: str, client: str = "codex") -> None:
    monkeypatch.setenv("AI_COORD_CLIENT", client)
    monkeypatch.setenv("AI_COORD_SESSION_ID", session_id)


def _start_worker(
    db_path: str, repo: str, gate: str, session_id: str, label: str, scope: str
) -> tuple[str, int]:
    os.environ["AI_COORD_CLIENT"] = "codex"
    os.environ["AI_COORD_SESSION_ID"] = session_id
    while not Path(gate).exists():
        time.sleep(0.005)
    outcome = _coordinator(Path(db_path)).start(label, (scope,), cwd=Path(repo))
    return outcome.kind, outcome.code


def test_direct_environment_identity_precedes_process_ancestry(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "direct")
    monkeypatch.setattr(
        coordinator_module,
        "process_ancestors",
        lambda: pytest.fail("ancestry should not be inspected"),
    )

    assert coordinator.identity() == Identity("codex", "direct")


def test_ancestry_identity_prefers_an_exact_fingerprint_over_legacy_pid(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    for key in (
        "AI_COORD_CLIENT",
        "AI_COORD_SESSION_ID",
        "CODEX_THREAD_ID",
        "CLAUDE_CODE_SESSION_ID",
    ):
        monkeypatch.delenv(key, raising=False)
    legacy = Identity("codex", "legacy")
    exact = Identity("codex", "exact")
    for identity, started_at in ((legacy, None), (exact, 10.0)):
        coordinator.store.upsert_session(
            identity,
            cwd="/repo",
            repo_root="/repo",
            state="working",
            source="test",
            pid=42,
            process_started_at=started_at,
        )
    monkeypatch.setattr(
        coordinator_module,
        "process_ancestors",
        lambda: (ProcessReference(42, 10.0),),
    )

    assert coordinator.identity() == exact


def test_start_intent_and_idempotent_active(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "session-a")
    assert coordinator.start("plan work", (), cwd=git_repo).kind == "INTENT"
    ready = coordinator.start("plan work", ("src",), cwd=git_repo)
    assert (ready.kind, ready.code, ready.paths) == ("READY", 0, ("src",))
    assert coordinator.start("plan work", ("src",), cwd=git_repo).kind == "READY"
    changed = coordinator.start("plan work", ("docs",), cwd=git_repo)
    assert changed.kind == "ACTIVE"
    assert changed.code == 3


def test_claim_cannot_move_between_repositories(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    other_repo = tmp_path / "other-repo"
    other_repo.mkdir()
    subprocess.run(["git", "init", "-b", "main"], cwd=other_repo, check=True, capture_output=True)
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "session-a")
    assert coordinator.start("first", ("src",), cwd=git_repo).kind == "READY"

    moved = coordinator.start("second", (), cwd=other_repo)

    assert moved.kind == "ACTIVE"
    identity = coordinator.identity()
    assert identity is not None
    claim = coordinator.store.claim(identity)
    session = coordinator.store.session(identity)
    assert claim is not None
    assert session is not None
    assert claim["repo_root"] == str(git_repo)
    assert claim["paths"] == ("src",)
    assert session["repo_root"] == str(git_repo)


def test_incomplete_coverage_and_unowned_dirt_fail_closed(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    _set_identity(monkeypatch, "session-a")
    incomplete = _coordinator(tmp_path / "incomplete.db", complete=False)
    outcome = incomplete.start("work", ("src",), cwd=git_repo)
    assert (outcome.kind, outcome.code, outcome.detail) == ("UNKNOWN", 2, "coverage")
    identity = incomplete.identity()
    assert identity is not None
    claim = incomplete.store.claim(identity)
    assert claim is not None
    assert claim["state"] == "queued"
    waited = incomplete.wait(timeout_seconds=2, poll_seconds=0.01)
    assert (waited.kind, waited.code) == ("UNKNOWN", 2)

    (git_repo / "src" / "app.py").write_text("changed = True\n")
    dirty = _coordinator(tmp_path / "dirty.db")
    outcome = dirty.start("work", ("src",), cwd=git_repo)
    assert outcome.kind == "UNKNOWN"
    assert outcome.code == 2
    assert outcome.detail == "dirty:src/app.py"


def test_blocked_claim_messages_holder_and_promotes_after_done(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "holder-session")
    assert coordinator.start("holder", ("src",), cwd=git_repo).kind == "READY"

    _set_identity(monkeypatch, "waiter-session")
    blocked = coordinator.start("waiter", ("src/app.py",), cwd=git_repo)
    assert blocked.kind == "BLOCKED"
    assert blocked.code == 3

    _set_identity(monkeypatch, "holder-session")
    assert len(coordinator.inbox()) == 1
    assert coordinator.done().kind == "DONE"

    _set_identity(monkeypatch, "waiter-session")
    assert coordinator.wait(timeout_seconds=2, poll_seconds=0.01).kind == "MESSAGE"
    assert coordinator.acknowledge(None) == 1
    promoted = coordinator.wait(timeout_seconds=2, poll_seconds=0.01)
    assert promoted.kind == "READY"
    identity = coordinator.identity()
    assert identity is not None
    claim = coordinator.store.claim(identity)
    assert claim is not None
    assert claim["state"] == "active"


@pytest.mark.parametrize("dirty", [False, True])
def test_wait_rechecks_dirt_after_orphaned_holder_is_pruned(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
    dirty: bool,
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    holder = Identity("codex", "holder-session")
    _set_identity(monkeypatch, holder.session_id)
    assert coordinator.start("holder", ("src",), cwd=git_repo).kind == "READY"

    _set_identity(monkeypatch, "waiter-session")
    assert coordinator.start("waiter", ("src/app.py",), cwd=git_repo).kind == "BLOCKED"
    coordinator.store.upsert_session(
        holder,
        cwd=str(git_repo),
        repo_root=str(git_repo),
        state="working",
        source="hook",
        pid=101,
        current=0,
    )
    if dirty:
        (git_repo / "src" / "app.py").write_text("changed = True\n")
    coordinator.inventory = _PruningInventory(CODEX_ORPHAN_GRACE + 1, (holder,))

    outcome = coordinator.wait(timeout_seconds=1, poll_seconds=0.01)

    assert coordinator.store.session(holder) is None
    if dirty:
        assert (outcome.kind, outcome.detail) == ("UNKNOWN", "dirty:src/app.py")
    else:
        assert outcome.kind == "READY"


def test_wait_uses_claim_repository_when_session_cwd_changes(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "holder")
    assert coordinator.start("holder", ("src",), cwd=git_repo).kind == "READY"

    _set_identity(monkeypatch, "waiter")
    assert coordinator.start("waiter", ("src",), cwd=git_repo).kind == "BLOCKED"
    coordinator.store.upsert_session(
        Identity("codex", "waiter"),
        cwd=str(tmp_path),
        repo_root=None,
        state="working",
        source="hook",
    )

    _set_identity(monkeypatch, "holder")
    coordinator.done()
    _set_identity(monkeypatch, "waiter")
    assert coordinator.acknowledge(None) == 1
    assert coordinator.wait(timeout_seconds=2, poll_seconds=0.01).kind == "READY"


def test_wait_runs_full_arbitration_only_on_slow_fallback_ticks(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    inventory = _CountingInventory()
    coordinator = Coordinator(Store(tmp_path / "state.db"), inventory)
    _set_identity(monkeypatch, "holder")
    assert coordinator.start("holder", ("src",), cwd=git_repo).kind == "READY"
    _set_identity(monkeypatch, "waiter")
    assert coordinator.start("waiter", ("src/app.py",), cwd=git_repo).kind == "BLOCKED"
    inventory.calls = 0
    clock = _FakeClock()
    monkeypatch.setattr(coordinator_module.time, "monotonic", clock.monotonic)
    monkeypatch.setattr(coordinator_module.time, "sleep", clock.sleep)

    outcome = coordinator.wait(timeout_seconds=40, poll_seconds=1)

    assert outcome.kind == "TIMEOUT"
    assert inventory.calls == 3


def test_wait_rechecks_immediately_when_generation_changes(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    path = tmp_path / "state.db"
    inventory = _CountingInventory()
    coordinator = Coordinator(Store(path), inventory)
    other_store = Store(path)
    _set_identity(monkeypatch, "holder")
    assert coordinator.start("holder", ("src",), cwd=git_repo).kind == "READY"
    _set_identity(monkeypatch, "waiter")
    assert coordinator.start("waiter", ("src/app.py",), cwd=git_repo).kind == "BLOCKED"
    inventory.calls = 0
    clock = _FakeClock()

    def send_wake_message() -> None:
        other_store.send_message(
            Identity("codex", "sender"),
            [Identity("codex", "waiter")],
            "wake now",
            str(git_repo),
        )

    clock.on_sleep = send_wake_message
    monkeypatch.setattr(coordinator_module.time, "monotonic", clock.monotonic)
    monkeypatch.setattr(coordinator_module.time, "sleep", clock.sleep)

    outcome = coordinator.wait(timeout_seconds=40, poll_seconds=1)

    assert (outcome.kind, outcome.detail) == ("MESSAGE", "1")
    assert inventory.calls == 2
    assert clock.current == 1


def test_done_notifies_only_overlapping_nonlegacy_waiters(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "holder")
    assert coordinator.start("holder", ("src",), cwd=git_repo).kind == "READY"
    _set_identity(monkeypatch, "overlap-waiter")
    assert coordinator.start("overlap", ("src/app.py",), cwd=git_repo).kind == "BLOCKED"
    _set_identity(monkeypatch, "docs-holder")
    assert coordinator.start("docs holder", ("docs",), cwd=git_repo).kind == "READY"
    _set_identity(monkeypatch, "docs-waiter")
    assert coordinator.start("docs waiter", ("docs/readme.md",), cwd=git_repo).kind == "BLOCKED"
    legacy = Identity("claude", "legacy-waiter")
    coordinator.store.upsert_session(
        legacy,
        cwd=str(git_repo),
        repo_root=str(git_repo),
        state="waiting",
        source="test",
    )
    with coordinator.store.transaction() as connection:
        coordinator.store.save_claim(
            connection,
            legacy,
            repo_root=str(git_repo),
            label="legacy",
            state="queued",
            paths=(),
            blocked_reason="legacy-pattern",
            created_at=0,
            updated_at=0,
        )

    _set_identity(monkeypatch, "holder")
    assert coordinator.done().detail == "released"

    overlap_messages = coordinator.store.inbox(Identity("codex", "overlap-waiter"))
    assert [message["text"] for message in overlap_messages] == [
        "released 'holder' — your queued claim may now be READY"
    ]
    assert coordinator.store.inbox(Identity("codex", "docs-waiter")) == []
    assert coordinator.store.inbox(legacy) == []
    assert coordinator.done().detail == "already clear"
    assert len(coordinator.store.inbox(Identity("codex", "overlap-waiter"))) == 1


def test_earlier_overlapping_waiter_reserves_scope(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "holder")
    assert coordinator.start("holder", ("src",), cwd=git_repo).kind == "READY"
    _set_identity(monkeypatch, "first-waiter")
    assert coordinator.start("first", ("src", "docs"), cwd=git_repo).kind == "BLOCKED"
    _set_identity(monkeypatch, "second-waiter")
    second = coordinator.start("second", ("docs",), cwd=git_repo)
    assert second.kind == "BLOCKED"
    assert second.holders == ("codex/first-wa",)


def test_messages_notes_status_and_trailer(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "sender-session")
    coordinator.start("sender", (), cwd=git_repo)
    _set_identity(monkeypatch, "target-session", client="claude")
    coordinator.start("target", (), cwd=git_repo)
    _set_identity(monkeypatch, "decoy-session")
    coordinator.start("target-session label", (), cwd=git_repo)
    _set_identity(monkeypatch, "sender-session")
    ids, count = coordinator.send("target-session", "ready to continue", cwd=git_repo)
    assert count == 1
    assert len(ids) == 1

    _set_identity(monkeypatch, "target-session", client="claude")
    assert coordinator.inbox()[0]["text"] == "ready to continue"
    assert coordinator.acknowledge(ids[0]) == 1
    note_id = coordinator.add_note("verified stale behavior", cwd=git_repo)
    snapshot = coordinator.snapshot(cwd=git_repo)
    assert snapshot.complete
    assert snapshot.notes[0]["id"] == note_id
    assert coordinator.resolve_note(note_id, cwd=git_repo)
    assert coordinator.trailer() == "Agent-Session: claude/target-session"


def test_machine_notes_and_repo_scoped_delegates(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    other_repo = tmp_path / "other-repo"
    other_repo.mkdir()
    subprocess.run(["git", "init", "-b", "main"], cwd=other_repo, check=True, capture_output=True)
    coordinator = _coordinator(tmp_path / "state.db")
    first = Identity("codex", "first-parent")
    second = Identity("claude", "second-parent")
    for process_started_at, (identity, root) in enumerate(
        ((first, git_repo), (second, other_repo)), start=1
    ):
        coordinator.store.upsert_session(
            identity,
            cwd=str(root),
            repo_root=str(root),
            state="working",
            source="test",
            pid=100 + process_started_at,
            process_started_at=float(process_started_at),
        )
    coordinator.store.update_delegate(first, "first-child", "explorer", "active")
    coordinator.store.update_delegate(second, "second-child", "explorer", "active")
    first_note = coordinator.store.add_note(first, str(git_repo), "first note")
    second_note = coordinator.store.add_note(second, str(other_repo), "second note")
    _set_identity(monkeypatch, first.session_id)

    repo_snapshot = coordinator.snapshot(cwd=git_repo)
    machine_snapshot = coordinator.snapshot(machine_wide=True, cwd=git_repo)

    assert [note["id"] for note in repo_snapshot.notes] == [first_note]
    assert [delegate["agent_id"] for delegate in repo_snapshot.delegates] == ["first-child"]
    assert {note["id"] for note in machine_snapshot.notes} == {first_note, second_note}
    assert {note["repo_root"] for note in machine_snapshot.notes} == {
        str(git_repo),
        str(other_repo),
    }
    assert {delegate["agent_id"] for delegate in machine_snapshot.delegates} == {
        "first-child",
        "second-child",
    }
    assert all("process_started_at" not in session for session in machine_snapshot.sessions)
    assert machine_snapshot.schema_version == 1
    rendered = coordinator.render_status(machine_snapshot)
    assert f"{git_repo}  {first_note}" in rendered
    assert f"{other_repo}  {second_note}" in rendered


def test_simultaneous_overlapping_claims_have_one_winner(tmp_path: Path, git_repo: Path) -> None:
    db_path = tmp_path / "state.db"
    Store(db_path).close()
    gate = tmp_path / "go"
    with ProcessPoolExecutor(max_workers=2) as executor:
        futures = [
            executor.submit(
                _start_worker,
                str(db_path),
                str(git_repo),
                str(gate),
                f"session-{index}",
                f"worker {index}",
                "src",
            )
            for index in range(2)
        ]
        gate.touch()
        results = [future.result(timeout=10) for future in futures]
    assert sorted(results) == [("BLOCKED", 3), ("READY", 0)]


def test_simultaneous_disjoint_claims_both_win(tmp_path: Path, git_repo: Path) -> None:
    db_path = tmp_path / "state.db"
    Store(db_path).close()
    gate = tmp_path / "go"
    with ProcessPoolExecutor(max_workers=2) as executor:
        futures = [
            executor.submit(
                _start_worker,
                str(db_path),
                str(git_repo),
                str(gate),
                f"session-{index}",
                f"worker {index}",
                scope,
            )
            for index, scope in enumerate(("src", "docs"))
        ]
        gate.touch()
        results = [future.result(timeout=10) for future in futures]
    assert results == [("READY", 0), ("READY", 0)]
