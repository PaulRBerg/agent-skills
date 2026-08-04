"""Hook specifications, installers, and configuration checks."""

from __future__ import annotations

import json
import os
import re
import select
import stat
import subprocess
import tempfile
import time
from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal, Self

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
    HookSpec(
        "SessionStart",
        "ai-coord hook codex",
        matcher="startup|resume|clear",
        timeout=5,
    ),
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
    trust: Literal["updated", "unchanged", "skipped"] = "skipped"


@dataclass(frozen=True, slots=True)
class HooksCheck:
    client: str
    path: Path
    ok: bool
    missing: tuple[str, ...]
    legacy_commands: tuple[str, ...]
    error: str | None = None


@dataclass(frozen=True, slots=True)
class HookCheck:
    """The app-server's read-only assessment of ai-coord's Codex hooks."""

    ok: bool
    path: Path
    error: str | None = None
    details: dict[str, object] = field(default_factory=dict)


class CodexTrustError(RuntimeError):
    """Codex could not prove or update the narrowly owned hook trust state."""


class _CodexVersionConflict(CodexTrustError):
    pass


_CODEX_TIMEOUT_SECONDS = 10
_CODEX_MINIMUM_VERSION = (0, 146, 0)
_CODEX_MINIMUM_VERSION_TEXT = ".".join(str(part) for part in _CODEX_MINIMUM_VERSION)
_CODEX_VERSION_PATTERN = re.compile(
    r"^codex-cli (?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)"
    r"(?:-(?P<prerelease>[0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$"
)
_CODEX_EVENT_NAMES = {
    "SessionStart": "sessionStart",
    "UserPromptSubmit": "userPromptSubmit",
    "Stop": "stop",
    "SessionEnd": "sessionEnd",
    "SubagentStart": "subagentStart",
    "SubagentStop": "subagentStop",
    "PostToolUse": "postToolUse",
}


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


def trust_codex_hooks(
    path: Path | None = None, *, dry_run: bool = False
) -> Literal["updated", "unchanged", "skipped"]:
    """Trust only the seven exact ai-coord hooks in the active Codex source.

    The hooks file is written by :func:`link_hooks`; this operation only changes
    Codex's user config through its compare-and-swap app-server API.
    """
    hooks_path = _active_codex_hook_path(path)
    if dry_run:
        return "skipped"
    _require_codex_minimum_version()

    last_conflict: _CodexVersionConflict | None = None
    for attempt in range(3):
        try:
            with _CodexAppServer() as server:
                hooks = _owned_codex_hooks(server.request("hooks/list", {}), hooks_path)
                if all(hook["trustStatus"] == "trusted" for hook in hooks.values()):
                    return "unchanged"
                config = _user_config_layer(server.request("config/read", {"includeLayers": True}))
                edits = [
                    {
                        "keyPath": _codex_trust_key_path(key),
                        "value": hook["currentHash"],
                        "mergeStrategy": "upsert",
                    }
                    for key, hook in hooks.items()
                ]
                write_result = server.request(
                    "config/batchWrite",
                    {
                        "edits": edits,
                        "expectedVersion": config["version"],
                        "filePath": config["filePath"],
                    },
                )
                _validate_config_write(write_result, config["filePath"])
                expected_hashes = {key: hook["currentHash"] for key, hook in hooks.items()}
            with _CodexAppServer() as server:
                verified = _owned_codex_hooks(server.request("hooks/list", {}), hooks_path)
            if all(
                verified[key]["trustStatus"] == "trusted"
                and verified[key]["currentHash"] == expected_hash
                for key, expected_hash in expected_hashes.items()
            ):
                return "updated"
            raise CodexTrustError("Codex did not verify the submitted hook trust state")
        except _CodexVersionConflict as error:
            last_conflict = error
        if attempt == 2:
            break
    raise last_conflict or CodexTrustError("Codex hook trust did not converge")


def inspect_codex_hook_trust(path: Path | None = None) -> HookCheck:
    """Read Codex's trust state for exactly the hooks this integration owns."""
    try:
        hooks_path = _active_codex_hook_path(path)
        _require_codex_minimum_version()
        with _CodexAppServer() as server:
            hooks = _owned_codex_hooks(server.request("hooks/list", {}), hooks_path)
        details: dict[str, object] = {
            "hooks": {
                key: {"hash": hook["currentHash"], "trust": hook["trustStatus"]}
                for key, hook in hooks.items()
            }
        }
        untrusted = tuple(key for key, hook in hooks.items() if hook["trustStatus"] != "trusted")
        if untrusted:
            return HookCheck(
                False,
                hooks_path,
                "owned Codex hooks are not trusted",
                {**details, "untrusted": untrusted},
            )
        return HookCheck(True, hooks_path, details=details)
    except (CodexTrustError, OSError, ValueError) as error:
        requested = path if path is not None else default_hook_path("codex")
        return HookCheck(False, requested, str(error))


