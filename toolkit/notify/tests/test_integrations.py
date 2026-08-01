"""Tests for aggregate Claude and layered Codex integration checks."""

import json

from ai_notify.claude_hooks import HOOK_SPECS
from ai_notify.integrations import inspect_claude_hooks


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
