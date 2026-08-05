from __future__ import annotations

import os
import time
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import pytest

import ai_coord.coordinator as coordinator_module
from ai_coord.coordinator import Coordinator
from ai_coord.identity import Identity
from ai_coord.providers import StaticInventory
from ai_coord.store import Store


def _coordinator(db_path: Path, *, complete: bool = True) -> Coordinator:
    return Coordinator(Store(db_path), StaticInventory(complete))


def _set_identity(monkeypatch: pytest.MonkeyPatch, session_id: str) -> None:
    monkeypatch.setenv("AI_COORD_CLIENT", "codex")
    monkeypatch.setenv("AI_COORD_SESSION_ID", session_id)


def _replace_worker(
    db_path: str,
    repo: str,
    gate: str,
    session_id: str,
    original: str,
) -> tuple[str, str, int]:
    os.environ["AI_COORD_CLIENT"] = "codex"
    os.environ["AI_COORD_SESSION_ID"] = session_id
    while not Path(gate).exists():
        time.sleep(0.005)
    outcome = _coordinator(Path(db_path)).start(
        f"expand {session_id}", (original, "shared/new.py"), cwd=Path(repo)
    )
    return session_id, outcome.kind, outcome.code


def test_blocking_reports_only_real_overlap_and_nudges_broad_holder(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "holder")
    assert coordinator.start("ledger work", ("src",), cwd=git_repo).kind == "READY"
    _set_identity(monkeypatch, "waiter")

    blocked = coordinator.start("targeted edit", ("src/app.py", "docs/other.md"), cwd=git_repo)

    assert (blocked.kind, blocked.paths) == ("BLOCKED", ("src/app.py",))
    _set_identity(monkeypatch, "holder")
    assert [message["text"] for message in coordinator.inbox()] == [
        (
            "Narrow broad claim src with ai-coord start if unrelated; "
            "queued work 'targeted edit' overlaps: src/app.py."
        )
    ]


