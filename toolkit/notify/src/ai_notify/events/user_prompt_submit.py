"""
UserPromptSubmit event handler.
"""

import re

from loguru import logger

from ai_notify.database import SessionTracker


TERMINAL_ESCAPE_RE = re.compile(r"\x1b(?:\][^\x07\x1b]*(?:\x07|\x1b\\)|\[[0-?]*[ -/]*[@-~]|[@-_])")
CONTROL_CHARACTER_RE = re.compile(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]")
AGENT_NOTIFICATION_RE = re.compile(
    r"\A\s*<(?:subagent_notification|task-notification)(?=[\s>])",
    re.IGNORECASE,
)


def _is_internal_agent_notification(prompt: object) -> bool:
    """Return whether a prompt is Claude's internal agent-completion envelope."""
    if not isinstance(prompt, str):
        return False

    prompt = TERMINAL_ESCAPE_RE.sub("", prompt)
    prompt = CONTROL_CHARACTER_RE.sub("", prompt)
    return bool(AGENT_NOTIFICATION_RE.match(prompt))


def handle_user_prompt(data: dict) -> None:
    """
    Handle UserPromptSubmit event.

    Args:
        data: Event data containing session_id, prompt, and cwd

    Raises:
        ValueError: If session_id is missing from data
        Exception: For other failures during prompt tracking
    """
    # Extract required fields
    session_id = data.get("session_id", "")
    prompt = data.get("prompt", "")
    cwd = data.get("cwd", "")

    if not session_id:
        raise ValueError("Missing session_id in input")

    if _is_internal_agent_notification(prompt):
        logger.debug(f"Ignoring internal agent notification for session {session_id}")
        return

    # Track prompt in database
    tracker = SessionTracker()
    tracker.track_prompt(session_id, prompt, cwd)

    logger.info(f"Tracked prompt for session {session_id}")
