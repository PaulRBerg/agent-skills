from __future__ import annotations

import os
import subprocess
import tempfile
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from itertools import combinations
from pathlib import Path
from unittest.mock import patch

from hypothesis import settings
from hypothesis import strategies as st
from hypothesis.stateful import RuleBasedStateMachine, invariant, rule

import ai_coord.coordinator as coordinator_module
from ai_coord.coordinator import Coordinator
from ai_coord.providers import StaticInventory
from ai_coord.store import Store
from ai_coord.util import any_overlap, overlaps_outside_coverage, scopes_cover

SESSIONS = st.sampled_from(tuple(f"session-{index}" for index in range(4)))
SCOPES = st.sampled_from(
    (
        ("src",),
        ("src/app.py",),
        ("src/lib",),
        ("docs",),
        ("tests",),
        ("src", "docs"),
    )
)


@dataclass(frozen=True, slots=True)
class _Claim:
    state: str
    paths: tuple[str, ...]
    created_at: float


class CoordinatorStateMachine(RuleBasedStateMachine):
    def __init__(self) -> None:
        super().__init__()
        self.temporary = tempfile.TemporaryDirectory(prefix="ai-coord-properties-")
        self.repo = Path(self.temporary.name) / "repo"
        self.repo.mkdir()
        self.repo = self.repo.resolve()
        subprocess.run(
            ["git", "init", "-b", "main"],
            cwd=self.repo,
            check=True,
            capture_output=True,
        )
        self.store = Store(Path(self.temporary.name) / "state.db")
        self.coordinator = Coordinator(self.store, StaticInventory())
        self.clock = 0.0
        self.clock_patch = patch.object(
            coordinator_module,
            "now_ts",
            new=lambda: self.clock,
        )
        self.clock_patch.start()
        self.model: dict[str, _Claim] = {}

    @contextmanager
    def identity(self, session: str) -> Iterator[None]:
        with patch.dict(
            os.environ,
            {"AI_COORD_CLIENT": "codex", "AI_COORD_SESSION_ID": session},
        ):
            yield

    def expected_state(self, session: str, paths: tuple[str, ...], created_at: float) -> str:
        active_blocker = any(
            candidate != session and claim.state == "active" and any_overlap(paths, claim.paths)
            for candidate, claim in self.model.items()
        )
        earlier_waiter = any(
            candidate != session
            and claim.state == "queued"
            and claim.created_at < created_at
            and any_overlap(paths, claim.paths)
            for candidate, claim in self.model.items()
        )
        return "queued" if active_blocker or earlier_waiter else "active"

    @rule(session=SESSIONS, paths=SCOPES)
    def acquire(self, session: str, paths: tuple[str, ...]) -> None:
        if session in self.model:
            return
        self.clock += 1
        expected = self.expected_state(session, paths, self.clock)

        with self.identity(session):
            outcome = self.coordinator.start(f"claim {session}", paths, cwd=self.repo)

        assert outcome.kind == ("READY" if expected == "active" else "BLOCKED")
        self.model[session] = _Claim(expected, tuple(sorted(paths)), self.clock)

    @rule(session=SESSIONS, reverse_paths=st.booleans())
    def retry(self, session: str, reverse_paths: bool) -> None:
        claim = self.model.get(session)
        if claim is None:
            return
        self.clock += 1
        expected = (
            "active"
            if claim.state == "active"
            else self.expected_state(session, claim.paths, claim.created_at)
        )
        retry_paths = claim.paths[::-1] if reverse_paths else claim.paths

        with self.identity(session):
            outcome = self.coordinator.start(f"claim {session}", retry_paths, cwd=self.repo)

        assert outcome.kind == ("READY" if expected == "active" else "BLOCKED")
        self.model[session] = _Claim(expected, claim.paths, claim.created_at)

    @rule(session=SESSIONS, paths=SCOPES)
    def replace_scope(self, session: str, paths: tuple[str, ...]) -> None:
        claim = self.model.get(session)
        if claim is None:
            return
        self.clock += 1
        normalized = tuple(sorted(paths))
        if claim.state == "active":
            blocked = not scopes_cover(claim.paths, normalized) and any(
                candidate != session
                and other.state in {"active", "queued"}
                and overlaps_outside_coverage(normalized, other.paths, claim.paths)
                for candidate, other in self.model.items()
            )
            expected_kind = "ACTIVE" if blocked else "READY"
            expected = claim if blocked else _Claim("active", normalized, claim.created_at)
        else:
            created_at = claim.created_at if scopes_cover(claim.paths, normalized) else self.clock
            state = self.expected_state(session, normalized, created_at)
            expected_kind = "READY" if state == "active" else "BLOCKED"
            expected = _Claim(state, normalized, created_at)

        with self.identity(session):
            outcome = self.coordinator.start(f"replace {session}", paths, cwd=self.repo)

        assert outcome.kind == expected_kind
        self.model[session] = expected

    @rule(session=SESSIONS)
    def release(self, session: str) -> None:
        self.clock += 1
        existed = session in self.model

        with self.identity(session):
            outcome = self.coordinator.done()

        assert outcome.detail == ("released" if existed else "already clear")
        self.model.pop(session, None)

    @invariant()
    def store_matches_model_and_preserves_arbitration_invariants(self) -> None:
        actual = {str(claim["session_id"]): claim for claim in self.store.claims(str(self.repo))}
        assert set(actual) == set(self.model)
        for session, expected in self.model.items():
            claim = actual[session]
            assert claim["state"] == expected.state
            assert tuple(claim["paths"]) == expected.paths
            assert float(claim["created_at"]) == expected.created_at

        active = [claim for claim in self.model.values() if claim.state == "active"]
        assert all(
            not any_overlap(left.paths, right.paths) for left, right in combinations(active, 2)
        )
        for active_claim in active:
            assert not any(
                queued.state == "queued"
                and queued.created_at < active_claim.created_at
                and any_overlap(queued.paths, active_claim.paths)
                for queued in self.model.values()
            )

    def teardown(self) -> None:
        self.clock_patch.stop()
        self.store.close()
        self.temporary.cleanup()


class TestCoordinatorStateMachine(CoordinatorStateMachine.TestCase):
    settings = settings(max_examples=20, stateful_step_count=20, deadline=None)
