"""
Tests for notification mode controls.
"""

from ai_notify.config_loader import AINotifyConfig, NotificationConfig, NotificationMode
from ai_notify.helpers.filters import (
    should_send_completion_notification,
    should_send_permission_notification,
)


class TestNotificationModes:
    """Test notification mode filtering."""

    def test_mode_all_allows_completion_notifications(self):
        """ALL mode allows completion notifications (subject to other filters)."""
        config = AINotifyConfig(
            notification=NotificationConfig(mode=NotificationMode.ALL, threshold_seconds=10)
        )

        # Should allow notification when duration meets threshold
        assert should_send_completion_notification("test prompt", 15, config) is True

        # Should still apply duration filter
        assert should_send_completion_notification("test prompt", 5, config) is False

    def test_mode_all_allows_permission_notifications(self):
        """ALL mode allows permission notifications."""
        config = AINotifyConfig(notification=NotificationConfig(mode=NotificationMode.ALL))

        assert should_send_permission_notification(config) is True

    def test_mode_permission_only_blocks_completion(self):
        """PERMISSION_ONLY blocks completion notifications."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                mode=NotificationMode.PERMISSION_ONLY, threshold_seconds=10
            )
        )

        # Should block completion even if duration meets threshold
        assert should_send_completion_notification("test prompt", 100, config) is False

    def test_mode_permission_only_allows_permission(self):
        """PERMISSION_ONLY allows permission notifications."""
        config = AINotifyConfig(
            notification=NotificationConfig(mode=NotificationMode.PERMISSION_ONLY)
        )

        assert should_send_permission_notification(config) is True

    def test_mode_disabled_blocks_completion(self):
        """DISABLED blocks completion notifications."""
        config = AINotifyConfig(
            notification=NotificationConfig(mode=NotificationMode.DISABLED, threshold_seconds=10)
        )

        # Should block completion regardless of duration
        assert should_send_completion_notification("test prompt", 100, config) is False

    def test_mode_disabled_blocks_permission(self):
        """DISABLED blocks permission notifications."""
        config = AINotifyConfig(notification=NotificationConfig(mode=NotificationMode.DISABLED))

        assert should_send_permission_notification(config) is False

    def test_mode_all_still_applies_exclude_patterns(self):
        """ALL mode still applies existing exclude pattern filters."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                mode=NotificationMode.ALL,
                threshold_seconds=10,
                exclude_patterns=["/fix", "/test"],
            )
        )

        # Should block notification for excluded pattern even if duration meets threshold
        assert should_send_completion_notification("/fix bug", 100, config) is False
        assert should_send_completion_notification("/test code", 100, config) is False

        # Should allow notification for non-excluded pattern with sufficient duration
        assert should_send_completion_notification("implement feature", 100, config) is True

    def test_default_mode_is_all(self):
        """Default notification mode should be ALL."""
        config = AINotifyConfig()

        assert config.notification.mode == NotificationMode.ALL

    def test_mode_enum_values(self):
        """Test that NotificationMode enum has expected values."""
        assert NotificationMode.ALL.value == "all"
        assert NotificationMode.PERMISSION_ONLY.value == "permission_only"
        assert NotificationMode.DISABLED.value == "disabled"

    def test_mode_from_string(self):
        """Test creating NotificationMode from string values."""
        assert NotificationMode("all") == NotificationMode.ALL
        assert NotificationMode("permission_only") == NotificationMode.PERMISSION_ONLY
        assert NotificationMode("disabled") == NotificationMode.DISABLED
