"""
Tests for utility functions.
"""

from unittest.mock import patch

import pytest
from loguru import logger

from ai_notify.utils import format_duration, load_json_payload, setup_logging, validate_input


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


class TestJsonParsing:
    """Test JSON parsing helpers."""

    def test_load_json_payload_str(self):
        assert load_json_payload('{"ok": true, "n": 1}') == {"ok": True, "n": 1}

    def test_load_json_payload_bytes(self):
        assert load_json_payload(b'{"ok": true, "n": 1}') == {"ok": True, "n": 1}


class TestLoggingSetup:
    """Test logging setup behavior."""

    def test_setup_logging_disabled(self, monkeypatch):
        monkeypatch.setenv("AI_NOTIFY_LOG", "0")
        with patch.object(logger, "remove") as remove_mock, patch.object(logger, "add") as add_mock:
            setup_logging()
            assert remove_mock.called
            add_mock.assert_not_called()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
