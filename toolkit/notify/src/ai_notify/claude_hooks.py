"""
Helpers for managing ai-notify hooks in Claude Code's settings.json.

Claude Code reads hooks only from settings.json files (``~/.claude/settings.json``,
``.claude/settings.json``, ``.claude/settings.local.json``) using the nested schema
``"<Event>": [ { "matcher"?: str, "hooks": [ { "type": "command", "command": str } ] } ]``.
"""

from __future__ import annotations

import json
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class HookSpec:
    """A single ai-notify hook registration."""

    event: str
    command: str
    matcher: str | None = None


# Single source of truth for the hooks ai-notify installs into Claude Code.
# UserPromptSubmit/Stop ignore matchers (Claude Code drops them); Notification and
# PermissionRequest use no matcher so they fire for every notification/request; the
# AskUserQuestion notifier is a PreToolUse hook scoped to the AskUserQuestion tool.
HOOK_SPECS: list[HookSpec] = [
    HookSpec("UserPromptSubmit", "ai-notify event user-prompt-submit"),
    HookSpec("Stop", "ai-notify event stop"),
    HookSpec("Notification", "ai-notify event notification"),
    HookSpec("PermissionRequest", "ai-notify event permission-request"),
    HookSpec("PreToolUse", "ai-notify event ask-user-question", matcher="AskUserQuestion"),
]


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
    Ensure Claude Code settings include ai-notify hook commands.

    Args:
        path: Path to a Claude Code settings.json file
        force: Replace a conflicting non-ai-notify entry for an event
        dry_run: Do not write changes

    Returns:
        ClaudeHooksUpdate with changes and any skipped hooks.
    """
    data: dict[str, Any] = {}
    if path.exists():
        data = _load_json(path)

    updated_data, report = _update_hooks_data(data, force=force)

    # Every error path also reports changed=False, so this guard skips the write on errors too.
    if report.changed and not dry_run:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(updated_data, indent=2) + "\n")

    return ClaudeHooksUpdate(
        path=path,
        changed=report.changed,
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
            False, added, updated, skipped, ["Claude settings file is not a JSON object"]
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

    for spec in HOOK_SPECS:
        existing = hooks.get(spec.event)

        # Proper nested schema: append our group next to any existing hooks.
        if isinstance(existing, list):
            if _command_present(existing, spec.command):
                continue
            existing.append(_build_group(spec))
            added.append(spec.event)
            continue

        # No entry yet: create the event's hook list.
        if existing is None:
            hooks[spec.event] = [_build_group(spec)]
            added.append(spec.event)
            continue

        # Legacy/foreign non-list value (e.g. the old flat ``{"command": ...}`` form).
        if _command_present(existing, spec.command):
            # Old ai-notify flat entry — migrate it to the nested schema.
            hooks[spec.event] = [_build_group(spec)]
            updated.append(spec.event)
        elif force:
            hooks[spec.event] = [_build_group(spec)]
            updated.append(spec.event)
        else:
            skipped[spec.event] = _summarize_hook(existing)

    changed = bool(added or updated)
    return data, _HooksUpdateReport(changed, added, updated, skipped, errors)


def _build_group(spec: HookSpec) -> dict[str, Any]:
    """Build a Claude Code matcher group for a single command hook."""
    group: dict[str, Any] = {}
    if spec.matcher is not None:
        group["matcher"] = spec.matcher
    group["hooks"] = [{"type": "command", "command": spec.command}]
    return group


def iter_hook_commands(value: Any) -> Iterator[str]:
    """Yield every ``command`` string nested anywhere under a hooks value."""
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        command = value.get("command")
        if isinstance(command, str):
            yield command
        yield from iter_hook_commands(value.get("hooks"))
    elif isinstance(value, list):
        for item in value:
            yield from iter_hook_commands(item)


def _command_present(value: Any, command: str) -> bool:
    return any(found.strip() == command for found in iter_hook_commands(value))


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
