"""Tests for Codex config notify updates."""

import tomllib

import pytest

from ai_notify.codex_config import resolve_codex_config_path, set_codex_notify

COMMAND = ["ai-notify", "codex"]


def test_insert_root_notify_preserves_surrounding_format_and_comments(tmp_path):
    path = tmp_path / "config.toml"
    path.write_text(
        """# model selection
model = "gpt-5.6"

[features]
# keep this comment
shell_snapshot = true
"""
    )

    result = set_codex_notify(path, COMMAND)

    assert result.changed is True
    assert result.previous_notify is None
    updated = path.read_text()
    assert "# model selection" in updated
    assert "# keep this comment" in updated
    assert updated.index('notify = ["ai-notify", "codex"]') < updated.index("[features]")
    assert tomllib.loads(updated)["notify"] == COMMAND


def test_profile_resolves_to_sibling_file_without_changing_base(tmp_path):
    base_path = tmp_path / "config.toml"
    base_text = 'model = "gpt-5.6"\n'
    base_path.write_text(base_text)

    result = set_codex_notify(base_path, COMMAND, profile="deep-review")

    profile_path = tmp_path / "deep-review.config.toml"
    assert result.path == profile_path
    assert tomllib.loads(profile_path.read_text())["notify"] == COMMAND
    assert base_path.read_text() == base_text


@pytest.mark.parametrize("profile", ["deep.review", "../quiet", "with space", "", "x/y"])
def test_profile_name_validation(profile, tmp_path):
    base_path = tmp_path / "config.toml"

    with pytest.raises(ValueError, match="letters, numbers, hyphens, and underscores"):
        resolve_codex_config_path(base_path, profile)


def test_matching_notify_is_idempotent(tmp_path):
    path = tmp_path / "config.toml"
    original = '# keep\nnotify = ["ai-notify", "codex"]\n'
    path.write_text(original)

    result = set_codex_notify(path, COMMAND)

    assert result.changed is False
    assert result.conflict is False
    assert result.previous_notify == COMMAND
    assert path.read_text() == original


def test_different_notify_is_refused_and_file_is_unchanged(tmp_path):
    path = tmp_path / "config.toml"
    original = 'notify = ["computer-use-notify"]\n'
    path.write_text(original)

    result = set_codex_notify(path, COMMAND)

    assert result.changed is False
    assert result.conflict is True
    assert result.previous_notify == ["computer-use-notify"]
    assert path.read_text() == original


def test_force_replaces_conflict_and_reports_previous_value(tmp_path):
    path = tmp_path / "config.toml"
    path.write_text('# keep\n"notify" = ["computer-use-notify"] # old command\n')

    result = set_codex_notify(path, COMMAND, force=True)

    assert result.changed is True
    assert result.conflict is True
    assert result.previous_notify == ["computer-use-notify"]
    updated = path.read_text()
    assert "# keep" in updated
    assert "# old command" in updated
    assert tomllib.loads(updated)["notify"] == COMMAND


def test_only_exact_root_notify_key_is_updated(tmp_path):
    path = tmp_path / "config.toml"
    path.write_text(
        """notify_command = ["leave", "alone"]

[features]
notify = ["nested", "value"]
"""
    )

    result = set_codex_notify(path, COMMAND)

    data = tomllib.loads(path.read_text())
    assert result.changed is True
    assert data["notify"] == COMMAND
    assert data["notify_command"] == ["leave", "alone"]
    assert data["features"]["notify"] == ["nested", "value"]


def test_invalid_toml_is_refused_without_modification(tmp_path):
    path = tmp_path / "config.toml"
    original = 'notify = ["unterminated"\n'
    path.write_text(original)

    with pytest.raises(ValueError, match="Failed to parse"):
        set_codex_notify(path, COMMAND, force=True)

    assert path.read_text() == original
