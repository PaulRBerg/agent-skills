"""Session identity resolution."""

from __future__ import annotations

import os
from dataclasses import dataclass

import psutil


@dataclass(frozen=True, slots=True)
class Identity:
    client: str
    session_id: str

    @property
    def key(self) -> str:
        return f"{self.client}/{self.session_id}"


@dataclass(frozen=True, slots=True)
class ProcessReference:
    pid: int
    started_at: float | None


def from_environment() -> Identity | None:
    """Resolve the direct host-provided identity, with test overrides."""
    override_client = os.environ.get("AI_COORD_CLIENT")
    override_session = os.environ.get("AI_COORD_SESSION_ID")
    if override_client in {"codex", "claude"} and override_session:
        return Identity(override_client, override_session)

    codex = os.environ.get("CODEX_THREAD_ID")
    if codex:
        return Identity("codex", codex)
    claude = os.environ.get("CLAUDE_CODE_SESSION_ID")
    if claude:
        return Identity("claude", claude)
    return None


def process_reference(pid: int) -> ProcessReference:
    """Return the strongest available reference for one process."""
    try:
        started_at = psutil.Process(pid).create_time()
    except (psutil.Error, OSError, ValueError):
        started_at = None
    return ProcessReference(pid, started_at)


def process_ancestors(start_pid: int | None = None) -> tuple[ProcessReference, ...]:
    """Return the starting parent and at most 15 of its ancestors."""
    pid = os.getppid() if start_pid is None else start_pid
    if pid <= 1:
        return ()
    try:
        process = psutil.Process(pid)
    except (psutil.Error, ValueError):
        return ()
    try:
        ancestors = process.parents()[:15]
    except (psutil.Error, OSError):
        ancestors = []
    chain = (process, *ancestors)
    references: list[ProcessReference] = []
    for candidate in chain:
        try:
            started_at = candidate.create_time()
        except (psutil.Error, OSError):
            started_at = None
        references.append(ProcessReference(candidate.pid, started_at))
    return tuple(references)
