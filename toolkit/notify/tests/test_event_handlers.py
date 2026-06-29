"""
Tests for Claude Code event handler payload parsing (v2.1.x schema).
"""

from unittest.mock import MagicMock, patch

from ai_notify.events.notification import handle_notification
from ai_notify.events.permission_request import handle_permission


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
