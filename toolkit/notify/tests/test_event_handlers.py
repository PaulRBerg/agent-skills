"""
Tests for Claude Code event handler payload parsing (v2.1.x schema).
"""

from unittest.mock import MagicMock, patch

import pytest

from ai_notify.config_loader import AINotifyConfig, NotificationConfig, NotificationMode
from ai_notify.events.ask_user_question import handle_ask_user_question
from ai_notify.events.notification import handle_notification
from ai_notify.events.permission_request import _permission_request, handle_permission
from ai_notify.events.stop import handle_stop
from ai_notify.events.stop_failure import _failure_message, handle_stop_failure
from ai_notify.events.user_prompt_submit import handle_user_prompt


class TestUserPromptSubmit:
    @pytest.mark.parametrize(
        "prompt",
        [
            "<task-notification>\n<task-id>task-1</task-id>\n</task-notification>",
            "<subagent_notification>completed</subagent_notification>",
            "<task-notifi\x1b]11;rgb:2f4f/3403/3f33\x1b\\cation>completed</task-notification>",
        ],
    )
    @patch("ai_notify.events.user_prompt_submit.SessionTracker")
    def test_ignores_internal_agent_notifications(self, mock_tracker_cls, prompt):
        handle_user_prompt({"session_id": "s1", "cwd": "/tmp/project", "prompt": prompt})

        mock_tracker_cls.assert_not_called()

    @patch("ai_notify.events.user_prompt_submit.SessionTracker")
    def test_tracks_user_prompt_that_mentions_internal_tag(self, mock_tracker_cls):
        tracker = MagicMock()
        mock_tracker_cls.return_value = tracker
        prompt = "Fix notifications that include <task-notification> XML."

        handle_user_prompt({"session_id": "s1", "cwd": "/tmp/project", "prompt": prompt})

        tracker.track_prompt.assert_called_once_with("s1", prompt, "/tmp/project")


class TestNotificationWaitingDetection:
    @patch("ai_notify.events.notification.SessionTracker")
    def test_idle_prompt_type_marks_waiting(self, mock_tracker_cls):
        """A structured idle_prompt notification marks waiting without any keyword."""
        tracker = MagicMock()
        mock_tracker_cls.return_value = tracker

        handle_notification(
            {"session_id": "s1", "message": "Anything", "notification_type": "idle_prompt"}
        )

        tracker.mark_waiting.assert_called_once_with("s1")

    @patch("ai_notify.events.notification.SessionTracker")
    def test_non_waiting_notification_does_not_mark(self, mock_tracker_cls):
        tracker = MagicMock()
        mock_tracker_cls.return_value = tracker

        handle_notification(
            {"session_id": "s1", "message": "Build complete", "notification_type": "auth_success"}
        )

        tracker.mark_waiting.assert_not_called()


class TestPermissionToolName:
    @patch("ai_notify.events.permission_request.MacNotifier")
    @patch("ai_notify.events.permission_request.SessionTracker")
    @patch(
        "ai_notify.events.permission_request.should_send_permission_notification",
        return_value=True,
    )
    @patch("ai_notify.events.permission_request.get_runtime_config")
    def test_reads_top_level_tool_name(
        self, mock_config, mock_send, mock_tracker_cls, mock_notifier_cls
    ):
        """Current payloads put the tool name at top-level tool_name, not tool_input.name."""
        tracker = MagicMock()
        tracker.get_active_prompt.return_value = "Choose a framework"
        mock_tracker_cls.return_value = tracker

        notifier = MagicMock()
        notifier.get_project_name.return_value = "proj"
        mock_notifier_cls.return_value = notifier

        handle_permission(
            {
                "session_id": "s1",
                "cwd": "/tmp/p",
                "tool_name": "Bash",
                "tool_input": {
                    "command": "npm install",
                    "description": "Install dependencies",
                },
            }
        )

        notifier.notify_permission_request.assert_called_once_with(
            "proj",
            task="Choose a framework",
            request="Bash — npm install",
        )

    @pytest.mark.parametrize(
        ("tool_name", "tool_input", "expected"),
        [
            ("Write", {"file_path": "/tmp/result.txt"}, "Write — /tmp/result.txt"),
            ("WebFetch", {"url": "https://example.com"}, "WebFetch — https://example.com"),
            ("", {"name": "CustomTool", "description": "Run it"}, "CustomTool — Run it"),
            ("Bash", {}, "Bash"),
            (None, None, "Permission requested"),
        ],
    )
    def test_permission_request_context_fallbacks(self, tool_name, tool_input, expected):
        assert _permission_request(tool_name, tool_input) == expected


