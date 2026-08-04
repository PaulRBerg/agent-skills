"""Hook specifications, installers, and configuration checks."""

from __future__ import annotations

import json
import os
import stat
import tempfile
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from ai_coord.jsonc import ArrayNode, JsoncDocument, ObjectNode


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
        "PostToolUseFailure",
        "ai-coord waker claude",
        matcher="Bash",
        timeout=3600,
        if_filter="Bash(ai-coord start *)",
        async_=True,
        async_rewake=True,
    ),
)


@dataclass(frozen=True, slots=True)
class LinkResult:
    path: Path
    changed: bool
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
            text = path.read_text(encoding="utf-8")
            if path.suffix.lower() != ".jsonc":
                json.loads(text)
            document = JsoncDocument.parse(text)
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise ValueError(f"could not parse {path}: {error}") from error
    else:
        document = JsoncDocument.parse("{}")
    if not isinstance(document.root, ObjectNode):
        raise TypeError(f"{path} must contain a JSON object")
    original = document.text
    hooks_member = document.member(document.root, "hooks")
    if hooks_member is None:
        document = document.insert_member(document.root, "hooks", {})
        hooks_member = _hooks_member(document)
    if not isinstance(hooks_member.value, ObjectNode):
        if not force:
            raise ValueError("hooks field must be an object; pass --force to replace it")
        document = document.replace_value(hooks_member.value, {})

    document, removed = _remove_stale_owned_commands(document, client, specs)
    for spec in specs:
        hooks = _hooks_object(document)
        event = document.member(hooks, spec.event)
        if event is not None and _spec_present(event.value.value, spec):
            continue
        if event is None:
            document = document.insert_member(hooks, spec.event, [_group(spec)])
            continue
        if not isinstance(event.value, ArrayNode):
            if not force:
                raise ValueError(f"hooks.{spec.event} must be a list; pass --force to replace it")
            document = document.replace_value(event.value, [])
            event = document.member(_hooks_object(document), spec.event)
            assert event is not None
        assert isinstance(event.value, ArrayNode)
        document = document.append_element(event.value, _group(spec))

    changed = document.text != original
    if changed and not dry_run:
        _write_config(path, document.text)
    return LinkResult(path, changed, removed)


def inspect_hooks(client: str, path: Path) -> HooksCheck:
    try:
        data = _read_config(path)
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


def default_link_path(client: str) -> Path:
    """Return the authoritative config source that link should update."""
    runtime_path = default_hook_path(client)
    if client == "claude":
        modular_source = runtime_path.parent / "settings" / "hooks.jsonc"
        if modular_source.exists():
            return modular_source
    return runtime_path


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


def _read_config(path: Path) -> Any:
    text = path.read_text(encoding="utf-8")
    if path.suffix.lower() == ".jsonc":
        return JsoncDocument.parse(text).value
    return json.loads(text)


def _write_config(path: Path, text: str) -> None:
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
            file.write(text)
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


def _remove_stale_owned_commands(
    document: JsoncDocument, client: str, specs: tuple[HookSpec, ...]
) -> tuple[JsoncDocument, int]:
    owned_commands = {f"ai-coord hook {client}", f"ai-coord waker {client}"}
    removed_legacy = 0
    preserved: set[HookSpec] = set()
    while True:
        hooks = _hooks_object(document)
        removed = False
        for event in hooks.members:
            if not isinstance(event.value, ArrayNode):
                continue
            for group_index, group_element in enumerate(event.value.elements):
                if not isinstance(group_element.value, ObjectNode):
                    continue
                handlers_member = document.member(group_element.value, "hooks")
                if handlers_member is None or not isinstance(handlers_member.value, ArrayNode):
                    continue
                for handler_index, handler_element in enumerate(handlers_member.value.elements):
                    handler = handler_element.value.value
                    command = handler.get("command") if isinstance(handler, dict) else None
                    if not isinstance(command, str):
                        continue
                    legacy = _is_legacy(command, client)
                    matching = (
                        _matching_spec(event.key, group_element.value.value, handler, specs)
                        if command.strip() in owned_commands
                        else None
                    )
                    if not legacy and (matching is not None and matching not in preserved):
                        preserved.add(matching)
                        continue
                    if not legacy and command.strip() not in owned_commands:
                        continue
                    if legacy:
                        removed_legacy += 1
                    document = document.remove_element(handlers_member.value, handler_index)
                    document = _prune_empty_group(document, event.key, group_index)
                    removed = True
                    break
                if removed:
                    break
            if removed:
                break
        if not removed:
            return document, removed_legacy


def _hooks_member(document: JsoncDocument):
    assert isinstance(document.root, ObjectNode)
    member = document.member(document.root, "hooks")
    assert member is not None
    return member


def _hooks_object(document: JsoncDocument) -> ObjectNode:
    member = _hooks_member(document)
    assert isinstance(member.value, ObjectNode)
    return member.value


def _prune_empty_group(document: JsoncDocument, event_name: str, group_index: int) -> JsoncDocument:
    hooks = _hooks_object(document)
    event_index = next(
        index for index, member in enumerate(hooks.members) if member.key == event_name
    )
    event = hooks.members[event_index]
    assert isinstance(event.value, ArrayNode)
    group = event.value.elements[group_index]
    if not isinstance(group.value, ObjectNode):
        return document
    handlers = document.member(group.value, "hooks")
    if handlers is None or not isinstance(handlers.value, ArrayNode) or handlers.value.elements:
        return document
    document = document.remove_element(event.value, group_index)
    hooks = _hooks_object(document)
    event_index = next(
        index for index, member in enumerate(hooks.members) if member.key == event_name
    )
    event = hooks.members[event_index]
    assert isinstance(event.value, ArrayNode)
    return document.remove_member(hooks, event_index) if not event.value.elements else document


def _matching_spec(
    event: str, group: dict[str, Any], handler: dict[str, Any], specs: tuple[HookSpec, ...]
) -> HookSpec | None:
    """Return the spec this already-installed handler satisfies, if any."""
    return next(
        (spec for spec in specs if spec.event == event and _handler_matches(group, handler, spec)),
        None,
    )


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
        handlers = group.get("hooks")
        if not isinstance(handlers, list):
            continue
        for handler in handlers:
            if isinstance(handler, dict) and _handler_matches(group, handler, spec):
                return True
    return False


def _handler_matches(group: dict[str, Any], handler: dict[str, Any], spec: HookSpec) -> bool:
    if spec.matcher is not None and group.get("matcher") != spec.matcher:
        return False
    if spec.matcher is None and group.get("matcher") not in (None, "", "*"):
        return False
    if handler.get("type") != "command":
        return False
    command = handler.get("command")
    if not isinstance(command, str) or command.strip() != spec.command:
        return False
    if spec.timeout is not None and handler.get("timeout") != spec.timeout:
        return False
    if (
        spec.additional_context_limit is not None
        and handler.get("additionalContextLimit") != spec.additional_context_limit
    ):
        return False
    if spec.if_filter is not None and handler.get("if") != spec.if_filter:
        return False
    if spec.async_ is not None and handler.get("async") is not spec.async_:
        return False
    return spec.async_rewake is None or handler.get("asyncRewake") is spec.async_rewake
