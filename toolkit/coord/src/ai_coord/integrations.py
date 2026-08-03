"""Hook specifications, installers, and configuration checks."""

from __future__ import annotations

import copy
import json
import os
import stat
import tempfile
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True, slots=True)
class HookSpec:
    event: str
    command: str
    matcher: str | None = None
    timeout: int | None = None
    additional_context_limit: int | None = None
    if_filter: str | None = None
    async_: bool | None = None
    async_rewake: bool | None = None


CODEX_HOOK_SPECS = (
    HookSpec("SessionStart", "ai-coord hook codex", timeout=5),
    HookSpec(
        "UserPromptSubmit",
        "ai-coord hook codex",
        timeout=5,
        additional_context_limit=200,
    ),
    HookSpec("Stop", "ai-coord hook codex", timeout=5),
    HookSpec("SessionEnd", "ai-coord hook codex", timeout=3),
    HookSpec("SubagentStart", "ai-coord hook codex", timeout=5),
    HookSpec("SubagentStop", "ai-coord hook codex", timeout=5),
    HookSpec("PostToolUse", "ai-coord hook codex", timeout=5),
)
CLAUDE_HOOK_SPECS = (
    HookSpec("SessionStart", "ai-coord hook claude", timeout=5),
    HookSpec("UserPromptSubmit", "ai-coord hook claude", timeout=5),
    HookSpec("Stop", "ai-coord hook claude", timeout=5),
    HookSpec("SessionEnd", "ai-coord hook claude", timeout=3),
    HookSpec("SubagentStart", "ai-coord hook claude", timeout=5),
    HookSpec("SubagentStop", "ai-coord hook claude", timeout=5),
    HookSpec("PostToolUse", "ai-coord hook claude", matcher="ExitPlanMode", timeout=5),
    HookSpec("PostToolBatch", "ai-coord hook claude", timeout=5),
    HookSpec(
        "PostToolUse",
        "ai-coord waker claude",
        matcher="Bash",
        timeout=3600,
        if_filter="Bash(ai-coord start*)",
        async_=True,
        async_rewake=True,
    ),
)


@dataclass(frozen=True, slots=True)
class LinkResult:
    path: Path
    changed: bool
    added: tuple[str, ...]
    removed_legacy: int


@dataclass(frozen=True, slots=True)
class HooksCheck:
    client: str
    path: Path
    ok: bool
    missing: tuple[str, ...]
    legacy_commands: tuple[str, ...]
    error: str | None = None


def link_hooks(
    client: str, path: Path, *, dry_run: bool = False, force: bool = False
) -> LinkResult:
    specs = hook_specs(client)
    if path.exists():
        try:
            data = json.loads(path.read_text())
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise ValueError(f"could not parse {path}: {error}") from error
    else:
        data = {}
    if not isinstance(data, dict):
        raise TypeError(f"{path} must contain a JSON object")
    original = copy.deepcopy(data)
    hooks = data.get("hooks")
    if hooks is None:
        hooks = {}
        data["hooks"] = hooks
    if not isinstance(hooks, dict):
        if not force:
            raise ValueError("hooks field must be an object; pass --force to replace it")
        hooks = {}
        data["hooks"] = hooks

    removed = _remove_owned_commands(hooks, client)
    added: list[str] = []
    for spec in specs:
        groups = hooks.get(spec.event)
        if groups is None:
            groups = []
            hooks[spec.event] = groups
        if not isinstance(groups, list):
            if not force:
                raise ValueError(f"hooks.{spec.event} must be a list; pass --force to replace it")
            groups = []
            hooks[spec.event] = groups
        groups.append(_group(spec))
        added.append(spec.event)

    changed = data != original
    if changed and not dry_run:
        _write_config(path, data)
    return LinkResult(path, changed, tuple(added), removed)


def inspect_hooks(client: str, path: Path) -> HooksCheck:
    try:
        data = json.loads(path.read_text())
    except FileNotFoundError:
        return HooksCheck(client, path, False, tuple(spec.event for spec in hook_specs(client)), ())
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        return HooksCheck(client, path, False, (), (), str(error))
    hooks = data.get("hooks") if isinstance(data, dict) else None
    if not isinstance(hooks, dict):
        return HooksCheck(client, path, False, (), (), "hooks field is not an object")
    missing = tuple(
        spec.event for spec in hook_specs(client) if not _spec_present(hooks.get(spec.event), spec)
    )
    legacy = tuple(command for command in iter_commands(hooks) if _is_legacy(command, client))
    return HooksCheck(client, path, not missing and not legacy, missing, legacy)


