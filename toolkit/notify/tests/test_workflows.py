"""
Integration tests for complete workflows.

Tests end-to-end workflows including event handlers and notification flows.
"""

import tempfile
from pathlib import Path

import pytest

from ai_notify import config_loader
from ai_notify.config import Config
from ai_notify.database import SessionTracker


class TestUserPromptToStopWorkflow:
    """Test complete workflow from UserPrompt to Stop."""

    @pytest.fixture
    def temp_config(self):
        """Create temporary config with test database."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cfg = Config()
            cfg.db_path = Path(tmpdir) / "test.db"
            cfg.config_dir = Path(tmpdir)
            yield cfg

    @pytest.fixture
    def temp_config_file(self):
        """Create temporary config file."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.yaml"
            # Create default config
            loader = config_loader.ConfigLoader(config_path)
            loader.save(config_loader.AINotifyConfig())
            yield config_path

    def test_complete_workflow_long_job(self, temp_config, temp_config_file):
        """Test complete workflow with a job that exceeds notification threshold."""
        session_id = "test-session-1"
        prompt = "Test prompt"
        cwd = "/Users/test/project"

        # Step 1: Track prompt (simulating UserPrompt event)
        tracker = SessionTracker(temp_config)
        tracker.track_prompt(session_id, prompt, cwd)

        # Verify prompt was tracked
        with tracker._get_connection() as conn:
            cursor = conn.execute(
                "SELECT session_id, prompt, cwd FROM sessions WHERE session_id = ?", (session_id,)
            )
            result = cursor.fetchone()
            assert result is not None
            assert result[0] == session_id
            assert result[1] == prompt
            assert result[2] == cwd

        # Step 2: Simulate some time passing (15 seconds)
        with tracker._get_connection() as conn:
            # Backdate the created_at timestamp to simulate 15 seconds ago
            conn.execute(
                "UPDATE sessions SET created_at = datetime('now', '-15 seconds') WHERE session_id = ?",
                (session_id,),
            )
            conn.commit()

        # Step 3: Mark as stopped (simulating Stop event)
        tracker.mark_stopped(session_id)

        # Step 4: Verify job info
        job_number, duration_seconds, prompt_result = tracker.get_job_info(session_id)
        assert job_number == 1
        assert duration_seconds is not None
        assert duration_seconds >= 14  # Should be ~15 seconds (allow for rounding)
        assert prompt_result == prompt

        # Step 5: Verify notification would be sent (duration >= 10s threshold)
        loader = config_loader.ConfigLoader(temp_config_file)
        cfg = loader.load()
        assert duration_seconds >= cfg.notification.threshold_seconds

    def test_complete_workflow_short_job(self, temp_config, temp_config_file):
        """Test workflow with a job below notification threshold (should be filtered)."""
        session_id = "test-session-2"
        prompt = "Quick test"
        cwd = "/Users/test/project"

        # Track prompt
        tracker = SessionTracker(temp_config)
        tracker.track_prompt(session_id, prompt, cwd)

        # Simulate 5 seconds passing (below 10s threshold)
        with tracker._get_connection() as conn:
            conn.execute(
                "UPDATE sessions SET created_at = datetime('now', '-5 seconds') WHERE session_id = ?",
                (session_id,),
            )
            conn.commit()

        # Mark as stopped
        tracker.mark_stopped(session_id)

        # Verify job info
        job_number, duration_seconds, prompt_result = tracker.get_job_info(session_id)
        assert job_number == 1
        assert duration_seconds is not None
        assert duration_seconds >= 4  # Should be ~5 seconds (allow for rounding)
        assert prompt_result == prompt

        # Verify notification would NOT be sent (duration < 10s threshold)
        loader = config_loader.ConfigLoader(temp_config_file)
        cfg = loader.load()
        assert duration_seconds < cfg.notification.threshold_seconds