def test_active_refinement_replaces_scope_and_wakes_newly_unblocked_waiter(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    holder = Identity("codex", "holder")
    waiter = Identity("codex", "waiter")
    _set_identity(monkeypatch, holder.session_id)
    assert coordinator.start("broad work", ("src",), cwd=git_repo).kind == "READY"
    original = coordinator.store.claim(holder)
    assert original is not None
    _set_identity(monkeypatch, waiter.session_id)
    assert coordinator.start("other file", ("src/other.py",), cwd=git_repo).kind == "BLOCKED"

    _set_identity(monkeypatch, holder.session_id)
    narrowed = coordinator.start("app only", ("src/app.py",), cwd=git_repo)

    claim = coordinator.store.claim(holder)
    assert claim is not None
    assert (narrowed.kind, claim["label"], claim["paths"]) == (
        "READY",
        "app only",
        ("src/app.py",),
    )
    assert claim["created_at"] == original["created_at"]
    assert [message["text"] for message in coordinator.store.inbox(waiter)] == [
        "Narrowed claim 'broad work'; your queued claim may now be ready."
    ]


def test_active_expansion_failure_keeps_the_old_claim_atomic(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    owner = Identity("codex", "owner")
    _set_identity(monkeypatch, owner.session_id)
    assert coordinator.start("app", ("src/app.py",), cwd=git_repo).kind == "READY"
    original = coordinator.store.claim(owner)
    assert original is not None
    _set_identity(monkeypatch, "docs-holder")
    assert coordinator.start("docs", ("docs/readme.md",), cwd=git_repo).kind == "READY"

    _set_identity(monkeypatch, owner.session_id)
    blocked = coordinator.start("expanded", ("src/app.py", "docs/readme.md"), cwd=git_repo)

    assert blocked.kind == "ACTIVE"
    assert blocked.detail == "update-blocked:codex/docs-hol"
    assert coordinator.store.claim(owner) == original


def test_earlier_queued_scope_blocks_an_active_expansion(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    owner = Identity("codex", "owner")
    _set_identity(monkeypatch, owner.session_id)
    assert coordinator.start("app", ("src/app.py",), cwd=git_repo).kind == "READY"
    _set_identity(monkeypatch, "docs-holder")
    assert coordinator.start("readme", ("docs/readme.md",), cwd=git_repo).kind == "READY"
    _set_identity(monkeypatch, "docs-waiter")
    assert coordinator.start("docs queue", ("docs",), cwd=git_repo).kind == "BLOCKED"

    _set_identity(monkeypatch, owner.session_id)
    blocked = coordinator.start(
        "app plus other docs", ("src/app.py", "docs/other.md"), cwd=git_repo
    )

    assert blocked.kind == "ACTIVE"
    assert blocked.detail == "update-blocked:codex/docs-wai"
    claim = coordinator.store.claim(owner)
    assert claim is not None
    assert (claim["label"], claim["paths"]) == ("app", ("src/app.py",))


def test_active_expansion_fails_closed_without_replacing_scope(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = _coordinator(tmp_path / "state.db")
    owner = Identity("codex", "owner")
    _set_identity(monkeypatch, owner.session_id)
    assert coordinator.start("app", ("src/app.py",), cwd=git_repo).kind == "READY"
    original = coordinator.store.claim(owner)
    assert original is not None

    coordinator.inventory = StaticInventory(False)
    incomplete = coordinator.start("expanded", ("src/app.py", "docs/readme.md"), cwd=git_repo)
    assert (incomplete.kind, incomplete.detail) == ("ACTIVE", "update-unknown:coverage")
    assert coordinator.store.claim(owner) == original

    coordinator.inventory = StaticInventory()
    (git_repo / "docs" / "readme.md").write_text("changed\n")
    dirty = coordinator.start("expanded", ("src/app.py", "docs/readme.md"), cwd=git_repo)
    assert dirty.kind == "ACTIVE"
    assert dirty.detail == "update-unknown:dirty-settling:docs/readme.md"
    assert coordinator.store.claim(owner) == original


def test_queued_refinement_preserves_age_but_lateral_replacement_resets_it(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    current = 1.0
    monkeypatch.setattr(coordinator_module, "now_ts", lambda: current)
    coordinator = _coordinator(tmp_path / "state.db")
    _set_identity(monkeypatch, "holder")
    assert coordinator.start("holder", ("src/app.py",), cwd=git_repo).kind == "READY"
    waiter = Identity("codex", "waiter")
    _set_identity(monkeypatch, waiter.session_id)
    current = 2.0
    assert coordinator.start("broad", ("src",), cwd=git_repo).kind == "BLOCKED"

    current = 3.0
    assert coordinator.start("exact", ("src/app.py",), cwd=git_repo).kind == "BLOCKED"
    refined = coordinator.store.claim(waiter)
    assert refined is not None
    assert (refined["paths"], refined["created_at"]) == (("src/app.py",), 2.0)

    current = 4.0
    assert coordinator.start("moved", ("docs/readme.md",), cwd=git_repo).kind == "READY"
    moved = coordinator.store.claim(waiter)
    assert moved is not None
    assert (moved["paths"], moved["created_at"]) == (("docs/readme.md",), 4.0)


def test_intent_age_resets_when_it_becomes_a_scoped_claim(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    current = 1.0
    monkeypatch.setattr(coordinator_module, "now_ts", lambda: current)
    coordinator = _coordinator(tmp_path / "state.db")
    identity = Identity("codex", "planner")
    _set_identity(monkeypatch, identity.session_id)
    assert coordinator.start("plan", (), cwd=git_repo).kind == "INTENT"

    current = 5.0
    assert coordinator.start("implement", ("src/app.py",), cwd=git_repo).kind == "READY"
    claim = coordinator.store.claim(identity)
    assert claim is not None
    assert claim["created_at"] == 5.0


def test_active_refinement_moves_released_dirt_to_residual_ownership(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    (git_repo / ".ai-coord.toml").write_text('[dirt]\nbenign = ["src", "docs"]\n')
    (git_repo / "src" / "app.py").write_text("changed src\n")
    (git_repo / "docs" / "readme.md").write_text("changed docs\n")
    coordinator = _coordinator(tmp_path / "state.db")
    owner = Identity("codex", "owner")
    _set_identity(monkeypatch, owner.session_id)
    assert coordinator.start("both", ("src/app.py", "docs/readme.md"), cwd=git_repo).kind == "READY"
    assert {row["path"] for row in coordinator.baselines()} == {
        "src/app.py",
        "docs/readme.md",
    }

    assert coordinator.start("src only", ("src/app.py",), cwd=git_repo).kind == "READY"

    assert [row["path"] for row in coordinator.baselines()] == ["src/app.py"]
    residual = coordinator.store.residual_owners(str(git_repo))
    assert (residual["docs/readme.md"]["client"], residual["docs/readme.md"]["session_id"]) == (
        owner.client,
        owner.session_id,
    )


def test_simultaneous_active_expansions_have_one_atomic_winner(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    db_path = tmp_path / "state.db"
    coordinator = _coordinator(db_path)
    originals = {"session-a": "src/app.py", "session-b": "docs/readme.md"}
    for session_id, scope in originals.items():
        _set_identity(monkeypatch, session_id)
        assert coordinator.start(session_id, (scope,), cwd=git_repo).kind == "READY"
    gate = tmp_path / "go"

    with ProcessPoolExecutor(max_workers=2) as executor:
        futures = [
            executor.submit(
                _replace_worker,
                str(db_path),
                str(git_repo),
                str(gate),
                session_id,
                scope,
            )
            for session_id, scope in originals.items()
        ]
        gate.touch()
        results = [future.result(timeout=10) for future in futures]

    assert sorted((kind, code) for _, kind, code in results) == [("ACTIVE", 3), ("READY", 0)]
    claims = {str(claim["session_id"]): claim for claim in coordinator.store.claims(str(git_repo))}
    assert sum("shared/new.py" in claim["paths"] for claim in claims.values()) == 1
    loser = next(session_id for session_id, kind, _ in results if kind == "ACTIVE")
    assert claims[loser]["paths"] == (originals[loser],)
