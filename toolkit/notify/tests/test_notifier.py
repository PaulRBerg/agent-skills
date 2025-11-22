"""
Tests for notification system.
"""

from pathlib import Path

import pytest

from ai_notify.notifier import MacNotifier


class TestMacNotifier:
    """Test MacNotifier notification logic."""

    @pytest.fixture
    def notifier(self):
        return MacNotifier()

    def test_check_available_true(self, notifier, mocker):
        # Mock platform and terminal-notifier availability
        mocker.patch("platform.system", return_value="Darwin")
        mocker.patch("shutil.which", return_value="/opt/homebrew/bin/terminal-notifier")

        assert notifier.check_available() is True

    def test_check_available_not_macos(self, notifier, mocker):
        # Mock non-macOS platform
        mocker.patch("platform.system", return_value="Linux")

        assert notifier.check_available() is False

    def test_check_available_terminal_notifier_missing(self, notifier, mocker):
        # Mock macOS but terminal-notifier not installed
        mocker.patch("platform.system", return_value="Darwin")
        mocker.patch("shutil.which", return_value=None)

        assert notifier.check_available() is False

    def test_send_notification_success(self, notifier, mocker):
        notifier._available = True

        # Mock subprocess.run in the notifier module
        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0

        # Mock icon path
        mocker.patch.object(notifier, "_get_icon_path", return_value=Path("/fake/icon.png"))

        result = notifier.send_notification("Test", "Subtitle")
        assert result is True

        # Verify subprocess was called with correct arguments
        mock_run.assert_called_once()
        cmd = mock_run.call_args[0][0]
        assert cmd[0] == "terminal-notifier"
        assert "-title" in cmd
        assert "Test" in cmd
        assert "-message" in cmd
        assert "Subtitle" in cmd
        assert "-activate" in cmd
        assert "-sound" in cmd
        assert "-contentImage" in cmd

    def test_send_notification_with_message(self, notifier, mocker):
        notifier._available = True

        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0

        # Mock icon path
        mocker.patch.object(notifier, "_get_icon_path", return_value=Path("/fake/icon.png"))

        result = notifier.send_notification("Test", "Subtitle", "Body message")
        assert result is True

        # Verify message includes both subtitle and body
        cmd = mock_run.call_args[0][0]
        message_idx = cmd.index("-message") + 1
        message = cmd[message_idx]
        assert "Subtitle" in message
        assert "Body message" in message

    def test_send_notification_without_icon(self, notifier, mocker):
        notifier._available = True

        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0

        # Mock missing icon
        mocker.patch.object(notifier, "_get_icon_path", return_value=None)

        result = notifier.send_notification("Test", "Subtitle")
        assert result is True

        # Verify -contentImage is not in command
        cmd = mock_run.call_args[0][0]
        assert "-contentImage" not in cmd

    def test_send_notification_with_sound(self, notifier, mocker):
        notifier._available = True

        # Mock runtime config with custom sound
        from ai_notify.config_loader import AINotifyConfig, NotificationConfig

        mock_config = AINotifyConfig()
        mock_config.notification = NotificationConfig(sound="Glass")
        mocker.patch("ai_notify.notifier.get_runtime_config", return_value=mock_config)

        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0

        # Mock icon path
        mocker.patch.object(notifier, "_get_icon_path", return_value=None)

        result = notifier.send_notification("Test", "Subtitle")
        assert result is True

        # Verify sound parameter
        cmd = mock_run.call_args[0][0]
        sound_idx = cmd.index("-sound") + 1
        assert cmd[sound_idx] == "Glass"

    def test_send_notification_with_activation(self, notifier, mocker):
        notifier._available = True

        # Mock runtime config with custom app bundle
        from ai_notify.config_loader import AINotifyConfig, NotificationConfig

        mock_config = AINotifyConfig()
        mock_config.notification = NotificationConfig(app_bundle="com.example.MyApp")
        mocker.patch("ai_notify.notifier.get_runtime_config", return_value=mock_config)

        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0

        # Mock icon path
        mocker.patch.object(notifier, "_get_icon_path", return_value=None)

        result = notifier.send_notification("Test", "Subtitle")
        assert result is True

        # Verify activation parameter
        cmd = mock_run.call_args[0][0]
        activate_idx = cmd.index("-activate") + 1
        assert cmd[activate_idx] == "com.example.MyApp"

    def test_send_notification_terminal_notifier_fails(self, notifier, mocker):
        notifier._available = True

        # Mock terminal-notifier failure
        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 1
        mock_run.return_value.stderr = "Error: something went wrong"

        # Mock icon path
        mocker.patch.object(notifier, "_get_icon_path", return_value=None)

        result = notifier.send_notification("Test", "Subtitle")
        assert result is False

    def test_send_notification_unavailable(self, notifier, mocker):
        notifier._available = False
        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")

        result = notifier.send_notification("Test", "Subtitle")
        assert result is False
        # subprocess.run should not be called when unavailable
        mock_run.assert_not_called()

    def test_get_project_name(self, notifier):
        assert notifier.get_project_name("/Users/test/my-project") == "my-project"
        assert notifier.get_project_name("/Users/test/project/") == "project"
        assert notifier.get_project_name("/Users/test/.claude") == ".claude"

    def test_get_icon_path_exists(self, notifier, mocker):
        # Mock icon file existence
        mocker.patch("pathlib.Path.exists", return_value=True)

        icon_path = notifier._get_icon_path()
        assert icon_path is not None
        assert "claude-icon.png" in str(icon_path)

    def test_get_icon_path_missing(self, notifier, mocker):
        # Mock icon file missing
        mocker.patch("pathlib.Path.exists", return_value=False)

        icon_path = notifier._get_icon_path()
        assert icon_path is None

    def test_notify_permission_request_with_job_number(self, notifier, mocker):
        """Test permission notification with job number."""
        notifier._available = True

        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0

        # Mock icon path
        mocker.patch.object(notifier, "_get_icon_path", return_value=None)

        result = notifier.notify_permission_request(
            "test-project", "Command: npm install", job_number=3
        )
        assert result is True

        # Verify command includes job number in subtitle
        cmd = mock_run.call_args[0][0]
        message_idx = cmd.index("-message") + 1
        message = cmd[message_idx]
        assert "Prompt #3 needs approval" in message
        assert "Command: npm install" in message

    def test_notify_permission_request_without_job_number(self, notifier, mocker):
        """Test permission notification without job number."""
        notifier._available = True

        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0

        # Mock icon path
        mocker.patch.object(notifier, "_get_icon_path", return_value=None)

        result = notifier.notify_permission_request("test-project", "Command: npm install")
        assert result is True

        # Verify command uses default subtitle
        cmd = mock_run.call_args[0][0]
        message_idx = cmd.index("-message") + 1
        message = cmd[message_idx]
        assert "Approval needed" in message
        assert "Command: npm install" in message


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
