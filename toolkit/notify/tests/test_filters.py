"""
Tests for notification filtering logic.
"""

from ai_notify.config_loader import AINotifyConfig, NotificationConfig
from ai_notify.helpers.filters import should_send_notification


class TestNotificationFiltering:
    """Test should_send_notification() function."""

    def test_duration_below_threshold_filtered(self):
        """Test that notifications are filtered when duration < threshold."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=[],
            )
        )

        # Duration below threshold should be filtered
        assert not should_send_notification("any prompt", 5, config)
        assert not should_send_notification("any prompt", 9, config)

    def test_duration_meets_threshold_sent(self):
        """Test that notifications are sent when duration >= threshold."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=[],
            )
        )

        # Duration at or above threshold should pass
        assert should_send_notification("any prompt", 10, config)
        assert should_send_notification("any prompt", 15, config)
        assert should_send_notification("any prompt", 100, config)

    def test_zero_threshold_never_filters_by_duration(self):
        """Test that threshold of 0 means all durations pass the duration check."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=0,
                exclude_patterns=[],
            )
        )

        # All durations should pass when threshold is 0
        assert should_send_notification("prompt", 0, config)
        assert should_send_notification("prompt", 1, config)
        assert should_send_notification("prompt", 100, config)

    def test_exact_pattern_match_filtered(self):
        """Test that prompts matching exclude patterns are filtered."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=["/commit"],
            )
        )

        # Exact match should be filtered
        assert not should_send_notification("/commit", 15, config)

    def test_prefix_pattern_match_filtered(self):
        """Test that prompts starting with patterns are filtered (prefix matching)."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=["/commit"],
            )
        )

        # Prompts starting with pattern should be filtered
        assert not should_send_notification("/commit --all", 15, config)
        assert not should_send_notification("/commit -m 'test'", 15, config)
        assert not should_send_notification("/commit\n\nMore text", 15, config)

    def test_case_sensitive_matching(self):
        """Test that pattern matching is case-sensitive."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=["/commit"],
            )
        )

        # Different case should NOT match (case-sensitive)
        assert should_send_notification("/Commit", 15, config)
        assert should_send_notification("/COMMIT", 15, config)
        assert should_send_notification("Commit", 15, config)

    def test_non_matching_pattern_sent(self):
        """Test that prompts not matching patterns are sent."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=["/commit"],
            )
        )

        # Non-matching prompts should pass
        assert should_send_notification("Fix the bug", 15, config)
        assert should_send_notification("add new feature", 15, config)
        assert should_send_notification("commit changes", 15, config)  # Not at start

    def test_multiple_patterns(self):
        """Test filtering with multiple exclude patterns."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=["/commit", "/update-pr", "/fix-issue"],
            )
        )

        # Each pattern should filter
        assert not should_send_notification("/commit --all", 15, config)
        assert not should_send_notification("/update-pr", 15, config)
        assert not should_send_notification("/fix-issue 123", 15, config)

        # Non-matching should pass
        assert should_send_notification("Regular task", 15, config)

    def test_empty_patterns_list(self):
        """Test that empty patterns list doesn't filter anything."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=[],
            )
        )

        # All prompts should pass when no patterns
        assert should_send_notification("/commit", 15, config)
        assert should_send_notification("anything", 15, config)

    def test_empty_prompt_not_filtered(self):
        """Test that empty prompts are not filtered by patterns."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=["/commit"],
            )
        )

        # Empty prompt should pass pattern check (only duration check applies)
        assert should_send_notification("", 15, config)

    def test_combined_duration_and_pattern_filtering(self):
        """Test that both duration and pattern filters are applied."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=["/commit"],
            )
        )

        # Duration too short - filtered regardless of pattern
        assert not should_send_notification("Regular task", 5, config)
        assert not should_send_notification("/commit", 5, config)

        # Duration ok but pattern matches - filtered
        assert not should_send_notification("/commit", 15, config)

        # Duration ok and pattern doesn't match - sent
        assert should_send_notification("Regular task", 15, config)

    def test_pattern_not_at_beginning(self):
        """Test that patterns only match at the beginning (not in middle)."""
        config = AINotifyConfig(
            notification=NotificationConfig(
                threshold_seconds=10,
                exclude_patterns=["/commit"],
            )
        )

        # Pattern in middle should NOT be filtered
        assert should_send_notification("Run /commit command", 15, config)
        assert should_send_notification("Please /commit the changes", 15, config)