class TestAskUserQuestion:
    @patch("ai_notify.events.ask_user_question.MacNotifier")
    @patch("ai_notify.events.ask_user_question.SessionTracker")
    @patch(
        "ai_notify.events.ask_user_question.should_send_permission_notification",
        return_value=True,
    )
    @patch("ai_notify.events.ask_user_question.get_runtime_config")
    def test_includes_active_task_and_remaining_question_count(
        self, mock_config, mock_send, mock_tracker_cls, mock_notifier_cls
    ):
        tracker = MagicMock()
        tracker.get_active_prompt.return_value = "Plan the frontend"
        mock_tracker_cls.return_value = tracker
        notifier = MagicMock()
        notifier.get_project_name.return_value = "project"
        mock_notifier_cls.return_value = notifier

        handle_ask_user_question(
            {
                "session_id": "s1",
                "cwd": "/tmp/project",
                "tool_input": {
                    "questions": [
                        {"question": "Which framework?"},
                        {"question": "Which test runner?"},
                    ]
                },
            }
        )

        notifier.notify_question.assert_called_once_with(
            "project",
            task="Plan the frontend",
            question="Which framework? (+1 more)",
        )


class TestStopPendingWork:
    @pytest.mark.parametrize(
        "pending",
        [
            {"background_tasks": [{"id": "task-1"}]},
            {"session_crons": [{"id": "cron-1"}]},
        ],
    )
    @patch("ai_notify.events.stop.SessionTracker")
    def test_pending_work_defers_completion(self, mock_tracker_cls, pending):
        handle_stop({"session_id": "s1", "cwd": "/tmp/project", **pending})

        mock_tracker_cls.assert_not_called()

    @pytest.mark.parametrize("extra", [{}, {"background_tasks": [], "session_crons": []}])
    @patch("ai_notify.events.stop.get_runtime_config")
    @patch("ai_notify.events.stop.SessionTracker")
    def test_absent_or_empty_pending_fields_preserve_completion(
        self, mock_tracker_cls, mock_config, extra
    ):
        tracker = MagicMock()
        tracker.get_job_info.return_value = (None, None, None)
        mock_tracker_cls.return_value = tracker
        mock_config.return_value = AINotifyConfig()

        handle_stop({"session_id": "s1", "cwd": "/tmp/project", **extra})

        tracker.mark_stopped.assert_called_once_with("s1")

    @patch("ai_notify.events.stop.MacNotifier")
    @patch("ai_notify.events.stop.get_runtime_config")
    @patch("ai_notify.events.stop.SessionTracker")
    def test_completion_includes_prompt_and_final_response(
        self, mock_tracker_cls, mock_config, mock_notifier_cls
    ):
        tracker = MagicMock()
        tracker.get_job_info.return_value = (3, 65, "Fix authentication")
        mock_tracker_cls.return_value = tracker
        config = AINotifyConfig()
        config.notification.threshold_seconds = 0
        config.cleanup.auto_cleanup_enabled = False
        mock_config.return_value = config
        notifier = MagicMock()
        notifier.get_project_name.return_value = "project"
        mock_notifier_cls.return_value = notifier

        handle_stop(
            {
                "session_id": "s1",
                "cwd": "/tmp/project",
                "last_assistant_message": "Fixed authentication and added tests.",
            }
        )

        notifier.notify_completion.assert_called_once_with(
            "project",
            agent="Claude",
            task="Fix authentication",
            result="Fixed authentication and added tests.",
            duration_str="1m5s",
        )


