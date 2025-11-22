"""
Tests for utility functions.
"""

import pytest

from ai_notify.utils import format_duration, validate_input


class TestFormatDuration:
    """Test duration formatting."""

    def test_seconds_only(self):
        assert format_duration(53) == "53s"
        assert format_duration(0) == "0s"
        assert format_duration(59) == "59s"

    def test_minutes_and_seconds(self):
        assert format_duration(130) == "2m10s"
        assert format_duration(413) == "6m53s"

    def test_minutes_only(self):
        assert format_duration(240) == "4m"
        assert format_duration(60) == "1m"

    def test_hours(self):
        assert format_duration(3661) == "1h1m"
        assert format_duration(3600) == "1h"
        assert format_duration(7384) == "2h3m"


class TestValidateInput:
    """Test input validation."""

    def test_valid_input(self):
        data = {
            "session_id": "test-session",
            "cwd": "/Users/test/project",
            "prompt": "test prompt",
        }
        # Should not raise
        validate_input(data)

    def test_path_traversal(self):
        data = {"cwd": "/Users/test/../../../etc/passwd"}
        with pytest.raises(ValueError, match="Path traversal"):
            validate_input(data)

    def test_invalid_session_id(self):
        data = {"session_id": ""}
        with pytest.raises(ValueError, match="Invalid session_id"):
            validate_input(data)

    def test_long_session_id(self):
        data = {"session_id": "x" * 256}
        with pytest.raises(ValueError, match="Invalid session_id"):
            validate_input(data)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