def hook_specs(client: str) -> tuple[HookSpec, ...]:
    if client == "codex":
        return CODEX_HOOK_SPECS
    if client == "claude":
        return CLAUDE_HOOK_SPECS
    raise ValueError(f"unsupported client: {client}")


def default_hook_path(client: str) -> Path:
    if client == "codex":
        root = Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")
        return root.expanduser() / "hooks.json"
    if client == "claude":
        root = Path(os.environ.get("CLAUDE_CONFIG_DIR") or Path.home() / ".claude")
        return root.expanduser() / "settings.json"
    raise ValueError(f"unsupported client: {client}")


def iter_commands(value: Any) -> Iterator[str]:
    if isinstance(value, dict):
        command = value.get("command")
        if isinstance(command, str):
            yield command
        for nested in value.values():
            yield from iter_commands(nested)
    elif isinstance(value, list):
        for nested in value:
            yield from iter_commands(nested)


def _write_config(path: Path, data: dict[str, Any]) -> None:
    target = path.resolve(strict=False) if path.is_symlink() else path
    target.parent.mkdir(parents=True, exist_ok=True)
    mode = stat.S_IMODE(target.stat().st_mode) if target.exists() else 0o600
    descriptor, temporary_name = tempfile.mkstemp(
        dir=target.parent,
        prefix=f".{target.name}.",
        suffix=".tmp",
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as file:
            file.write(json.dumps(data, indent=2) + "\n")
            file.flush()
            os.fsync(file.fileno())
        temporary.chmod(mode)
        os.replace(temporary, target)
    finally:
        temporary.unlink(missing_ok=True)


def _group(spec: HookSpec) -> dict[str, Any]:
    group: dict[str, Any] = {}
    if spec.matcher is not None:
        group["matcher"] = spec.matcher
    handler: dict[str, Any] = {"type": "command", "command": spec.command}
    if spec.timeout is not None:
        handler["timeout"] = spec.timeout
    if spec.additional_context_limit is not None:
        handler["additionalContextLimit"] = spec.additional_context_limit
    if spec.if_filter is not None:
        handler["if"] = spec.if_filter
    if spec.async_ is not None:
        handler["async"] = spec.async_
    if spec.async_rewake is not None:
        handler["asyncRewake"] = spec.async_rewake
    group["hooks"] = [handler]
    return group


def _remove_owned_commands(hooks: dict[str, Any], client: str) -> int:
    removed_legacy = 0
    for event, groups in list(hooks.items()):
        if not isinstance(groups, list):
            continue
        kept_groups: list[Any] = []
        for group in groups:
            if not isinstance(group, dict) or not isinstance(group.get("hooks"), list):
                kept_groups.append(group)
                continue
            handlers: list[Any] = []
            for handler in group["hooks"]:
                command = handler.get("command") if isinstance(handler, dict) else None
                if isinstance(command, str):
                    if _is_legacy(command, client):
                        removed_legacy += 1
                        continue
                    if command.strip() in {
                        f"ai-coord hook {client}",
                        f"ai-coord waker {client}",
                    }:
                        continue
                handlers.append(handler)
            if handlers:
                updated = dict(group)
                updated["hooks"] = handlers
                kept_groups.append(updated)
        if kept_groups:
            hooks[event] = kept_groups
        else:
            hooks.pop(event, None)
    return removed_legacy


def _is_legacy(command: str, client: str) -> bool:
    if "AgentSessionStatus/agent_session_status.py" in command:
        return True
    return client == "claude" and command.rstrip().endswith("/plan_claim.py")


def _spec_present(value: Any, spec: HookSpec) -> bool:
    if not isinstance(value, list):
        return False
    for group in value:
        if not isinstance(group, dict):
            continue
        if spec.matcher is not None and group.get("matcher") != spec.matcher:
            continue
        if spec.matcher is None and group.get("matcher") not in (None, "", "*"):
            continue
        handlers = group.get("hooks")
        if not isinstance(handlers, list):
            continue
        for handler in handlers:
            if not isinstance(handler, dict) or handler.get("type") != "command":
                continue
            command = handler.get("command")
            if not isinstance(command, str) or command.strip() != spec.command:
                continue
            if spec.timeout is not None and handler.get("timeout") != spec.timeout:
                continue
            if (
                spec.additional_context_limit is not None
                and handler.get("additionalContextLimit") != spec.additional_context_limit
            ):
                continue
            if spec.if_filter is not None and handler.get("if") != spec.if_filter:
                continue
            if spec.async_ is not None and handler.get("async") is not spec.async_:
                continue
            if (
                spec.async_rewake is not None
                and handler.get("asyncRewake") is not spec.async_rewake
            ):
                continue
            return True
    return False
