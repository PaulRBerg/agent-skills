"""Codex and Claude Code inventory adapters."""

from __future__ import annotations

import hashlib
import json
import math
import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Protocol

import psutil

from ai_coord.identity import Identity, ProcessReference, process_reference
from ai_coord.integrations import default_hook_path, inspect_codex_hook_trust, inspect_hooks
from ai_coord.store import CODEX_ORPHAN_GRACE, Store
from ai_coord.util import git_root, now_ts

CLAUDE_LIVE_STATES = {
    "busy": "working",
    "working": "working",
    "blocked": "waiting",
    "waiting": "waiting",
    "idle": "idle",
}
CLAUDE_TERMINAL_STATES = {"completed", "done", "failed", "stopped"}
INVENTORY_CACHE_SECONDS = 2.0
_PROVIDER_CLIENTS = ("codex", "claude")


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
    def refresh(self, store: Store, *, allow_cached: bool = False) -> InventoryResult: ...


class HostInventory:
    """Observe configured host clients and reconcile their live inventory."""

    def refresh(self, store: Store, *, allow_cached: bool = False) -> InventoryResult:
        current = now_ts()
        executables = {client: shutil.which(client) for client in _PROVIDER_CLIENTS}
        context_key = _cache_context(executables)
        store.prune(current, dead_codex_sessions=_dead_codex_sessions(store, current))
        if (
            allow_cached
            and (cached := _cached_inventory(store, context_key, current=current)) is not None
        ):
            return cached

        with ThreadPoolExecutor(max_workers=1, thread_name_prefix="ai-coord-claude") as executor:
            claude_future = executor.submit(self._collect_claude, executables["claude"])
            codex = self._codex_report(store, executables["codex"])
            claude, rows = claude_future.result()
        if claude.ok and claude.enabled:
            store.replace_claude_sessions(rows, current)
        reports = (codex, claude)
        complete = all(
            (not report.enabled) or (report.ok and report.dropped == 0) for report in reports
        )
        if complete:
            store.replace_provider_cache(
                context_key,
                [report.as_dict() for report in reports],
                current,
            )
        else:
            store.clear_provider_cache()
        return InventoryResult(complete=complete, providers=reports)

    def _codex_report(self, store: Store, executable: str | None) -> ProviderReport:
        if executable is None:
            return ProviderReport("codex", True, "hook-ledger", enabled=False)
        hooks = inspect_hooks("codex", default_hook_path("codex"))
        errors = [
            row
            for row in store.hook_health()
            if row["client"] == "codex" and row["last_error_code"]
        ]
        if not hooks.ok:
            details: list[str] = []
            if hooks.error:
                details.append(hooks.error)
            if hooks.missing:
                details.append(f"missing or invalid hooks: {', '.join(sorted(hooks.missing))}")
            if hooks.legacy_commands:
                details.append("legacy hooks remain")
            return ProviderReport("codex", False, "hook-ledger", error="; ".join(details))
        if errors:
            return ProviderReport(
                "codex",
                False,
                "hook-ledger",
                error=f"last hook error: {errors[-1]['last_error_code']}",
            )
        trust = inspect_codex_hook_trust(hooks.path)
        if not trust.ok:
            detail = trust.error or "app-server hook trust could not be verified"
            return ProviderReport("codex", False, "hook-ledger", error=f"hook trust: {detail}")
        return ProviderReport("codex", True, "hook-ledger")

    def _collect_claude(
        self, executable: str | None
    ) -> tuple[ProviderReport, list[dict[str, Any]]]:
        if executable is None:
            return ProviderReport("claude", True, "claude-agents-json", enabled=False), []
        try:
            result = subprocess.run(
                [executable, "agents", "--json"],
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

    def refresh(self, store: Store, *, allow_cached: bool = False) -> InventoryResult:
        del store, allow_cached
        reports = (
            ProviderReport("codex", self.complete, "static"),
            ProviderReport("claude", self.complete, "static"),
        )
        return InventoryResult(self.complete, reports)


def _cache_context(executables: dict[str, str | None]) -> str:
    context = {
        "codex_executable": executables["codex"],
        "codex_home": str(default_hook_path("codex").parent.resolve(strict=False)),
        "claude_executable": executables["claude"],
        "claude_config_dir": str(default_hook_path("claude").parent.resolve(strict=False)),
    }
    encoded = json.dumps(context, separators=(",", ":"), sort_keys=True).encode()
    return hashlib.sha256(encoded).hexdigest()


def _cached_inventory(store: Store, context_key: str, *, current: float) -> InventoryResult | None:
    rows = store.provider_cache(context_key)
    by_client = {str(row["client"]): row for row in rows}
    if set(by_client) != set(_PROVIDER_CLIENTS):
        return None
    refreshed_at = {float(row["refreshed_at"]) for row in rows}
    if len(refreshed_at) != 1:
        return None
    age = current - refreshed_at.pop()
    if age < 0 or age >= INVENTORY_CACHE_SECONDS:
        return None
    reports = tuple(
        ProviderReport(
            client,
            bool(by_client[client]["ok"]),
            str(by_client[client]["source"]),
            enabled=bool(by_client[client]["enabled"]),
            dropped=int(by_client[client]["dropped"]),
        )
        for client in _PROVIDER_CLIENTS
    )
    return InventoryResult(complete=True, providers=reports)


def _dead_codex_sessions(store: Store, current: float) -> tuple[Identity, ...]:
    cutoff = current - CODEX_ORPHAN_GRACE
    dead: list[Identity] = []
    for row in store.sessions():
        pid = row.get("pid")
        if (
            row["client"] != "codex"
            or not isinstance(pid, int)
            or pid <= 0
            or float(row["last_seen"]) >= cutoff
        ):
            continue
        started_at = row.get("process_started_at")
        reference = ProcessReference(
            pid,
            float(started_at) if isinstance(started_at, (int, float)) else None,
        )
        if not _process_exists(reference):
            dead.append(Identity("codex", str(row["session_id"])))
    return tuple(dead)


def _process_exists(reference: ProcessReference) -> bool:
    try:
        process = psutil.Process(reference.pid)
        if process.status() in {psutil.STATUS_DEAD, psutil.STATUS_ZOMBIE}:
            return False
        if reference.started_at is not None:
            return process.create_time() == reference.started_at
    except (psutil.NoSuchProcess, psutil.ZombieProcess):
        return False
    except (psutil.Error, OSError):
        return True
    return True


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
        try:
            root = git_root(Path(cwd))
        except ValueError:
            dropped += 1
            continue
        pid = raw.get("pid")
        if isinstance(pid, bool) or not isinstance(pid, int) or pid <= 0:
            pid = None
        process_started_at = process_reference(pid).started_at if pid is not None else None
        name = raw.get("name") if isinstance(raw.get("name"), str) else None
        waiting = raw.get("waitingFor") if isinstance(raw.get("waitingFor"), str) else None
        rows.append(
            {
                "session_id": session_id,
                "cwd": cwd,
                "repo_root": str(root) if root else None,
                "state": CLAUDE_LIVE_STATES.get(lowered, "unknown"),
                "name": name,
                "waiting_for": waiting,
                "pid": pid,
                "process_started_at": process_started_at,
                "started_at": started_at,
            }
        )
    return rows, dropped


def _timestamp(value: Any) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        timestamp = float(value / 1000 if value > 10_000_000_000 else value)
        return timestamp if math.isfinite(timestamp) else None
    if isinstance(value, str) and value:
        try:
            timestamp = datetime.fromisoformat(value).timestamp()
            return timestamp if math.isfinite(timestamp) else None
        except (OSError, OverflowError, ValueError):
            return None
    return None