def _active_codex_hook_path(path: Path | None) -> Path:
    active = default_hook_path("codex").expanduser().resolve(strict=False)
    selected = (path or active).expanduser().resolve(strict=False)
    if selected != active:
        raise ValueError(f"Codex hooks path must be the active source: {active}")
    return active


def _require_codex_minimum_version() -> None:
    try:
        result = subprocess.run(
            ["codex", "--version"],
            capture_output=True,
            check=False,
            encoding="utf-8",
            timeout=_CODEX_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise CodexTrustError(f"could not determine Codex version: {error}") from error
    if result.returncode != 0:
        detail = result.stderr.strip() or f"exit {result.returncode}"
        raise CodexTrustError(f"could not determine Codex version: {detail}")

    output = result.stdout.strip()
    match = _CODEX_VERSION_PATTERN.fullmatch(output)
    if match is None:
        raise CodexTrustError("could not parse `codex --version` output")
    version = tuple(int(match[name]) for name in ("major", "minor", "patch"))
    if version < _CODEX_MINIMUM_VERSION or (
        version == _CODEX_MINIMUM_VERSION and match["prerelease"] is not None
    ):
        raise CodexTrustError(
            f"Codex hook trust requires codex-cli >= {_CODEX_MINIMUM_VERSION_TEXT}; "
            f"found {output.removeprefix('codex-cli ')}"
        )


def _owned_codex_hooks(response: object, hooks_path: Path) -> dict[str, dict[str, Any]]:
    if not isinstance(response, dict) or not isinstance(response.get("data"), list):
        raise CodexTrustError("malformed hooks/list response")
    found: dict[str, dict[str, Any]] = {}
    for entry in response["data"]:
        if not isinstance(entry, dict) or not isinstance(entry.get("hooks"), list):
            raise CodexTrustError("malformed hooks/list entry")
        if entry.get("errors"):
            raise CodexTrustError("Codex reported hook loading errors")
        for hook in entry["hooks"]:
            if not isinstance(hook, dict):
                raise CodexTrustError("malformed hook metadata")
            matching = _matching_codex_spec(hook, hooks_path)
            if matching is None:
                continue
            if matching.event in found:
                raise CodexTrustError(f"duplicate owned Codex hook: {matching.event}")
            found[matching.event] = hook
    missing = [spec.event for spec in CODEX_HOOK_SPECS if spec.event not in found]
    if missing:
        raise CodexTrustError(f"missing exact Codex hooks: {', '.join(missing)}")
    by_key = {hook["key"]: hook for hook in found.values()}
    if len(by_key) != len(found):
        raise CodexTrustError("duplicate Codex hook key")
    return by_key


def _matching_codex_spec(hook: dict[str, Any], hooks_path: Path) -> HookSpec | None:
    source_path = hook.get("sourcePath")
    if not isinstance(source_path, str):
        return None
    try:
        is_active_source = Path(source_path).expanduser().resolve(strict=False) == hooks_path
    except OSError:
        return None
    if not is_active_source:
        return None
    for spec in CODEX_HOOK_SPECS:
        if (
            hook.get("eventName") == _CODEX_EVENT_NAMES[spec.event]
            and hook.get("enabled") is True
            and hook.get("isManaged") is False
            and hook.get("source") == "user"
            and hook.get("handlerType") == "command"
            and hook.get("command") == spec.command
            and hook.get("matcher") == spec.matcher
            and hook.get("timeoutSec") == spec.timeout
            and hook.get("additionalContextLimit") == spec.additional_context_limit
            and isinstance(hook.get("key"), str)
            and bool(hook["key"])
            and isinstance(hook.get("currentHash"), str)
            and bool(hook["currentHash"])
            and hook.get("trustStatus") in {"trusted", "untrusted", "modified"}
        ):
            return spec
    return None


def _user_config_layer(response: object) -> dict[str, str]:
    if not isinstance(response, dict) or not isinstance(response.get("layers"), list):
        raise CodexTrustError("malformed config/read response")
    expected = (default_hook_path("codex").parent / "config.toml").resolve(strict=False)
    for layer in response["layers"]:
        if not isinstance(layer, dict) or not isinstance(layer.get("name"), dict):
            continue
        name = layer["name"]
        file_path = name.get("file")
        if (
            name.get("type") == "user"
            and isinstance(file_path, str)
            and Path(file_path).expanduser().resolve(strict=False) == expected
            and isinstance(layer.get("version"), str)
            and layer["version"]
        ):
            return {"filePath": file_path, "version": layer["version"]}
    raise CodexTrustError(f"missing active Codex user config layer: {expected}")


def _codex_trust_key_path(key: str) -> str:
    """Quote the server's opaque hook key as one TOML basic-string segment."""
    return f"hooks.state.{json.dumps(key, ensure_ascii=False)}.trusted_hash"


def _validate_config_write(response: object, expected_path: str) -> None:
    if not isinstance(response, dict):
        raise CodexTrustError("malformed config/batchWrite response")
    file_path = response.get("filePath")
    version = response.get("version")
    try:
        path_matches = isinstance(file_path, str) and Path(file_path).expanduser().resolve(
            strict=False
        ) == Path(expected_path).expanduser().resolve(strict=False)
    except OSError:
        path_matches = False
    if (
        not path_matches
        or response.get("status") not in {"ok", "okOverridden"}
        or not isinstance(version, str)
        or not version
    ):
        raise CodexTrustError("malformed config/batchWrite response")


class _CodexAppServer:
    """Minimal JSONL client for compatible Codex 0.146.0+ app-server schemas."""

    def __init__(self) -> None:
        self._process: subprocess.Popen[str] | None = None
        self._next_id = 1

    def __enter__(self) -> Self:
        try:
            self._process = subprocess.Popen(
                ["codex", "app-server", "--stdio"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                encoding="utf-8",
                bufsize=1,
            )
        except OSError as error:
            raise CodexTrustError(f"could not start Codex app-server: {error}") from error
        try:
            result = self.request(
                "initialize",
                {"clientInfo": {"name": "ai-coord", "version": "0"}},
            )
            if not isinstance(result, dict):
                raise CodexTrustError("malformed initialize response")
            self.notify("initialized", {})
        except BaseException:
            self.__exit__()
            raise
        return self

    def __exit__(self, *_: object) -> None:
        if self._process is None:
            return
        if self._process.stdin is not None:
            try:
                self._process.stdin.close()
            except OSError:
                pass
        try:
            self._process.wait(timeout=_CODEX_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            self._process.terminate()
            try:
                self._process.wait(timeout=_CODEX_TIMEOUT_SECONDS)
            except subprocess.TimeoutExpired:
                self._process.kill()
                self._process.wait(timeout=_CODEX_TIMEOUT_SECONDS)

    def notify(self, method: str, params: dict[str, object]) -> None:
        self._write({"jsonrpc": "2.0", "method": method, "params": params})

    def request(self, method: str, params: dict[str, object]) -> object:
        request_id = self._next_id
        self._next_id += 1
        self._write({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        deadline = time.monotonic() + _CODEX_TIMEOUT_SECONDS
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise CodexTrustError("Codex app-server response timed out")
            response = self._read_response(remaining)
            if response.get("id") != request_id:
                continue
            if "error" in response:
                error = response["error"]
                if (
                    method == "config/batchWrite"
                    and isinstance(error, dict)
                    and error.get("code") == -32600
                    and isinstance(error.get("data"), dict)
                    and error["data"].get("config_write_error_code") == "configVersionConflict"
                ):
                    raise _CodexVersionConflict("Codex config version conflict")
                raise CodexTrustError(f"Codex {method} failed: {response['error']}")
            if "result" not in response:
                raise CodexTrustError(f"malformed {method} response")
            return response["result"]

    def _write(self, message: dict[str, object]) -> None:
        if self._process is None or self._process.stdin is None:
            raise CodexTrustError("Codex app-server stdin is unavailable")
        try:
            self._process.stdin.write(json.dumps(message) + "\n")
            self._process.stdin.flush()
        except OSError as error:
            raise CodexTrustError(f"could not write Codex app-server request: {error}") from error

    def _read_response(self, timeout: float = _CODEX_TIMEOUT_SECONDS) -> dict[str, object]:
        if self._process is None or self._process.stdout is None:
            raise CodexTrustError("Codex app-server stdout is unavailable")
        ready, _, _ = select.select([self._process.stdout], [], [], timeout)
        if not ready:
            raise CodexTrustError("Codex app-server response timed out")
        line = self._process.stdout.readline()
        if not line:
            raise CodexTrustError("Codex app-server closed stdout")
        try:
            response = json.loads(line)
        except json.JSONDecodeError as error:
            raise CodexTrustError("Codex app-server emitted malformed JSON") from error
        if not isinstance(response, dict):
            raise CodexTrustError("Codex app-server emitted malformed JSON-RPC")
        # Codex 0.146.0 omits the JSON-RPC version marker on its JSONL output.
        if response.get("jsonrpc") not in (None, "2.0") or not (
            "id" in response or "method" in response
        ):
            raise CodexTrustError("Codex app-server emitted malformed JSON-RPC")
        return response


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
