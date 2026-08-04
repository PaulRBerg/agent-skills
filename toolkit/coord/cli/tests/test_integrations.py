from __future__ import annotations

import io
import json
import os
import shutil
import stat
from pathlib import Path

import pytest
from hypothesis import HealthCheck, example, given, settings
from hypothesis import strategies as st

import ai_coord.integrations as integrations_module
from ai_coord.integrations import (
    CODEX_HOOK_SPECS,
    CodexTrustError,
    default_hook_path,
    default_link_path,
    inspect_codex_hook_trust,
    inspect_hooks,
    link_hooks,
    trust_codex_hooks,
)
from ai_coord.jsonc import JsoncDocument

JSON_SCALARS = st.none() | st.booleans() | st.integers() | st.text()


def _codex_hook(
    spec: integrations_module.HookSpec, source_path: Path, *, trust: str = "untrusted"
) -> dict[str, object]:
    event_names = {
        "SessionStart": "sessionStart",
        "UserPromptSubmit": "userPromptSubmit",
        "Stop": "stop",
        "SessionEnd": "sessionEnd",
        "SubagentStart": "subagentStart",
        "SubagentStop": "subagentStop",
        "PostToolUse": "postToolUse",
    }
    return {
        "key": f'key.{spec.event}."quoted"',
        "currentHash": f"hash-{spec.event}",
        "enabled": True,
        "eventName": event_names[spec.event],
        "handlerType": "command",
        "isManaged": False,
        "source": "user",
        "sourcePath": str(source_path),
        "command": spec.command,
        "matcher": spec.matcher,
        "timeoutSec": spec.timeout,
        "additionalContextLimit": spec.additional_context_limit,
        "trustStatus": trust,
    }


def _hooks_response(source_path: Path, *, trust: str = "untrusted") -> dict[str, object]:
    return {
        "data": [
            {
                "cwd": str(source_path.parent),
                "errors": [],
                "warnings": [],
                "hooks": [_codex_hook(spec, source_path, trust=trust) for spec in CODEX_HOOK_SPECS],
            }
        ]
    }


@st.composite
def _handler_order_case(draw: st.DrawFn) -> tuple[list[str], int]:
    commands = draw(
        st.lists(
            st.text(alphabet="abcdefghijklmnopqrstuvwxyz-", min_size=1, max_size=16).map(
                lambda command: f"custom-{command}"
            ),
            unique=True,
            max_size=8,
        )
    )
    position = draw(st.integers(min_value=0, max_value=len(commands)))
    return commands, position


@pytest.mark.parametrize("initial_trust", ("untrusted", "modified"))
def test_codex_trust_batches_only_exact_owned_hooks(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, initial_trust: str
) -> None:
    codex_home = tmp_path / "codex"
    hooks_path = codex_home / "hooks.json"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    requests: list[tuple[str, dict[str, object]]] = []
    responses = iter(
        [
            {"hooks/list": _hooks_response(hooks_path, trust=initial_trust)},
            {
                "config/read": {
                    "layers": [
                        {
                            "name": {"type": "user", "file": str(codex_home / "config.toml")},
                            "version": "v1",
                        }
                    ]
                }
            },
            {
                "config/batchWrite": {
                    "filePath": str(codex_home / "config.toml"),
                    "status": "ok",
                    "version": "v2",
                }
            },
            {"hooks/list": _hooks_response(hooks_path, trust="trusted")},
        ]
    )
    servers: list[object] = []

    class FakeServer:
        def __init__(self) -> None:
            servers.append(self)

        def __enter__(self):
            return self

        def __exit__(self, *_: object) -> None:
            return None

        def request(self, method: str, params: dict[str, object]) -> object:
            requests.append((method, params))
            response = next(responses)
            return response[method]

    monkeypatch.setattr(integrations_module, "_CodexAppServer", FakeServer)

    assert trust_codex_hooks() == "updated"
    batch = requests[2][1]
    assert batch["expectedVersion"] == "v1"
    assert batch["filePath"] == str(codex_home / "config.toml")
    edits = batch["edits"]
    assert isinstance(edits, list) and len(edits) == 7
    assert all(edit["mergeStrategy"] == "upsert" for edit in edits if isinstance(edit, dict))
    assert all('."key.' in edit["keyPath"] for edit in edits if isinstance(edit, dict))
    assert len(servers) == 2


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("eventName", "sessionEnd"),
        ("enabled", False),
        ("isManaged", True),
        ("source", "project"),
        ("sourcePath", "/not-the-active-hooks-file"),
        ("handlerType", "prompt"),
        ("command", "ai-coord hook codex "),
        ("matcher", "*"),
        ("timeoutSec", 6),
        ("additionalContextLimit", 0),
        ("key", ""),
        ("currentHash", ""),
        ("trustStatus", "managed"),
    ),
)
def test_codex_owned_hook_filter_rejects_near_matches(
    tmp_path: Path, field: str, value: object
) -> None:
    hooks_path = tmp_path / "hooks.json"
    hook = _codex_hook(CODEX_HOOK_SPECS[0], hooks_path)
    hook[field] = value

    assert integrations_module._matching_codex_spec(hook, hooks_path) is None


