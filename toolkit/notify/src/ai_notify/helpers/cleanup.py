"""
Auto-cleanup helper functions for managing database maintenance.
"""

from datetime import datetime, timedelta

from loguru import logger

from ai_notify.config_loader import DEFAULT_CONFIG_DIR


# Timestamp file for tracking last cleanup
LAST_CLEANUP_FILE = DEFAULT_CONFIG_DIR / ".last_cleanup"


def should_run_auto_cleanup() -> bool:
    """
    Check if auto-cleanup should run (if >24 hours since last cleanup).

    Returns:
        True if cleanup should run, False otherwise
    """
    if not LAST_CLEANUP_FILE.exists():
        return True

    try:
        last_cleanup = datetime.fromtimestamp(LAST_CLEANUP_FILE.stat().st_mtime)
        return datetime.now() - last_cleanup > timedelta(hours=24)
    except OSError:
        return True


def mark_cleanup_done() -> None:
    """Mark cleanup as completed by updating timestamp file."""
    try:
        LAST_CLEANUP_FILE.parent.mkdir(parents=True, exist_ok=True)
        LAST_CLEANUP_FILE.touch()
    except OSError as e:
        logger.warning(f"Failed to mark cleanup done: {e}")
