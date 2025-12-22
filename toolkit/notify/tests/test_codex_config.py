"""
Tests for Codex config notify updater.
"""

from ai_notify.codex_config import _update_notify_in_toml


def test_update_notify_replaces_root_block():
    text = """notify = [
  "bash",
  "-lc",
  "echo test",
]

[features]
  shell_snapshot = true
"""
    updated, changed = _update_notify_in_toml(text, ["ai-notify", "codex"], profile=None)

    assert changed is True
    assert 'notify = ["ai-notify", "codex"]' in updated
    assert "[features]" in updated


def test_update_notify_inserts_root_before_tables():
    text = """model = "gpt-5.2-codex"

[features]
  shell_snapshot = true
"""
    updated, changed = _update_notify_in_toml(text, ["ai-notify", "codex"], profile=None)

    assert changed is True
    assert 'notify = ["ai-notify", "codex"]' in updated
    assert updated.index('notify = ["ai-notify", "codex"]') < updated.index("[features]")


def test_update_notify_replaces_profile_block():
    text = """notify = ["bash", "-lc", "echo root"]

[profiles.quiet]
  notify = []
"""
    updated, changed = _update_notify_in_toml(text, ["ai-notify", "codex"], profile="quiet")

    assert changed is True
    assert 'notify = ["ai-notify", "codex"]' in updated
    assert "profiles.quiet" in updated
