"""
Helpers for managing Claude Code hooks.json configuration.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


REQUIRED_HOOKS = {
    "UserPromptSubmit": "ai-notify event user-prompt-submit",
    "Stop": "ai-notify event stop",
    "Notification": "ai-notify event notification",
    "PermissionRequest": "ai-notify event permission-request",
}


@dataclass(frozen=True)
class ClaudeHooksUpdate:
    path: Path
    changed: bool
    added: list[str]
    updated: list[str]
    skipped: dict[str, str]
    errors: list[str]


def ensure_claude_hooks(
    path: Path, force: bool = False, dry_run: bool = False
) -> ClaudeHooksUpdate:
    """
    Ensure Claude Code hooks include ai-notify commands.

    Args:
        path: Path to hooks.json
        force: Overwrite existing hook commands
        dry_run: Do not write changes

    Returns:
        ClaudeHooksUpdate with changes and any skipped hooks.
    """
    data: dict[str, Any] = {}
    if path.exists():
        data = _load_json(path)

    updated_data, report = _update_hooks_data(data, force=force)

    if report.errors:
        return ClaudeHooksUpdate(
            path=path,
            changed=False,
            added=report.added,
            updated=report.updated,
            skipped=report.skipped,
            errors=report.errors,
        )

    changed = report.changed
    if changed and not dry_run:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(updated_data, indent=2) + "\n")

    return ClaudeHooksUpdate(
        path=path,
        changed=changed,
        added=report.added,
        updated=report.updated,
        skipped=report.skipped,
        errors=report.errors,
    )


@dataclass
class _HooksUpdateReport:
    changed: bool
    added: list[str]
    updated: list[str]
    skipped: dict[str, str]
    errors: list[str]


def _update_hooks_data(
    data: dict[str, Any], force: bool
) -> tuple[dict[str, Any], _HooksUpdateReport]:
    added: list[str] = []
    updated: list[str] = []
    skipped: dict[str, str] = {}
    errors: list[str] = []

    if not isinstance(data, dict):
        return data, _HooksUpdateReport(
            False, added, updated, skipped, ["hooks.json is not a JSON object"]
        )

    hooks = data.get("hooks")
    if hooks is None:
        hooks = {}
        data["hooks"] = hooks

    if not isinstance(hooks, dict):
        return data, _HooksUpdateReport(
            False,
            added,
            updated,
            skipped,
            ["hooks field must be an object"],
        )

    for event, command in REQUIRED_HOOKS.items():
        existing = hooks.get(event)

        if _has_ai_notify_command(existing, command):
            continue

        if existing is None:
            hooks[event] = {"command": command}
            added.append(event)
            continue

        if force:
            hooks[event] = {"command": command}
            updated.append(event)
        else:
            skipped[event] = _summarize_hook(existing)

    changed = bool(added or updated)
    return data, _HooksUpdateReport(changed, added, updated, skipped, errors)


def _has_ai_notify_command(existing: Any, expected: str) -> bool:
    if existing is None:
        return False

    if isinstance(existing, str):
        return existing.strip() == expected

    if isinstance(existing, dict):
        command = existing.get("command")
        if isinstance(command, str) and command.strip() == expected:
            return True
        return False

    if isinstance(existing, list):
        return any(_has_ai_notify_command(item, expected) for item in existing)

    return False


def _summarize_hook(existing: Any) -> str:
    if isinstance(existing, str):
        return existing
    if isinstance(existing, dict):
        command = existing.get("command")
        if isinstance(command, str):
            return command
        return "<object>"
    if isinstance(existing, list):
        return f"<list:{len(existing)}>"
    return "<unknown>"


def _load_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text())
    except Exception as exc:  # noqa: BLE001
        raise ValueError(f"Failed to parse {path}: {exc}") from exc

    if not isinstance(data, dict):
        raise ValueError(f"{path} must contain a JSON object at the root")
    return data
