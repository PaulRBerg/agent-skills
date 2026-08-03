from __future__ import annotations

from dataclasses import dataclass

import psutil
import pytest

from ai_coord import identity
from ai_coord.identity import ProcessReference, process_ancestors, process_reference


@dataclass
class _Process:
    pid: int
    started_at: float | None = None
    ancestors: tuple[_Process, ...] = ()
    denied: bool = False

    def create_time(self) -> float:
        if self.denied:
            raise psutil.AccessDenied(self.pid)
        assert self.started_at is not None
        return self.started_at

    def parents(self) -> list[_Process]:
        return list(self.ancestors)


def test_process_ancestors_records_starting_parent_and_at_most_fifteen_ancestors(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    ancestors = tuple(_Process(pid, float(pid)) for pid in range(2, 22))
    parent = _Process(42, 42.5, ancestors)
    monkeypatch.setattr(identity.psutil, "Process", lambda pid: parent)

    references = process_ancestors(42)

    assert references == (
        ProcessReference(42, 42.5),
        *(ProcessReference(pid, float(pid)) for pid in range(2, 17)),
    )


def test_process_ancestry_preserves_reference_when_creation_time_is_denied(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    parent = _Process(42, denied=True)
    monkeypatch.setattr(identity.psutil, "Process", lambda pid: parent)

    assert process_ancestors(42) == (ProcessReference(42, None),)
    assert process_reference(42) == ProcessReference(42, None)


def test_process_ancestry_is_empty_for_missing_starting_process(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def missing(pid: int) -> _Process:
        raise psutil.NoSuchProcess(pid)

    monkeypatch.setattr(identity.psutil, "Process", missing)

    assert process_ancestors(42) == ()
