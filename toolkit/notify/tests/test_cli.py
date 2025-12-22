"""
Tests for CLI commands and auto-cleanup.
"""

import tempfile
from datetime import datetime, timedelta
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest
from click.testing import CliRunner

from ai_notify import cli
from ai_notify import config_loader
from ai_notify.claude_hooks import ClaudeHooksUpdate
from ai_notify.config import Config
from ai_notify.codex_config import CodexNotifyUpdate
from ai_notify.database import SessionTracker


class TestCLICommands:
    """Test CLI commands."""

    @pytest.fixture
    def runner(self):
        """Create CLI test runner."""
        return CliRunner()

    @pytest.fixture
    def temp_config_file(self):
        """Create temporary config file."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.yaml"
            loader = config_loader.ConfigLoader(config_path)
            loader.save(config_loader.AINotifyConfig())
            yield config_path

    def test_config_show(self, runner, temp_config_file):
        """Test config show command."""
        result = runner.invoke(cli.cli, ["config", "show", "--path", str(temp_config_file)])
        assert result.exit_code == 0
        assert "Current Configuration" in result.output
        assert "Notification Threshold" in result.output
        assert "10s" in result.output

    def test_config_reset(self, runner, temp_config_file):
        """Test config reset command."""
        # Modify config first
        loader = config_loader.ConfigLoader(temp_config_file)
        cfg = loader.load()
        cfg.notification.threshold_seconds = 20
        loader.save(cfg)

        # Reset config
        result = runner.invoke(
            cli.cli, ["config", "reset", "--path", str(temp_config_file)], input="y\n"
        )
        assert result.exit_code == 0
        assert "reset to defaults" in result.output

        # Verify reset - create new loader to avoid cache
        new_loader = config_loader.ConfigLoader(temp_config_file)
        cfg = new_loader.load()
        assert cfg.notification.threshold_seconds == 10  # Back to default

    def test_test_command(self, runner):
        """Test the test notification command."""
        with patch("ai_notify.cli.MacNotifier") as mock_notifier:
            mock_instance = MagicMock()
            mock_notifier.return_value = mock_instance

            result = runner.invoke(cli.cli, ["test"])

            # Should call notify_job_done
            assert mock_instance.notify_job_done.called
            assert result.exit_code == 0
            assert "Test notification sent" in result.output

    def test_cleanup_dry_run(self, runner):
        """Test cleanup with dry-run flag."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create temp database with test data
            cfg = Config()
            cfg.db_path = Path(tmpdir) / "test.db"
            cfg.config_dir = Path(tmpdir)

            tracker = SessionTracker(cfg)
            tracker.track_prompt("test-1", "prompt", "/test")

            with patch("ai_notify.cli.SessionTracker") as mock_tracker:
                mock_tracker.return_value = tracker

                result = runner.invoke(cli.cli, ["cleanup", "--dry-run"])

                assert result.exit_code == 0
                assert "DRY RUN MODE" in result.output
                assert "No data will be deleted" in result.output

    def test_link_codex_updates_notify(self, runner, tmp_path):
        """Test link codex updates notify setting."""
        config_path = tmp_path / "config.toml"

        with patch("ai_notify.cli.set_codex_notify") as mock_set:
            mock_set.return_value = CodexNotifyUpdate(
                path=config_path,
                changed=True,
                profile=None,
            )

            result = runner.invoke(cli.cli, ["link", "codex", "--path", str(config_path)])

        assert result.exit_code == 0
        mock_set.assert_called_once_with(config_path, ["ai-notify", "codex"], profile=None)
        assert "Updated root config notify" in result.output

    def test_link_codex_no_change(self, runner, tmp_path):
        """Test link codex reports already set."""
        config_path = tmp_path / "config.toml"

        with patch("ai_notify.cli.set_codex_notify") as mock_set:
            mock_set.return_value = CodexNotifyUpdate(
                path=config_path,
                changed=False,
                profile="quiet",
            )

            result = runner.invoke(
                cli.cli, ["link", "codex", "--path", str(config_path), "--profile", "quiet"]
            )

        assert result.exit_code == 0
        mock_set.assert_called_once_with(config_path, ["ai-notify", "codex"], profile="quiet")
        assert "profile 'quiet' notify already set" in result.output

    def test_link_claude_installs_hooks(self, runner, tmp_path):
        """Test link claude installs hooks and reports skips."""
        hooks_path = tmp_path / "hooks.json"

        with patch("ai_notify.cli.ensure_claude_hooks") as mock_ensure:
            mock_ensure.return_value = ClaudeHooksUpdate(
                path=hooks_path,
                changed=True,
                added=["Stop"],
                updated=[],
                skipped={"Notification": "echo notify"},
                errors=[],
            )

            result = runner.invoke(cli.cli, ["link", "claude", "--path", str(hooks_path)])

        assert result.exit_code == 0
        mock_ensure.assert_called_once_with(hooks_path, force=False, dry_run=False)
        assert "Updated hooks" in result.output
        assert "Skipped existing hooks" in result.output
        assert "Notification" in result.output

    def test_codex_notify_cli_invokes_handler(self, runner):
        """Test codex notify handler invocation."""
        payload = '{"type": "agent-turn-complete", "input-messages": ["hi"]}'

        with patch("ai_notify.cli.handle_codex_notify") as mock_handler:
            result = runner.invoke(cli.cli, ["codex", payload])

        assert result.exit_code == 0
        assert mock_handler.called


class TestAutoCleanup:
    """Test auto-cleanup functionality."""

    @pytest.fixture
    def temp_config(self):
        """Create temporary config with test database."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cfg = Config()
            cfg.db_path = Path(tmpdir) / "test.db"
            cfg.config_dir = Path(tmpdir)
            yield cfg

    def test_should_run_auto_cleanup_first_time(self):
        """Test auto-cleanup runs on first execution."""
        with tempfile.TemporaryDirectory() as tmpdir:
            last_cleanup_file = Path(tmpdir) / ".last_cleanup"

            # Mock the LAST_CLEANUP_FILE
            with patch("ai_notify.helpers.cleanup.LAST_CLEANUP_FILE", last_cleanup_file):
                from ai_notify.helpers.cleanup import should_run_auto_cleanup

                result = should_run_auto_cleanup()
                assert result is True  # Should run since file doesn't exist

    def test_should_run_auto_cleanup_after_24_hours(self):
        """Test auto-cleanup runs after 24 hours."""
        with tempfile.TemporaryDirectory() as tmpdir:
            last_cleanup_file = Path(tmpdir) / ".last_cleanup"
            last_cleanup_file.touch()

            # Set file mtime to 25 hours ago
            old_time = (datetime.now() - timedelta(hours=25)).timestamp()
            import os

            os.utime(last_cleanup_file, (old_time, old_time))

            with patch("ai_notify.helpers.cleanup.LAST_CLEANUP_FILE", last_cleanup_file):
                from ai_notify.helpers.cleanup import should_run_auto_cleanup

                result = should_run_auto_cleanup()
                assert result is True  # Should run since >24 hours

    def test_should_not_run_auto_cleanup_recent(self):
        """Test auto-cleanup doesn't run if recent."""
        with tempfile.TemporaryDirectory() as tmpdir:
            last_cleanup_file = Path(tmpdir) / ".last_cleanup"
            last_cleanup_file.touch()  # Just created

            with patch("ai_notify.helpers.cleanup.LAST_CLEANUP_FILE", last_cleanup_file):
                from ai_notify.helpers.cleanup import should_run_auto_cleanup

                result = should_run_auto_cleanup()
                assert result is False  # Should not run since <24 hours


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
