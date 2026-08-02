from __future__ import annotations

import json
from pathlib import Path

import pytest

from ai_coord.integrations import default_hook_path, inspect_hooks, link_hooks


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
