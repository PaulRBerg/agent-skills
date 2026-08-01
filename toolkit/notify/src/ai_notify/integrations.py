"""
Integration checks for Claude Code hooks and Codex CLI notify settings.
"""

from __future__ import annotations

import json
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable

from ai_notify.claude_hooks import HOOK_SPECS, iter_hook_commands
from ai_notify.codex_config import resolve_codex_config_path


# Map of Claude Code event name -> ai-notify event subcommand, derived from the
# installer's HOOK_SPECS so `check` and `link claude` can never drift.
CLAUDE_REQUIRED_EVENTS = {spec.event: spec.command.split("event ", 1)[1] for spec in HOOK_SPECS}


@dataclass
class ClaudeHooksReport:
    status: str
    paths: list[Path]
    missing_events: list[str]
    errors: dict[Path, str]
    ignored_paths: list[Path]

    @property
    def path(self) -> Path | None:
        """Return the first contributing path for compatibility with older callers."""
        return self.paths[0] if self.paths else None


@dataclass
class CodexNotifyReport:
    status: str
    path: Path | None
    notify: Any
    error: str | None
    profile: str | None = None
    paths: list[Path] = field(default_factory=list)


def inspect_claude_hooks(config_root: Path, project_root: Path) -> ClaudeHooksReport:
    """
    Inspect the aggregate Claude Code hook configuration for ai-notify commands.
    """
    active_paths = [
        config_root / "settings.json",
        project_root / ".claude" / "settings.json",
        project_root / ".claude" / "settings.local.json",
    ]
    ignored_paths = [
        path
        for path in (
            config_root / "hooks" / "hooks.json",
            config_root / "settings.local.json",
        )
        if path.exists()
    ]

    errors: dict[Path, str] = {}
    commands_by_event: dict[str, list[str]] = {}
    contributing_paths: list[Path] = []

    for path in active_paths:
        if not path.exists():
            continue

        try:
            with open(path, "r") as f:
                data = json.load(f)
        except Exception as exc:  # noqa: BLE001
            errors[path] = str(exc)
            continue

        hooks = data.get("hooks") if isinstance(data, dict) else None
        path_commands = _extract_hook_commands(hooks)
        for event, commands in path_commands.items():
            commands_by_event.setdefault(event, []).extend(commands)

        if any(
            _has_ai_notify_event_command(path_commands.get(event, []), subcommand)
            for event, subcommand in CLAUDE_REQUIRED_EVENTS.items()
        ):
            contributing_paths.append(path)

    missing_events = _find_missing_events(commands_by_event)
    status = "ok" if not missing_events else "partial" if contributing_paths else "missing"
    return ClaudeHooksReport(
        status=status,
        paths=contributing_paths,
        missing_events=missing_events,
        errors=errors,
        ignored_paths=ignored_paths,
    )


def inspect_codex_notify(config_root: Path, profile: str | None = None) -> CodexNotifyReport:
    """
    Inspect Codex CLI notify using base-plus-profile overlay semantics.
    """
    base_path = config_root / "config.toml"
    profile_path = resolve_codex_config_path(base_path, profile)
    layer_paths = [base_path] if profile is None else [base_path, profile_path]
    loaded_paths: list[Path] = []
    notify: Any = None
    notify_found = False
    notify_path: Path | None = None

    if profile is not None and not profile_path.exists():
        return CodexNotifyReport(
            status="error",
            path=profile_path,
            notify=None,
            error=f"Profile config not found: {profile_path}",
            profile=profile,
            paths=[base_path] if base_path.exists() else [],
        )

    for path in layer_paths:
        if not path.exists():
            continue
        loaded_paths.append(path)
        try:
            with open(path, "rb") as file:
                data = tomllib.load(file)
        except Exception as exc:  # noqa: BLE001
            return CodexNotifyReport(
                status="error",
                path=path,
                notify=None,
                error=str(exc),
                profile=profile,
                paths=loaded_paths,
            )

        if "notify" in data:
            notify = data["notify"]
            notify_found = True
            notify_path = path

    if not notify_found:
        return CodexNotifyReport(
            status="missing",
            path=loaded_paths[-1] if loaded_paths else None,
            notify=None,
            error=None,
            profile=profile,
            paths=loaded_paths,
        )

    if _notify_uses_ai_notify(notify):
        return CodexNotifyReport(
            status="ok",
            path=notify_path,
            notify=notify,
            error=None,
            profile=profile,
            paths=loaded_paths,
        )

    return CodexNotifyReport(
        status="partial",
        path=notify_path,
        notify=notify,
        error=None,
        profile=profile,
        paths=loaded_paths,
    )


def _extract_hook_commands(hooks: Any) -> dict[str, list[str]]:
    if not isinstance(hooks, dict):
        return {}

    # Reuse the installer's traversal so `check` and `link claude` read the schema identically.
    return {
        event_name: list(iter_hook_commands(hook_value)) for event_name, hook_value in hooks.items()
    }


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