class TestPatternFilteringWorkflow:
    """Test complete workflow with prompt pattern filtering."""

    @pytest.fixture
    def temp_config(self):
        """Create temporary config with test database."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cfg = Config()
            cfg.db_path = Path(tmpdir) / "test.db"
            cfg.config_dir = Path(tmpdir)
            yield cfg

    @pytest.fixture
    def temp_config_with_patterns(self):
        """Create temporary config file with exclude patterns."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.yaml"
            # Create config with exclude patterns
            cfg = config_loader.AINotifyConfig(
                notification=config_loader.NotificationConfig(
                    threshold_seconds=10,
                    exclude_patterns=["/commit", "/update-pr"],
                )
            )
            loader = config_loader.ConfigLoader(config_path)
            loader.save(cfg)
            yield config_path

    def test_workflow_excluded_pattern_filtered(self, temp_config, temp_config_with_patterns):
        """Test that prompts matching exclude patterns are filtered even if duration is sufficient."""
        session_id = "test-session-excluded"
        prompt = "/commit --all"
        cwd = "/Users/test/project"

        # Track prompt with excluded pattern
        tracker = SessionTracker(temp_config)
        tracker.track_prompt(session_id, prompt, cwd)

        # Simulate 15 seconds passing (above threshold)
        with tracker._get_connection() as conn:
            conn.execute(
                "UPDATE sessions SET created_at = datetime('now', '-15 seconds') WHERE session_id = ?",
                (session_id,),
            )
            conn.commit()

        # Mark as stopped
        tracker.mark_stopped(session_id)

        # Verify job info
        job_number, duration_seconds, prompt_result = tracker.get_job_info(session_id)
        assert job_number == 1
        assert duration_seconds is not None
        assert duration_seconds >= 14
        assert prompt_result == prompt

        # Verify that despite meeting duration threshold, notification should be filtered
        loader = config_loader.ConfigLoader(temp_config_with_patterns)
        cfg = loader.load()
        from ai_notify.helpers.filters import should_send_notification

        assert not should_send_notification(prompt_result or "", duration_seconds, cfg)

    def test_workflow_excluded_pattern_prefix_match(self, temp_config, temp_config_with_patterns):
        """Test that prefix matching works for exclude patterns."""
        session_id = "test-session-prefix"
        prompt = "/update-pr with new changes"
        cwd = "/Users/test/project"

        # Track prompt starting with excluded pattern
        tracker = SessionTracker(temp_config)
        tracker.track_prompt(session_id, prompt, cwd)

        # Simulate 15 seconds passing (above threshold)
        with tracker._get_connection() as conn:
            conn.execute(
                "UPDATE sessions SET created_at = datetime('now', '-15 seconds') WHERE session_id = ?",
                (session_id,),
            )
            conn.commit()

        # Mark as stopped
        tracker.mark_stopped(session_id)

        # Verify job info
        job_number, duration_seconds, prompt_result = tracker.get_job_info(session_id)
        assert job_number == 1
        assert duration_seconds is not None
        assert duration_seconds >= 14
        assert prompt_result == prompt

        # Verify notification is filtered
        loader = config_loader.ConfigLoader(temp_config_with_patterns)
        cfg = loader.load()
        from ai_notify.helpers.filters import should_send_notification

        assert not should_send_notification(prompt_result or "", duration_seconds, cfg)

    def test_workflow_non_excluded_pattern_sent(self, temp_config, temp_config_with_patterns):
        """Test that prompts not matching exclude patterns trigger notifications."""
        session_id = "test-session-allowed"
        prompt = "Fix the authentication bug"
        cwd = "/Users/test/project"

        # Track prompt without excluded pattern
        tracker = SessionTracker(temp_config)
        tracker.track_prompt(session_id, prompt, cwd)

        # Simulate 15 seconds passing (above threshold)
        with tracker._get_connection() as conn:
            conn.execute(
                "UPDATE sessions SET created_at = datetime('now', '-15 seconds') WHERE session_id = ?",
                (session_id,),
            )
            conn.commit()

        # Mark as stopped
        tracker.mark_stopped(session_id)

        # Verify job info
        job_number, duration_seconds, prompt_result = tracker.get_job_info(session_id)
        assert job_number == 1
        assert duration_seconds is not None
        assert duration_seconds >= 14
        assert prompt_result == prompt

        # Verify notification should be sent
        loader = config_loader.ConfigLoader(temp_config_with_patterns)
        cfg = loader.load()
        from ai_notify.helpers.filters import should_send_notification

        assert should_send_notification(prompt_result or "", duration_seconds, cfg)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
