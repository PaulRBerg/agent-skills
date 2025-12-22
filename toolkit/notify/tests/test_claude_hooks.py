"""
Tests for Claude Code hooks installer.
"""

import json

from ai_notify.claude_hooks import REQUIRED_HOOKS, ensure_claude_hooks


def test_install_creates_hooks_file(tmp_path):
    path = tmp_path / "hooks.json"

    result = ensure_claude_hooks(path)

    assert result.changed is True
    data = json.loads(path.read_text())
    for event, command in REQUIRED_HOOKS.items():
        assert data["hooks"][event]["command"] == command


def test_install_skips_existing_without_force(tmp_path):
    path = tmp_path / "hooks.json"
    path.write_text(
        json.dumps(
            {
                "hooks": {
                    "Stop": {"command": "echo stop"},
                }
            }
        )
    )

    result = ensure_claude_hooks(path, force=False)

    assert result.changed is True
    assert "Stop" in result.skipped


def test_install_overwrites_with_force(tmp_path):
    path = tmp_path / "hooks.json"
    path.write_text(
        json.dumps(
            {
                "hooks": {
                    "Stop": {"command": "echo stop"},
                }
            }
        )
    )

    result = ensure_claude_hooks(path, force=True)

    assert result.changed is True
    data = json.loads(path.read_text())
    assert data["hooks"]["Stop"]["command"] == REQUIRED_HOOKS["Stop"]
