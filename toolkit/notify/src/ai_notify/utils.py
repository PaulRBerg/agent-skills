"""
Utility functions for ai-notify.
"""

import json
import sys
from typing import Any

from loguru import logger

from ai_notify.config import Config


def format_duration(seconds: int) -> str:
    """
    Format duration in seconds to human-readable string.

    Examples:
        53 -> "53s"
        130 -> "2m10s"
        413 -> "6m53s"
        240 -> "4m"
        3661 -> "1h1m"

    Args:
        seconds: Duration in seconds

    Returns:
        Human-readable duration string
    """
    if seconds < 60:
        return f"{seconds}s"

    minutes = seconds // 60
    remaining_seconds = seconds % 60

    if minutes < 60:
        if remaining_seconds == 0:
            return f"{minutes}m"
        return f"{minutes}m{remaining_seconds}s"

    hours = minutes // 60
    remaining_minutes = minutes % 60

    if remaining_minutes == 0:
        return f"{hours}h"
    return f"{hours}h{remaining_minutes}m"


def setup_logging() -> None:
    """
    Set up logging with rotation using loguru.

    Configures loguru to write to the configured log file with rotation.
    """
    config = Config()
    config.ensure_directories()

    # Remove default handler and add file handler with rotation
    logger.remove()  # Remove default handler
    logger.add(
        config.log_path,
        rotation="10 MB",
        retention=5,
        format="{time:YYYY-MM-DD HH:mm:ss} - {name} - {level} - {message}",
        level="INFO",
    )


def validate_input(data: dict[str, Any]) -> None:
    """
    Validate and sanitize input data for security.

    Raises:
        ValueError: If input validation fails

    Args:
        data: Input data dictionary to validate
    """
    # Check for path traversal in cwd
    if "cwd" in data:
        cwd = str(data["cwd"])
        if ".." in cwd:
            raise ValueError("Path traversal detected in cwd")

    # Validate session_id format if present
    if "session_id" in data:
        session_id = str(data["session_id"])
        if not session_id or len(session_id) > 255:
            raise ValueError("Invalid session_id")


def read_stdin_json() -> dict[str, Any]:
    """
    Read and parse JSON from stdin.

    Returns:
        Parsed JSON data

    Raises:
        ValueError: If JSON parsing fails
    """
    try:
        data = json.load(sys.stdin)
        return data
    except json.JSONDecodeError as e:
        raise ValueError(f"Failed to parse JSON from stdin: {e}")
    except Exception as e:
        raise ValueError(f"Failed to read stdin: {e}")
