"""
Integration checks for Claude Code hooks and Codex CLI notify settings.
"""

from __future__ import annotations

import json
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


CLAUDE_REQUIRED_EVENTS = {
    "UserPromptSubmit": "user-prompt-submit",
    "Stop": "stop",
    "Notification": "notification",
    "PermissionRequest": "permission-request",
}


@dataclass
class ClaudeHooksReport:
    status: str
    path: Path | None
    missing_events: list[str]
    errors: dict[Path, str]


@dataclass
class CodexNotifyReport:
    status: str
    path: Path | None
    notify: Any
    error: str | None


def inspect_claude_hooks(config_root: Path, project_root: Path) -> ClaudeHooksReport:
    """
    Inspect Claude Code hook configuration for ai-notify commands.
    """
    candidate_paths = [
        config_root / "hooks" / "hooks.json",
        config_root / "settings.json",
        config_root / "settings.local.json",
        project_root / ".claude" / "settings.json",
        project_root / ".claude" / "settings.local.json",
    ]

    errors: dict[Path, str] = {}
    best_report: ClaudeHooksReport | None = None

    for path in candidate_paths:
        if not path.exists():
            continue

        try:
            with open(path, "r") as f:
                data = json.load(f)
        except Exception as exc:  # noqa: BLE001
            errors[path] = str(exc)
            continue

        hooks = data.get("hooks") if isinstance(data, dict) else None
        commands_by_event = _extract_hook_commands(hooks)
        missing_events = _find_missing_events(commands_by_event)

        if not missing_events:
            return ClaudeHooksReport(status="ok", path=path, missing_events=[], errors=errors)

        report = ClaudeHooksReport(
            status="partial" if missing_events else "missing",
            path=path,
            missing_events=missing_events,
            errors=errors,
        )

        if best_report is None or _report_score(report) > _report_score(best_report):
            best_report = report

    if best_report is not None:
        return best_report

    return ClaudeHooksReport(
        status="missing",
        path=None,
        missing_events=list(CLAUDE_REQUIRED_EVENTS),
        errors=errors,
    )


def inspect_codex_notify(config_root: Path) -> CodexNotifyReport:
    """
    Inspect Codex CLI notify setting in the root config.
    """
    config_path = config_root / "config.toml"
    if not config_path.exists():
        return CodexNotifyReport(status="missing", path=None, notify=None, error=None)

    try:
        with open(config_path, "rb") as f:
            data = tomllib.load(f)
    except Exception as exc:  # noqa: BLE001
        return CodexNotifyReport(
            status="error",
            path=config_path,
            notify=None,
            error=str(exc),
        )

    notify = data.get("notify")
    if not notify:
        return CodexNotifyReport(status="missing", path=config_path, notify=None, error=None)

    if _notify_uses_ai_notify(notify):
        return CodexNotifyReport(status="ok", path=config_path, notify=notify, error=None)

    return CodexNotifyReport(status="partial", path=config_path, notify=notify, error=None)


def _extract_hook_commands(hooks: Any) -> dict[str, list[str]]:
    if not isinstance(hooks, dict):
        return {}

    commands_by_event: dict[str, list[str]] = {}
    for event_name, hook_value in hooks.items():
        commands_by_event[event_name] = _extract_commands(hook_value)
    return commands_by_event


def _extract_commands(value: Any) -> list[str]:
    commands: list[str] = []

    if isinstance(value, str):
        commands.append(value)
        return commands

    if isinstance(value, dict):
        command = value.get("command")
        if isinstance(command, str):
            commands.append(command)
        nested = value.get("hooks")
        if nested:
            commands.extend(_extract_commands(nested))
        return commands

    if isinstance(value, list):
        for item in value:
            commands.extend(_extract_commands(item))
        return commands

    return commands


def _find_missing_events(commands_by_event: dict[str, list[str]]) -> list[str]:
    missing: list[str] = []
    for event, subcommand in CLAUDE_REQUIRED_EVENTS.items():
        commands = commands_by_event.get(event, [])
        if not _has_ai_notify_event_command(commands, subcommand):
            missing.append(event)
    return missing


def _has_ai_notify_event_command(commands: Iterable[str], subcommand: str) -> bool:
    for command in commands:
        if "ai-notify" in command and f"event {subcommand}" in command:
            return True
    return False


def _notify_uses_ai_notify(notify: Any) -> bool:
    if isinstance(notify, str):
        return "ai-notify" in notify and "codex" in notify
    if isinstance(notify, list):
        as_strings = [str(item) for item in notify]
        has_ai_notify = any("ai-notify" in item for item in as_strings)
        has_codex = any("codex" in item for item in as_strings)
        return has_ai_notify and has_codex
    return False


def _report_score(report: ClaudeHooksReport) -> int:
    if report.status == "ok":
        return 3
    if report.status == "partial":
        return 2
    if report.status == "missing":
        return 1
    return 0
