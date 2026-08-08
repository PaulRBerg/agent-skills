"""
Tests for notification system.
"""

from pathlib import Path

import pytest

from ai_notify.notifier import (
    DETAIL_EXCERPT_LENGTH,
    TASK_EXCERPT_LENGTH,
    MacNotifier,
    _build_focus_command,
    _format_context_message,
)


class TestMacNotifier:
    """Test MacNotifier notification logic."""

    @pytest.fixture
    def notifier(self):
        return MacNotifier()

    @pytest.fixture(autouse=True)
    def _clear_iterm_session(self, monkeypatch):
        # Ambient ITERM_SESSION_ID (set when tests run inside iTerm2) would otherwise
        # make the -execute vs -activate branch nondeterministic across dev machines/CI.
        monkeypatch.delenv("ITERM_SESSION_ID", raising=False)

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
        assert "-subtitle" not in cmd
        assert "-ignoreDnD" in cmd
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

        # Verify native subtitle and message fields remain distinct.
        cmd = mock_run.call_args[0][0]
        assert cmd[cmd.index("-subtitle") + 1] == "Subtitle"
        assert cmd[cmd.index("-message") + 1] == "Body message"

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

    def test_send_notification_uses_execute_when_iterm_session_present(
        self, notifier, mocker, monkeypatch
    ):
        notifier._available = True
        monkeypatch.setenv("ITERM_SESSION_ID", "w0t0p0:8FC1AAA5-2AA5-40D4-A410-1234567890AB")
        mocker.patch("ai_notify.notifier.shutil.which", return_value="/opt/homebrew/bin/it2")

        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0
        mocker.patch.object(notifier, "_get_icon_path", return_value=None)

        result = notifier.send_notification("Test", "Subtitle")
        assert result is True

        cmd = mock_run.call_args[0][0]
        assert "-activate" not in cmd
        assert "-execute" in cmd
        execute_arg = cmd[cmd.index("-execute") + 1]
        assert "/opt/homebrew/bin/it2" in execute_arg
        assert "8FC1AAA5-2AA5-40D4-A410-1234567890AB" in execute_arg
        assert "session focus" in execute_arg
        assert "open -b com.googlecode.iterm2" in execute_arg

    def test_send_notification_falls_back_when_it2_missing(self, notifier, mocker, monkeypatch):
        notifier._available = True
        monkeypatch.setenv("ITERM_SESSION_ID", "w0t0p0:8FC1AAA5-2AA5-40D4-A410-1234567890AB")
        mocker.patch("ai_notify.notifier.shutil.which", return_value=None)

        mock_run = mocker.patch("ai_notify.notifier.subprocess.run")
        mock_run.return_value.returncode = 0
        mocker.patch.object(notifier, "_get_icon_path", return_value=None)

        result = notifier.send_notification("Test", "Subtitle")
        assert result is True

        cmd = mock_run.call_args[0][0]
        assert "-execute" not in cmd
        assert "-activate" in cmd
        assert cmd[cmd.index("-activate") + 1] == "com.googlecode.iterm2"

    def test_build_focus_command_without_session_id(self, monkeypatch):
        monkeypatch.delenv("ITERM_SESSION_ID", raising=False)
        assert _build_focus_command("com.googlecode.iterm2") is None

    def test_build_focus_command_without_it2(self, mocker, monkeypatch):
        monkeypatch.setenv("ITERM_SESSION_ID", "w0t0p0:UUID-VALUE")
        mocker.patch("ai_notify.notifier.shutil.which", return_value=None)
        assert _build_focus_command("com.googlecode.iterm2") is None

    def test_build_focus_command_parses_session_uuid(self, mocker, monkeypatch):
        monkeypatch.setenv("ITERM_SESSION_ID", "w2t3p0:8FC1AAA5-2AA5-40D4-A410-1234567890AB")
        mocker.patch("ai_notify.notifier.shutil.which", return_value="/usr/local/bin/it2")

        command = _build_focus_command("com.googlecode.iterm2")

        assert command is not None
        assert "/usr/local/bin/it2" in command
        assert "session focus" in command
        assert "8FC1AAA5-2AA5-40D4-A410-1234567890AB" in command
        assert "w2t3p0" not in command
        assert command.endswith("|| open -b com.googlecode.iterm2")

    def test_build_focus_command_handles_session_id_without_colon(self, mocker, monkeypatch):
        monkeypatch.setenv("ITERM_SESSION_ID", "8FC1AAA5-2AA5-40D4-A410-1234567890AB")
        mocker.patch("ai_notify.notifier.shutil.which", return_value="/usr/local/bin/it2")

        command = _build_focus_command("com.googlecode.iterm2")

        assert command is not None
        assert "8FC1AAA5-2AA5-40D4-A410-1234567890AB" in command

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

    def test_get_codex_icon_path_exists(self):
        icon_path = MacNotifier(icon_name="codex")._get_icon_path()

        assert icon_path is not None
        assert icon_path.name == "codex-icon.png"
        assert icon_path.is_file()

    def test_get_icon_path_missing(self, notifier, mocker):
        # Mock icon file missing
        mocker.patch("pathlib.Path.exists", return_value=False)

        icon_path = notifier._get_icon_path()
        assert icon_path is None

    def test_notify_completion_includes_task_result_and_duration(self, notifier, mocker):
        mock_send = mocker.patch.object(notifier, "send_notification", return_value=True)

        result = notifier.notify_completion(
            "test-project",
            agent="Claude",
            task="Fix the bug",
            result="Fixed the bug and added tests.",
            duration_str="1m23s",
        )

        assert result is True
        mock_send.assert_called_once_with(
            title="test-project",
            subtitle="Claude completed in 1m23s",
            message="Task: Fix the bug\nResult: Fixed the bug and added tests.",
        )
        assert "Prompt #" not in mock_send.call_args.kwargs["message"]

    def test_notify_completion_falls_back_without_context(self, notifier, mocker):
        mock_send = mocker.patch.object(notifier, "send_notification", return_value=True)

        notifier.notify_completion("", agent="Codex", task="", result="")

        mock_send.assert_called_once_with(
            title="Codex",
            subtitle="Codex completed",
            message="Result: Turn completed.",
        )

    def test_notify_job_failed_includes_task_error_and_duration(self, notifier, mocker):
        mock_send = mocker.patch.object(notifier, "send_notification", return_value=True)

        notifier.notify_job_failed(
            "test-project",
            task="Deploy the app",
            error="Rate limit reached",
            duration_str="45s",
        )

        mock_send.assert_called_once_with(
            title="test-project",
            subtitle="Claude failed after 45s",
            message="Task: Deploy the app\nError: Rate limit reached",
        )

    def test_notify_permission_request_includes_task_and_request(self, notifier, mocker):
        mock_send = mocker.patch.object(notifier, "send_notification", return_value=True)

        notifier.notify_permission_request(
            "test-project",
            task="Install dependencies",
            request="Bash — npm install",
        )

        mock_send.assert_called_once_with(
            title="test-project",
            subtitle="Claude needs approval",
            message="Task: Install dependencies\nRequest: Bash — npm install",
        )

    def test_notify_question_includes_task_and_question(self, notifier, mocker):
        mock_send = mocker.patch.object(notifier, "send_notification", return_value=True)

        notifier.notify_question(
            "test-project",
            task="Plan the frontend",
            question="Which framework? (+1 more)",
        )

        mock_send.assert_called_once_with(
            title="test-project",
            subtitle="Claude needs input",
            message="Task: Plan the frontend\nQuestion: Which framework? (+1 more)",
        )

    def test_context_message_normalizes_and_truncates_each_excerpt(self):
        message = _format_context_message(
            "task \n" * 40,
            "Result",
            "detail \t" * 60,
        )
        task_line, detail_line = message.splitlines()
        task_excerpt = task_line.removeprefix("Task: ")
        detail_excerpt = detail_line.removeprefix("Result: ")

        assert len(task_excerpt) == TASK_EXCERPT_LENGTH
        assert len(detail_excerpt) == DETAIL_EXCERPT_LENGTH
        assert task_excerpt.endswith("...")
        assert detail_excerpt.endswith("...")
        assert "\n" not in task_excerpt
        assert "\t" not in detail_excerpt


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
