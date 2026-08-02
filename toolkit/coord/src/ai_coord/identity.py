"""Session identity resolution."""

from __future__ import annotations

import os
import subprocess
import time
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Identity:
    client: str
    session_id: str

    @property
    def key(self) -> str:
        return f"{self.client}/{self.session_id}"


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


def process_ancestors(
    start_pid: int | None = None, timeout_seconds: float = 1.5
) -> tuple[int, ...]:
    """Return a bounded process ancestry chain."""
    pid = start_pid or os.getppid()
    visited: set[int] = set()
    chain: list[int] = []
    deadline = time.monotonic() + timeout_seconds
    for _ in range(16):
        if pid <= 1 or pid in visited or time.monotonic() >= deadline:
            break
        visited.add(pid)
        chain.append(pid)
        try:
            result = subprocess.run(
                ["ps", "-o", "ppid=", "-p", str(pid)],
                capture_output=True,
                check=False,
                text=True,
                timeout=max(0.05, deadline - time.monotonic()),
            )
        except (OSError, subprocess.SubprocessError):
            break
        try:
            pid = int(result.stdout.strip())
        except ValueError:
            break
    return tuple(chain)
