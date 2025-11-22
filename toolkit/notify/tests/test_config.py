"""
Tests for configuration loading and validation.
"""

import tempfile
from pathlib import Path

import pytest

from ai_notify import config_loader


class TestConfigLoader:
    """Test configuration loader."""

    def test_load_default_config(self):
        """Test loading default configuration."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "nonexistent.yaml"
            loader = config_loader.ConfigLoader(config_path)
            cfg = loader.load()

            # Should load defaults
            assert cfg.notification.threshold_seconds == 10
            assert cfg.cleanup.retention_days == 30
            assert cfg.cleanup.auto_cleanup_enabled is True

    def test_load_custom_config(self):
        """Test loading custom configuration."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "custom.yaml"

            # Create custom config
            custom_cfg = config_loader.AINotifyConfig()
            custom_cfg.notification.threshold_seconds = 15
            custom_cfg.cleanup.retention_days = 60

            loader = config_loader.ConfigLoader(config_path)
            loader.save(custom_cfg)

            # Load and verify
            loaded_cfg = loader.load()
            assert loaded_cfg.notification.threshold_seconds == 15
            assert loaded_cfg.cleanup.retention_days == 60

    def test_config_validation(self):
        """Test configuration validation."""
        # Invalid log level should raise error
        with pytest.raises(Exception):
            config_loader.LoggingConfig(level="INVALID")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