class TestStopFailure:
    @patch("ai_notify.events.stop_failure.MacNotifier")
    @patch("ai_notify.events.stop_failure.SessionTracker")
    @patch("ai_notify.events.stop_failure.get_runtime_config")
    def test_tracked_failure_ignores_threshold_and_excludes(
        self, mock_config, mock_tracker_cls, mock_notifier_cls
    ):
        mock_config.return_value = AINotifyConfig(
            notification=NotificationConfig(
                mode=NotificationMode.ALL,
                threshold_seconds=999,
                exclude_patterns=["/skip"],
            )
        )
        tracker = MagicMock()
        tracker.get_active_prompt.return_value = "/skip fix authentication"
        tracker.get_job_info.return_value = (4, 75, "/skip fix authentication")
        mock_tracker_cls.return_value = tracker
        notifier = MagicMock()
        notifier.get_project_name.return_value = "project"
        mock_notifier_cls.return_value = notifier

        handle_stop_failure(
            {
                "session_id": "s1",
                "cwd": "/tmp/project",
                "last_assistant_message": "  API Error:\n rate limit reached  ",
            }
        )

        tracker.mark_stopped.assert_called_once_with("s1")
        notifier.notify_job_failed.assert_called_once_with(
            "project",
            task="/skip fix authentication",
            error="API Error: rate limit reached",
            duration_str="1m15s",
        )

    @pytest.mark.parametrize("mode", [NotificationMode.PERMISSION_ONLY, NotificationMode.DISABLED])
    @patch("ai_notify.events.stop_failure.MacNotifier")
    @patch("ai_notify.events.stop_failure.SessionTracker")
    @patch("ai_notify.events.stop_failure.get_runtime_config")
    def test_failure_notification_requires_all_mode(
        self, mock_config, mock_tracker_cls, mock_notifier_cls, mode
    ):
        mock_config.return_value = AINotifyConfig(notification=NotificationConfig(mode=mode))
        tracker = MagicMock()
        tracker.get_active_prompt.return_value = "Build the app"
        tracker.get_job_info.return_value = (2, 3, "Build the app")
        mock_tracker_cls.return_value = tracker

        handle_stop_failure({"session_id": "s1", "error": "server_error"})

        tracker.mark_stopped.assert_called_once_with("s1")
        mock_notifier_cls.assert_not_called()

    @patch("ai_notify.events.stop_failure.MacNotifier")
    @patch("ai_notify.events.stop_failure.SessionTracker")
    @patch("ai_notify.events.stop_failure.get_runtime_config")
    def test_missing_job_still_sends_generic_failure(
        self, mock_config, mock_tracker_cls, mock_notifier_cls
    ):
        mock_config.return_value = AINotifyConfig()
        tracker = MagicMock()
        tracker.get_active_prompt.return_value = None
        mock_tracker_cls.return_value = tracker
        notifier = MagicMock()
        notifier.get_project_name.return_value = ""
        mock_notifier_cls.return_value = notifier

        handle_stop_failure({"session_id": "untracked", "error_details": "Service unavailable"})

        notifier.notify_job_failed.assert_called_once_with(
            "Claude Code",
            task="",
            error="Service unavailable",
            duration_str=None,
        )
        tracker.get_job_info.assert_not_called()

    @pytest.mark.parametrize(
        ("payload", "expected"),
        [
            ({"error_details": "details"}, "details"),
            ({"error": "rate_limit"}, "rate_limit"),
            ({}, "Claude Code API error"),
        ],
    )
    def test_failure_message_fallbacks(self, payload, expected):
        assert _failure_message(payload) == expected

    def test_failure_message_is_normalized(self):
        message = _failure_message({"last_assistant_message": "word \n" * 100})

        assert "\n" not in message
        assert message == " ".join(["word"] * 100)
