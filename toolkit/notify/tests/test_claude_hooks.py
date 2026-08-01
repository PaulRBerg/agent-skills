"""
Tests for the Claude Code settings.json hooks installer.
"""

import json

from ai_notify.claude_hooks import HOOK_SPECS, ensure_claude_hooks, iter_hook_commands


def _commands_for(data, event):
    return list(iter_hook_commands(data["hooks"].get(event)))


def test_install_creates_settings_file(tmp_path):
    path = tmp_path / "settings.json"

    result = ensure_claude_hooks(path)

    assert result.changed is True
    data = json.loads(path.read_text())
    for spec in HOOK_SPECS:
        # Each event maps to a list of matcher groups (Claude Code's nested schema).
        assert isinstance(data["hooks"][spec.event], list)
        assert spec.command in _commands_for(data, spec.event)


def test_command_hook_entries_have_type_and_command(tmp_path):
    path = tmp_path / "settings.json"

    ensure_claude_hooks(path)

    data = json.loads(path.read_text())
    handler = data["hooks"]["Stop"][0]["hooks"][0]
    assert handler == {"type": "command", "command": "ai-notify event stop"}


def test_pretooluse_uses_askuserquestion_matcher(tmp_path):
    path = tmp_path / "settings.json"

    ensure_claude_hooks(path)

    data = json.loads(path.read_text())
    group = data["hooks"]["PreToolUse"][0]
    assert group["matcher"] == "AskUserQuestion"
    assert group["hooks"][0]["command"] == "ai-notify event ask-user-question"


def test_matcherless_events_omit_matcher(tmp_path):
    path = tmp_path / "settings.json"

    ensure_claude_hooks(path)

    data = json.loads(path.read_text())
    assert "matcher" not in data["hooks"]["Stop"][0]
    assert "matcher" not in data["hooks"]["StopFailure"][0]
    assert "matcher" not in data["hooks"]["UserPromptSubmit"][0]


def test_install_is_idempotent(tmp_path):
    path = tmp_path / "settings.json"

    first = ensure_claude_hooks(path)
    second = ensure_claude_hooks(path)

    assert first.changed is True
    assert second.changed is False
    assert second.added == []


def test_install_preserves_existing_user_hook(tmp_path):
    path = tmp_path / "settings.json"
    path.write_text(
        json.dumps(
            {
                "hooks": {
                    "Stop": [
                        {"hooks": [{"type": "command", "command": "echo stop"}]},
                    ],
                }
            }
        )
    )

    result = ensure_claude_hooks(path)

    assert result.changed is True
    commands = _commands_for(json.loads(path.read_text()), "Stop")
    assert "echo stop" in commands  # user's existing hook is preserved
    assert "ai-notify event stop" in commands  # ours is appended alongside it


def test_install_migrates_legacy_flat_entry(tmp_path):
    path = tmp_path / "settings.json"
    path.write_text(json.dumps({"hooks": {"Stop": {"command": "ai-notify event stop"}}}))

    result = ensure_claude_hooks(path)

    assert "Stop" in result.updated
    data = json.loads(path.read_text())
    # The old flat ai-notify entry is migrated to the nested list schema.
    assert isinstance(data["hooks"]["Stop"], list)
    assert "ai-notify event stop" in _commands_for(data, "Stop")


def test_install_skips_foreign_flat_without_force(tmp_path):
    path = tmp_path / "settings.json"
    path.write_text(json.dumps({"hooks": {"Stop": {"command": "echo stop"}}}))

    result = ensure_claude_hooks(path, force=False)

    assert "Stop" in result.skipped
    # A foreign flat entry is left untouched without --force.
    assert json.loads(path.read_text())["hooks"]["Stop"] == {"command": "echo stop"}


def test_force_replaces_foreign_flat_entry(tmp_path):
    path = tmp_path / "settings.json"
    path.write_text(json.dumps({"hooks": {"Stop": {"command": "echo stop"}}}))

    result = ensure_claude_hooks(path, force=True)

    assert "Stop" in result.updated
    data = json.loads(path.read_text())
    assert isinstance(data["hooks"]["Stop"], list)
    assert "ai-notify event stop" in _commands_for(data, "Stop")


def test_install_preserves_other_top_level_keys(tmp_path):
    path = tmp_path / "settings.json"
    path.write_text(json.dumps({"model": "opus", "permissions": {"allow": ["Bash"]}}))

    ensure_claude_hooks(path)

    data = json.loads(path.read_text())
    assert data["model"] == "opus"
    assert data["permissions"] == {"allow": ["Bash"]}
    assert "hooks" in data
