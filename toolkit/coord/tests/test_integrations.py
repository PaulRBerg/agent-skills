from __future__ import annotations

import json
import os
import stat
from pathlib import Path

import pytest
from hypothesis import HealthCheck, example, given, settings
from hypothesis import strategies as st

from ai_coord.integrations import (
    _read_config,
    _strip_jsonc,
    default_hook_path,
    default_link_path,
    inspect_hooks,
    link_hooks,
)

JSON_SCALARS = st.none() | st.booleans() | st.integers() | st.text()


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

    assert _read_config(path) == {
        "value": value,
        "literal": "https://example.test/a//b/*c*/",
        "items": items,
    }
    assert _strip_jsonc(text).count("\n") == text.count("\n")


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
        _strip_jsonc(text)


def test_link_preserves_existing_handler_order(tmp_path: Path) -> None:
    path = tmp_path / "settings.json"
    path.write_text(
        json.dumps(
            {
                "hooks": {
                    "UserPromptSubmit": [
                        {"hooks": [{"type": "command", "command": "clipboard-hook"}]},
                        {
                            "hooks": [
                                {
                                    "type": "command",
                                    "command": "ai-coord hook claude",
                                    "timeout": 5,
                                }
                            ]
                        },
                        {"hooks": [{"type": "command", "command": "notify-hook"}]},
                    ]
                }
            }
        )
    )

    link_hooks("claude", path)

    groups = json.loads(path.read_text())["hooks"]["UserPromptSubmit"]
    assert [group["hooks"][0]["command"] for group in groups] == [
        "clipboard-hook",
        "ai-coord hook claude",
        "notify-hook",
    ]


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
