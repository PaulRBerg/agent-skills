"""
Tests for Claude Code event handler payload parsing (v2.1.x schema).
"""

from unittest.mock import MagicMock, patch

import pytest

from ai_notify.config_loader import AINotifyConfig, NotificationConfig, NotificationMode
from ai_notify.events.notification import handle_notification
from ai_notify.events.permission_request import handle_permission
from ai_notify.events.stop import handle_stop
from ai_notify.events.stop_failure import _failure_message, handle_stop_failure


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
        tracker.get_active_job_number.return_value = 7
        mock_tracker_cls.return_value = tracker

        notifier = MagicMock()
        notifier.get_project_name.return_value = "proj"
        mock_notifier_cls.return_value = notifier

        handle_permission(
            {
                "session_id": "s1",
                "cwd": "/tmp/p",
                "tool_name": "AskUserQuestion",
                "tool_input": {},
            }
        )

        notifier.notify_permission_request.assert_called_once_with(
            "proj", "Tool: AskUserQuestion", 7
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
        tracker.get_active_job_number.return_value = 4
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
            "project", "API Error: rate limit reached", 4
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
        tracker.get_active_job_number.return_value = 2
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
        tracker.get_active_job_number.return_value = None
        mock_tracker_cls.return_value = tracker
        notifier = MagicMock()
        notifier.get_project_name.return_value = ""
        mock_notifier_cls.return_value = notifier

        handle_stop_failure({"session_id": "untracked", "error_details": "Service unavailable"})

        notifier.notify_job_failed.assert_called_once_with(
            "Claude Code", "Service unavailable", None
        )

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

    def test_failure_message_is_normalized_and_capped(self):
        message = _failure_message({"last_assistant_message": "word \n" * 100})

        assert "\n" not in message
        assert len(message) == 320
        assert message.endswith("...")
