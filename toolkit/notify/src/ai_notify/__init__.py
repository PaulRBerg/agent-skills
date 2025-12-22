"""
ai-notify: Desktop notification system for Claude Code and Codex CLI.

Tracks session activity and sends macOS notifications for key events.
"""

__version__ = "1.0.0"
__author__ = "Paul Razvan Berg"

from ai_notify.config import Config
from ai_notify.database import SessionTracker
from ai_notify.notifier import MacNotifier
from ai_notify.utils import format_duration, setup_logging, validate_input, read_stdin_json

__all__ = [
    "Config",
    "SessionTracker",
    "MacNotifier",
    "format_duration",
    "setup_logging",
    "validate_input",
    "read_stdin_json",
]
