"""
Tests for database operations and cleanup.
"""

import json
import tempfile
from datetime import datetime, timedelta
from pathlib import Path
from unittest.mock import patch

import pytest

from ai_notify.config import Config
from ai_notify.database import SessionTracker


class TestSessionTracker:
    """Test SessionTracker database operations."""

    @pytest.fixture
    def temp_config(self):
        """Create temporary config with test database."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config = Config()
            config.db_path = Path(tmpdir) / "test.db"
            config.config_dir = Path(tmpdir)
            yield config

    def test_track_prompt(self, temp_config):
        tracker = SessionTracker(temp_config)
        tracker.track_prompt("session-1", "test prompt", "/Users/test/project")

        # Verify insertion
        with tracker._get_connection() as conn:
            cursor = conn.execute("SELECT session_id, prompt, cwd, job_number FROM sessions")
            result = cursor.fetchone()
            assert result[0] == "session-1"
            assert result[1] == "test prompt"
            assert result[2] == "/Users/test/project"
            assert result[3] == 1  # Auto-incremented job number

    def test_job_number_increment(self, temp_config):
        tracker = SessionTracker(temp_config)
        tracker.track_prompt("session-1", "prompt 1", "/Users/test/project")
        tracker.track_prompt("session-1", "prompt 2", "/Users/test/project")

        with tracker._get_connection() as conn:
            cursor = conn.execute(
                "SELECT job_number FROM sessions WHERE session_id = 'session-1' ORDER BY id"
            )
            results = cursor.fetchall()
            assert results[0][0] == 1
            assert results[1][0] == 2

    def test_mark_stopped(self, temp_config):
        tracker = SessionTracker(temp_config)
        tracker.track_prompt("session-1", "test", "/Users/test/project")
        tracker.mark_stopped("session-1")

        with tracker._get_connection() as conn:
            cursor = conn.execute(
                "SELECT stopped_at, duration_seconds FROM sessions WHERE session_id = 'session-1'"
            )
            result = cursor.fetchone()
            assert result[0] is not None  # stopped_at should be set
            assert result[1] is not None  # duration should be calculated

    def test_get_job_info(self, temp_config):
        tracker = SessionTracker(temp_config)
        tracker.track_prompt("session-1", "test", "/Users/test/project")
        tracker.mark_stopped("session-1")

        job_number, duration, prompt = tracker.get_job_info("session-1")
        assert job_number == 1
        assert duration is not None
        assert duration >= 0
        assert prompt == "test"


class TestDataCleanup:
    """Test data cleanup functionality."""

    @pytest.fixture
    def temp_config(self):
        """Create temporary config with test database."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cfg = Config()
            cfg.db_path = Path(tmpdir) / "test.db"
            cfg.config_dir = Path(tmpdir)
            yield cfg

    def test_cleanup_old_data_with_export(self, temp_config):
        """Test cleanup with export functionality."""
        tracker = SessionTracker(temp_config)

        # Insert old sessions (31 days ago)
        with tracker._get_connection() as conn:
            old_timestamp = int((datetime.now() - timedelta(days=31)).timestamp())
            for i in range(5):
                conn.execute(
                    "INSERT INTO sessions (session_id, prompt, cwd, created_at) VALUES (?, ?, ?, ?)",
                    (f"old-session-{i}", f"old prompt {i}", "/test", old_timestamp),
                )

            # Insert recent sessions (5 days ago)
            recent_timestamp = int((datetime.now() - timedelta(days=5)).timestamp())
            for i in range(3):
                conn.execute(
                    "INSERT INTO sessions (session_id, prompt, cwd, created_at) VALUES (?, ?, ?, ?)",
                    (f"recent-session-{i}", f"recent prompt {i}", "/test", recent_timestamp),
                )
            conn.commit()

        # Run cleanup with 30 day retention
        with tempfile.TemporaryDirectory() as export_dir:
            with patch("ai_notify.database.EXPORT_DIR", Path(export_dir)):
                stats = tracker.cleanup_old_data(retention_days=30, export_before=True)

        # Verify stats
        assert stats["rows_deleted"] == 5  # Should delete 5 old sessions
        assert stats["rows_exported"] > 0  # Should export before cleanup

        # Verify recent sessions remain
        with tracker._get_connection() as conn:
            cursor = conn.execute("SELECT COUNT(*) FROM sessions")
            count = cursor.fetchone()[0]
            assert count == 3  # Only recent sessions remain

    def test_cleanup_without_export(self, temp_config):
        """Test cleanup without export."""
        tracker = SessionTracker(temp_config)

        # Insert old sessions
        with tracker._get_connection() as conn:
            old_timestamp = int((datetime.now() - timedelta(days=60)).timestamp())
            for i in range(10):
                conn.execute(
                    "INSERT INTO sessions (session_id, prompt, cwd, created_at) VALUES (?, ?, ?, ?)",
                    (f"old-session-{i}", f"old prompt {i}", "/test", old_timestamp),
                )
            conn.commit()

        # Run cleanup without export
        stats = tracker.cleanup_old_data(retention_days=30, export_before=False)

        # Verify stats
        assert stats["rows_deleted"] == 10
        assert stats["rows_exported"] == 0  # No export

    def test_export_to_json(self, temp_config):
        """Test JSON export functionality."""
        tracker = SessionTracker(temp_config)

        # Insert test data
        tracker.track_prompt("session-1", "prompt 1", "/test1")
        tracker.track_prompt("session-2", "prompt 2", "/test2")

        # Export to JSON
        with tempfile.TemporaryDirectory() as tmpdir:
            export_path = Path(tmpdir) / "export.json"
            count = tracker.export_to_json(export_path)

            assert count == 2
            assert export_path.exists()

            # Verify JSON content
            with open(export_path, "r") as f:
                data = json.load(f)
                assert len(data) == 2
                assert any(s["session_id"] == "session-1" for s in data)
                assert any(s["session_id"] == "session-2" for s in data)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