def test_codex_trust_key_path_quotes_the_opaque_key_as_one_toml_segment() -> None:
    key = 'path.with.dot:"quote"\\slash\nline'

    assert integrations_module._codex_trust_key_path(key) == (
        r'hooks.state."path.with.dot:\"quote\"\\slash\nline".trusted_hash'
    )


@pytest.mark.parametrize(
    "response",
    (
        None,
        {},
        {"filePath": "/wrong/config.toml", "status": "ok", "version": "v2"},
        {"filePath": "/active/config.toml", "status": "unexpected", "version": "v2"},
        {"filePath": "/active/config.toml", "status": "ok", "version": ""},
    ),
)
def test_codex_config_write_response_is_strict(response: object) -> None:
    with pytest.raises(CodexTrustError, match="malformed config/batchWrite response"):
        integrations_module._validate_config_write(response, "/active/config.toml")


def test_codex_trust_is_noop_when_exact_hooks_are_trusted(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    codex_home = tmp_path / "codex"
    hooks_path = codex_home / "hooks.json"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    methods: list[str] = []

    class FakeServer:
        def __enter__(self):
            return self

        def __exit__(self, *_: object) -> None:
            return None

        def request(self, method: str, _: dict[str, object]) -> object:
            methods.append(method)
            assert method == "hooks/list"
            return _hooks_response(hooks_path, trust="trusted")

    monkeypatch.setattr(integrations_module, "_CodexAppServer", FakeServer)
    assert trust_codex_hooks() == "unchanged"
    assert methods == ["hooks/list"]
    assert trust_codex_hooks(dry_run=True) == "skipped"


def test_codex_trust_rejects_non_active_hook_path(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    monkeypatch.setenv("CODEX_HOME", str(tmp_path / "codex"))
    with pytest.raises(ValueError, match="active source"):
        trust_codex_hooks(tmp_path / "other-hooks.json", dry_run=True)


def test_codex_trust_retries_only_config_version_conflict(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    codex_home = tmp_path / "codex"
    hooks_path = codex_home / "hooks.json"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    sequence = iter(
        [
            ("hooks/list", _hooks_response(hooks_path)),
            (
                "config/read",
                {
                    "layers": [
                        {
                            "name": {"type": "user", "file": str(codex_home / "config.toml")},
                            "version": "v1",
                        }
                    ]
                },
            ),
            ("config/batchWrite", integrations_module._CodexVersionConflict("conflict")),
            ("hooks/list", _hooks_response(hooks_path)),
            (
                "config/read",
                {
                    "layers": [
                        {
                            "name": {"type": "user", "file": str(codex_home / "config.toml")},
                            "version": "v2",
                        }
                    ]
                },
            ),
            (
                "config/batchWrite",
                {
                    "filePath": str(codex_home / "config.toml"),
                    "status": "ok",
                    "version": "v3",
                },
            ),
            ("hooks/list", _hooks_response(hooks_path, trust="trusted")),
        ]
    )

    class FakeServer:
        def __enter__(self):
            return self

        def __exit__(self, *_: object) -> None:
            return None

        def request(self, method: str, _: dict[str, object]) -> object:
            expected, result = next(sequence)
            assert method == expected
            if isinstance(result, Exception):
                raise result
            return result

    monkeypatch.setattr(integrations_module, "_CodexAppServer", FakeServer)
    assert trust_codex_hooks() == "updated"


def test_codex_trust_stops_after_three_fresh_config_version_conflicts(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    codex_home = tmp_path / "codex"
    hooks_path = codex_home / "hooks.json"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    instances: list[int] = []
    submitted_hashes: list[set[object]] = []

    class FakeServer:
        def __init__(self) -> None:
            self.attempt = len(instances) + 1
            instances.append(self.attempt)

        def __enter__(self):
            return self

        def __exit__(self, *_: object) -> None:
            return None

        def request(self, method: str, params: dict[str, object]) -> object:
            if method == "hooks/list":
                response = _hooks_response(hooks_path)
                for hook in response["data"][0]["hooks"]:  # type: ignore[index]
                    hook["currentHash"] = f"attempt-{self.attempt}-{hook['eventName']}"
                return response
            if method == "config/read":
                return {
                    "layers": [
                        {
                            "name": {"type": "user", "file": str(codex_home / "config.toml")},
                            "version": f"v{self.attempt}",
                        }
                    ]
                }
            assert method == "config/batchWrite"
            edits = params["edits"]
            assert isinstance(edits, list)
            submitted_hashes.append({edit["value"] for edit in edits if isinstance(edit, dict)})
            raise integrations_module._CodexVersionConflict("conflict")

    monkeypatch.setattr(integrations_module, "_CodexAppServer", FakeServer)

    with pytest.raises(CodexTrustError, match="conflict"):
        trust_codex_hooks()

    assert instances == [1, 2, 3]
    assert len(submitted_hashes) == 3
    assert all(
        all(str(value).startswith(f"attempt-{attempt}-") for value in hashes)
        for attempt, hashes in enumerate(submitted_hashes, start=1)
    )


def test_codex_trust_inspection_fails_closed_for_missing_exact_hook(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    codex_home = tmp_path / "codex"
    hooks_path = codex_home / "hooks.json"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))

    class FakeServer:
        def __enter__(self):
            return self

        def __exit__(self, *_: object) -> None:
            return None

        def request(self, _: str, __: dict[str, object]) -> object:
            response = _hooks_response(hooks_path, trust="trusted")
            response["data"][0]["hooks"].pop()  # type: ignore[index]
            return response

    monkeypatch.setattr(integrations_module, "_CodexAppServer", FakeServer)
    check = inspect_codex_hook_trust()
    assert not check.ok
    assert check.error is not None and "missing exact" in check.error


def test_codex_app_server_rejects_timeout_and_malformed_json(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class FakeProcess:
        def __init__(self) -> None:
            self.stdin = io.StringIO()
            self.stdout = io.StringIO("not-json\n")

        def wait(self, timeout: int) -> None:
            return None

        def terminate(self) -> None:
            return None

        def kill(self) -> None:
            return None

    monkeypatch.setattr(integrations_module.subprocess, "Popen", lambda *_, **__: FakeProcess())
    monkeypatch.setattr(integrations_module.select, "select", lambda *_: ([_[0][0]], [], []))
    with pytest.raises(CodexTrustError, match="malformed JSON"):
        integrations_module._CodexAppServer().__enter__()

    monkeypatch.setattr(integrations_module.select, "select", lambda *_: ([], [], []))
    with pytest.raises(CodexTrustError, match="timed out"):
        integrations_module._CodexAppServer().__enter__()


@pytest.mark.skipif(
    not os.environ.get("AI_COORD_TEST_CODEX_HOOK_TRUST") or shutil.which("codex") is None,
    reason="set AI_COORD_TEST_CODEX_HOOK_TRUST with Codex installed to exercise live hook trust",
)
def test_live_codex_trust_uses_isolated_home_and_preserves_unrelated_hook(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    codex_home = tmp_path / "codex"
    hooks_path = codex_home / "hooks.json"
    codex_home.mkdir()
    hooks_path.write_text(
        json.dumps(
            {"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "unrelated-hook"}]}]}}
        )
    )
    monkeypatch.setenv("CODEX_HOME", str(codex_home))

    link_hooks("codex", hooks_path)
    assert trust_codex_hooks() in {"updated", "unchanged"}
    assert inspect_codex_hook_trust().ok
    assert "unrelated-hook" in hooks_path.read_text()


def test_codex_link_preserves_unrelated_and_replaces_legacy(tmp_path: Path) -> None:
    path = tmp_path / "hooks.json"
    path.write_text(
        json.dumps(
            {
                "description": "keep",
                "hooks": {
                    "UserPromptSubmit": [
                        {"hooks": [{"type": "command", "command": "clipboard-hook"}]},
                        {
                            "hooks": [
                                {
                                    "type": "command",
                                    "command": "~/.codex/hooks/AgentSessionStatus/agent_session_status.py hook",
                                }
                            ]
                        },
                    ]
                },
            }
        )
    )
    result = link_hooks("codex", path)
    assert result.changed
    assert result.removed_legacy == 1
    data = json.loads(path.read_text())
    assert data["description"] == "keep"
    assert "clipboard-hook" in path.read_text()
    assert "AgentSessionStatus" not in path.read_text()
    assert inspect_hooks("codex", path).ok

    second = link_hooks("codex", path)
    assert not second.changed
    assert second.removed_legacy == 0


def test_claude_link_removes_plan_claim_only(tmp_path: Path) -> None:
    path = tmp_path / "settings.json"
    path.write_text(
        json.dumps(
            {
                "hooks": {
                    "PostToolUse": [
                        {"hooks": [{"type": "command", "command": "add_plan_frontmatter.py"}]},
                        {
                            "matcher": "ExitPlanMode",
                            "hooks": [
                                {
                                    "type": "command",
                                    "command": "~/.claude/hooks/PostToolUse/plan_claim.py",
                                }
                            ],
                        },
                    ]
                }
            }
        )
    )
    result = link_hooks("claude", path)
    assert result.removed_legacy == 1
    assert "add_plan_frontmatter.py" in path.read_text()
    assert "plan_claim.py" not in path.read_text()
    assert inspect_hooks("claude", path).ok
    assert not link_hooks("claude", path).changed


def test_claude_link_wires_nudge_and_async_rewake_contract(tmp_path: Path) -> None:
    path = tmp_path / "settings.json"

    link_hooks("claude", path)

    hooks = json.loads(path.read_text())["hooks"]
    assert hooks["PostToolBatch"] == [
        {"hooks": [{"type": "command", "command": "ai-coord hook claude", "timeout": 5}]}
    ]
    waker = next(
        group
        for group in hooks["PostToolUseFailure"]
        if group["hooks"][0]["command"] == "ai-coord waker claude"
    )
    assert waker == {
        "matcher": "Bash",
        "hooks": [
            {
                "type": "command",
                "command": "ai-coord waker claude",
                "timeout": 3600,
                "if": "Bash(ai-coord start *)",
                "async": True,
                "asyncRewake": True,
            }
        ],
    }
    waker["hooks"][0]["asyncRewake"] = False
    path.write_text(json.dumps({"hooks": hooks}))
    assert not inspect_hooks("claude", path).ok


def test_claude_link_supports_modular_jsonc_source(tmp_path: Path) -> None:
    path = tmp_path / "hooks.jsonc"
    path.write_text(
        """{
  // Keep this source-file guidance.
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "description": "keep https://example.com/path//literal",
  "hooks": {
    "UserPromptSubmit": [
      {"hooks": [{"type": "command", "command": "clipboard-hook"}]},
    ],
  },
}
"""
    )

    result = link_hooks("claude", path)

    assert result.changed
    assert inspect_hooks("claude", path).ok
    text = path.read_text()
    assert "// Keep this source-file guidance." in text
    assert "https://example.com/path//literal" in text
    assert "clipboard-hook" in text
    assert not link_hooks("claude", path).changed


def test_claude_link_preserves_jsonc_comments_and_unrelated_source(tmp_path: Path) -> None:
    path = tmp_path / "hooks.jsonc"
    original = """{
  // Source-file guidance.
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "description": "keep https://example.test/a//b/*c*/", // inline guidance
  /* The hook configuration starts below. */
  "hooks": {
    // Keep this user-owned group.
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/PostToolUse/plan_claim.py",
          },
        ],
      },
      // This comment belongs to the neighboring group.
      {
        "hooks": [{"type": "command", "command": "clipboard-hook"}],
      },
    ],
    /* Keep this event boundary. */
    "Stop": [
      {"hooks": [{"type": "command", "command": "notify-hook"}]},
    ],
  },
  // Keep this unrelated suffix exactly.
  "other": {"nested": true}, // and its inline comment
}
"""
    path.write_text(original)

    result = link_hooks("claude", path)

    assert result.removed_legacy == 1
    assert inspect_hooks("claude", path).ok
    rendered = path.read_text()
    for unchanged in (
        '  // Source-file guidance.\n  "$schema": "https://json.schemastore.org/claude-code-settings.json",\n',
        '  "description": "keep https://example.test/a//b/*c*/", // inline guidance\n',
        "  /* The hook configuration starts below. */\n",
        "    // Keep this user-owned group.\n",
        "      // This comment belongs to the neighboring group.\n",
        "    /* Keep this event boundary. */\n",
        '  // Keep this unrelated suffix exactly.\n  "other": {"nested": true}, // and its inline comment\n}\n',
    ):
        assert unchanged in rendered
    assert "plan_claim.py" not in rendered
    assert "clipboard-hook" in rendered
    assert "notify-hook" in rendered


def test_link_does_not_write_an_idempotent_jsonc_document(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "hooks.jsonc"
    path.write_text("{}\n")
    assert link_hooks("claude", path).changed
    monkeypatch.setattr(
        integrations_module,
        "_write_config",
        lambda *_args: pytest.fail("idempotent link must not write"),
    )

    assert not link_hooks("claude", path).changed


@settings(
    max_examples=25,
    deadline=None,
    suppress_health_check=[HealthCheck.function_scoped_fixture],
)
@given(value=JSON_SCALARS, items=st.lists(JSON_SCALARS, min_size=1, max_size=5))
def test_jsonc_parser_preserves_literals_and_accepts_comments_and_trailing_commas(
    tmp_path: Path, value: object, items: list[object]
) -> None:
    rendered_items = ",\n".join(f"    {json.dumps(item)}" for item in items)
    text = f"""{{
  // line comment
  "value": {json.dumps(value)},
  "literal": "https://example.test/a//b/*c*/",
  "items": [
{rendered_items},
  ],
  /* preserve
     newlines */
}}
"""
    path = tmp_path / "generated.jsonc"
    path.write_text(text)

    document = JsoncDocument.parse(text)
    assert document.value == {
        "value": value,
        "literal": "https://example.test/a//b/*c*/",
        "items": items,
    }
    assert document.text == text


@settings(max_examples=100)
@example(value={"accepted-before-fix": True}, suffix="")
@given(
    value=JSON_SCALARS,
    suffix=st.text().filter(lambda value: "*/" not in value),
)
def test_jsonc_parser_rejects_unterminated_trailing_block_comments(
    value: object, suffix: str
) -> None:
    text = f"{json.dumps(value)}\n/*{suffix}"

    with pytest.raises(json.JSONDecodeError):
        JsoncDocument.parse(text)


@settings(
    max_examples=25,
    deadline=None,
    suppress_health_check=[HealthCheck.function_scoped_fixture],
)
@example(case=(["clipboard-hook", "notify-hook"], 1))
@given(case=_handler_order_case())
def test_link_preserves_generated_existing_handler_order(
    tmp_path: Path, case: tuple[list[str], int]
) -> None:
    commands, position = case
    expected = list(commands)
    expected.insert(position, "ai-coord hook claude")
    path = tmp_path / "settings.json"
    path.write_text(
        json.dumps(
            {
                "hooks": {
                    "UserPromptSubmit": [
                        {
                            "hooks": [
                                {
                                    "type": "command",
                                    "command": command,
                                    **({"timeout": 5} if command == "ai-coord hook claude" else {}),
                                }
                            ]
                        }
                        for command in expected
                    ]
                }
            }
        )
    )

    link_hooks("claude", path)

    groups = json.loads(path.read_text())["hooks"]["UserPromptSubmit"]
    assert [group["hooks"][0]["command"] for group in groups] == expected


def test_link_dry_run_and_force(tmp_path: Path) -> None:
    path = tmp_path / "settings.json"
    path.write_text('{"hooks": "bad"}\n')
    with pytest.raises(ValueError, match="--force"):
        link_hooks("claude", path)
    preview = link_hooks("claude", path, force=True, dry_run=True)
    assert preview.changed
    assert path.read_text() == '{"hooks": "bad"}\n'


def test_hook_check_requires_full_handler_contract(tmp_path: Path) -> None:
    path = tmp_path / "hooks.json"
    link_hooks("codex", path)
    data = json.loads(path.read_text())
    data["hooks"]["Stop"][0]["hooks"][0]["timeout"] = 1
    data["hooks"]["UserPromptSubmit"][0]["hooks"][0].pop("additionalContextLimit")
    path.write_text(json.dumps(data))

    result = inspect_hooks("codex", path)

    assert not result.ok
    assert result.missing == ("UserPromptSubmit", "Stop")


def test_default_hook_paths_honor_client_config_roots(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    codex_home = tmp_path / "codex"
    claude_home = tmp_path / "claude"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(claude_home))

    assert default_hook_path("codex") == codex_home / "hooks.json"
    assert default_hook_path("claude") == claude_home / "settings.json"
    assert default_link_path("codex") == codex_home / "hooks.json"
    assert default_link_path("claude") == claude_home / "settings.json"

    modular_source = claude_home / "settings" / "hooks.jsonc"
    modular_source.parent.mkdir(parents=True)
    modular_source.write_text("{}\n")
    assert default_link_path("claude") == modular_source


def test_link_write_is_atomic_and_preserves_mode(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "hooks.json"
    original = '{"description": "keep"}\n'
    path.write_text(original)
    path.chmod(0o640)

    def fail_replace(_source: os.PathLike[str], _target: os.PathLike[str]) -> None:
        raise OSError("replace failed")

    monkeypatch.setattr(os, "replace", fail_replace)
    with pytest.raises(OSError, match="replace failed"):
        link_hooks("codex", path)

    assert path.read_text() == original
    assert stat.S_IMODE(path.stat().st_mode) == 0o640
    assert list(tmp_path.glob(".hooks.json.*.tmp")) == []


def test_link_updates_symlink_target_without_replacing_link(tmp_path: Path) -> None:
    target = tmp_path / "tracked-hooks.json"
    target.write_text("{}\n")
    path = tmp_path / "hooks.json"
    path.symlink_to(target)

    link_hooks("codex", path)

    assert path.is_symlink()
    assert inspect_hooks("codex", target).ok
