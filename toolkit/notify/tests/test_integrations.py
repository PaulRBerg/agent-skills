"""Tests for aggregate Claude and layered Codex integration checks."""

import json

import pytest

from ai_notify.claude_hooks import HOOK_SPECS
from ai_notify.integrations import inspect_claude_hooks, inspect_codex_notify


def _hooks_for(specs):
    return {
        spec.event: [
            {
                **({"matcher": spec.matcher} if spec.matcher else {}),
                "hooks": [{"type": "command", "command": spec.command}],
            }
        ]
        for spec in specs
    }


def test_claude_hooks_merge_active_settings_layers(tmp_path):
    config_root = tmp_path / "home" / ".claude"
    project_root = tmp_path / "project"
    user_path = config_root / "settings.json"
    project_path = project_root / ".claude" / "settings.json"
    local_path = project_root / ".claude" / "settings.local.json"
    for path in (user_path, project_path, local_path):
        path.parent.mkdir(parents=True, exist_ok=True)

    user_path.write_text(json.dumps({"hooks": _hooks_for(HOOK_SPECS[:2])}))
    project_path.write_text(json.dumps({"hooks": _hooks_for(HOOK_SPECS[2:4])}))
    local_path.write_text(json.dumps({"hooks": _hooks_for(HOOK_SPECS[4:])}))

    report = inspect_claude_hooks(config_root, project_root)

    assert report.status == "ok"
    assert report.missing_events == []
    assert report.paths == [user_path, project_path, local_path]


def test_claude_hooks_ignore_unsupported_global_locations(tmp_path):
    config_root = tmp_path / ".claude"
    project_root = tmp_path / "project"
    standalone_path = config_root / "hooks" / "hooks.json"
    global_local_path = config_root / "settings.local.json"
    for path in (standalone_path, global_local_path):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"hooks": _hooks_for(HOOK_SPECS)}))

    report = inspect_claude_hooks(config_root, project_root)

    assert report.status == "missing"
    assert report.paths == []
    assert report.missing_events == [spec.event for spec in HOOK_SPECS]
    assert report.ignored_paths == [standalone_path, global_local_path]


def test_codex_profile_inherits_base_notify(tmp_path):
    config_root = tmp_path / ".codex"
    config_root.mkdir()
    base_path = config_root / "config.toml"
    profile_path = config_root / "review.config.toml"
    base_path.write_text('notify = ["ai-notify", "codex"]\n')
    profile_path.write_text('model = "gpt-5.6"\n')

    report = inspect_codex_notify(config_root, profile="review")

    assert report.status == "ok"
    assert report.notify == ["ai-notify", "codex"]
    assert report.path == base_path
    assert report.paths == [base_path, profile_path]


def test_codex_profile_notify_overrides_base(tmp_path):
    config_root = tmp_path / ".codex"
    config_root.mkdir()
    base_path = config_root / "config.toml"
    profile_path = config_root / "review.config.toml"
    base_path.write_text('notify = ["ai-notify", "codex"]\n')
    profile_path.write_text('notify = ["different"]\n')

    report = inspect_codex_notify(config_root, profile="review")

    assert report.status == "partial"
    assert report.notify == ["different"]
    assert report.path == profile_path


def test_codex_missing_profile_is_an_error(tmp_path):
    config_root = tmp_path / ".codex"
    config_root.mkdir()
    (config_root / "config.toml").write_text('notify = ["ai-notify", "codex"]\n')

    report = inspect_codex_notify(config_root, profile="missing")

    assert report.status == "error"
    assert report.path == config_root / "missing.config.toml"
    assert "Profile config not found" in (report.error or "")


def test_codex_check_validates_profile_name(tmp_path):
    with pytest.raises(ValueError, match="letters, numbers"):
        inspect_codex_notify(tmp_path, profile="bad.profile")
