"""Codex and Claude Code inventory adapters."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Protocol

from ai_coord.store import Store
from ai_coord.util import git_root, now_ts

CLAUDE_LIVE_STATES = {
    "busy": "working",
    "working": "working",
    "blocked": "waiting",
    "waiting": "waiting",
    "idle": "idle",
}
CLAUDE_TERMINAL_STATES = {"completed", "done", "failed", "stopped"}
CODEX_EVENTS = {
    "SessionStart",
    "UserPromptSubmit",
    "Stop",
    "SessionEnd",
    "SubagentStart",
    "SubagentStop",
}


@dataclass(frozen=True, slots=True)
class ProviderReport:
    client: str
    ok: bool
    source: str
    enabled: bool = True
    dropped: int = 0
    error: str | None = None

    def as_dict(self) -> dict[str, Any]:
        return {
            "client": self.client,
            "ok": self.ok,
            "source": self.source,
            "enabled": self.enabled,
            "dropped": self.dropped,
            "error": self.error,
        }


@dataclass(frozen=True, slots=True)
class InventoryResult:
    complete: bool
    providers: tuple[ProviderReport, ...]


class Inventory(Protocol):
    def refresh(self, store: Store) -> InventoryResult: ...


class HostInventory:
    """Observe configured host clients and reconcile Claude's live inventory."""

    def refresh(self, store: Store) -> InventoryResult:
        current = now_ts()
        store.prune(current)
        codex = self._codex_report(store)
        claude, rows = self._collect_claude()
        if claude.ok and claude.enabled:
            store.replace_claude_sessions(rows, current)
        reports = (codex, claude)
        complete = all(
            (not report.enabled) or (report.ok and report.dropped == 0) for report in reports
        )
        return InventoryResult(complete=complete, providers=reports)

    def _codex_report(self, store: Store) -> ProviderReport:
        if shutil.which("codex") is None:
            return ProviderReport("codex", True, "hook-ledger", enabled=False)
        codex_home = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex"))
        path = codex_home / "hooks.json"
        try:
            data = json.loads(path.read_text())
        except FileNotFoundError:
            return ProviderReport("codex", False, "hook-ledger", error=f"missing {path}")
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            return ProviderReport("codex", False, "hook-ledger", error=str(error))
        missing = _missing_hook_events(data, CODEX_EVENTS, "ai-coord hook codex")
        errors = [
            row
            for row in store.hook_health()
            if row["client"] == "codex" and row["last_error_code"]
        ]
        if missing:
            return ProviderReport(
                "codex", False, "hook-ledger", error=f"missing hooks: {', '.join(sorted(missing))}"
            )
        if errors:
            return ProviderReport(
                "codex",
                False,
                "hook-ledger",
                error=f"last hook error: {errors[-1]['last_error_code']}",
            )
        return ProviderReport("codex", True, "hook-ledger")

    def _collect_claude(self) -> tuple[ProviderReport, list[dict[str, Any]]]:
        if shutil.which("claude") is None:
            return ProviderReport("claude", True, "claude-agents-json", enabled=False), []
        try:
            result = subprocess.run(
                ["claude", "agents", "--json"],
                capture_output=True,
                check=False,
                text=True,
                timeout=10,
            )
        except (OSError, subprocess.SubprocessError) as error:
            return ProviderReport("claude", False, "claude-agents-json", error=str(error)), []
        if result.returncode != 0:
            detail = result.stderr.strip() or f"exit {result.returncode}"
            return ProviderReport("claude", False, "claude-agents-json", error=detail), []
        try:
            payload = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            return (
                ProviderReport(
                    "claude", False, "claude-agents-json", error=f"invalid JSON: {error}"
                ),
                [],
            )
        rows, dropped = normalize_claude_sessions(payload)
        return ProviderReport("claude", True, "claude-agents-json", dropped=dropped), rows


@dataclass(frozen=True, slots=True)
class StaticInventory:
    """Deterministic inventory used by tests and embedded callers."""

    complete: bool = True

    def refresh(self, store: Store) -> InventoryResult:
        del store
        reports = (
            ProviderReport("codex", self.complete, "static"),
            ProviderReport("claude", self.complete, "static"),
        )
        return InventoryResult(self.complete, reports)


def normalize_claude_sessions(payload: Any) -> tuple[list[dict[str, Any]], int]:
    if not isinstance(payload, list):
        return [], 1
    rows: list[dict[str, Any]] = []
    dropped = 0
    for raw in payload:
        if not isinstance(raw, dict):
            dropped += 1
            continue
        state_value = raw.get("state") or raw.get("status")
        if not isinstance(state_value, str):
            dropped += 1
            continue
        lowered = state_value.lower()
        if lowered in CLAUDE_TERMINAL_STATES:
            continue
        session_id = raw.get("sessionId") or raw.get("id")
        cwd = raw.get("cwd")
        started_at = _timestamp(raw.get("startedAt"))
        if not isinstance(session_id, str) or not session_id or not isinstance(cwd, str) or not cwd:
            dropped += 1
            continue
        if started_at is None:
            dropped += 1
            continue
        pid = raw.get("pid")
        if isinstance(pid, bool) or not isinstance(pid, int) or pid <= 0:
            pid = None
        name = raw.get("name") if isinstance(raw.get("name"), str) else None
        waiting = raw.get("waitingFor") if isinstance(raw.get("waitingFor"), str) else None
        root = git_root(Path(cwd))
        rows.append(
            {
                "session_id": session_id,
                "cwd": cwd,
                "repo_root": str(root) if root else None,
                "state": CLAUDE_LIVE_STATES.get(lowered, "unknown"),
                "name": name,
                "waiting_for": waiting,
                "pid": pid,
                "started_at": started_at,
            }
        )
    return rows, dropped


def _timestamp(value: Any) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value / 1000 if value > 10_000_000_000 else value)
    if isinstance(value, str) and value:
        try:
            return datetime.fromisoformat(value).timestamp()
        except ValueError:
            return None
    return None


def _missing_hook_events(data: Any, events: set[str], command: str) -> set[str]:
    hooks = data.get("hooks") if isinstance(data, dict) else None
    if not isinstance(hooks, dict):
        return set(events)
    return {event for event in events if command not in _commands(hooks.get(event))}


def _commands(value: Any) -> set[str]:
    found: set[str] = set()
    if isinstance(value, dict):
        command = value.get("command")
        if isinstance(command, str):
            found.add(command.strip())
        for nested in value.values():
            found.update(_commands(nested))
    elif isinstance(value, list):
        for nested in value:
            found.update(_commands(nested))
    return found
