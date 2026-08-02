from __future__ import annotations

import os
import subprocess
import time
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import pytest

from ai_coord.coordinator import Coordinator
from ai_coord.identity import Identity
from ai_coord.providers import StaticInventory
from ai_coord.store import Store


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
    promoted = coordinator.wait(timeout_seconds=2, poll_seconds=0.01)
    assert promoted.kind == "READY"
    identity = coordinator.identity()
    assert identity is not None
    claim = coordinator.store.claim(identity)
    assert claim is not None
    assert claim["state"] == "active"


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
    assert coordinator.wait(timeout_seconds=2, poll_seconds=0.01).kind == "READY"


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
