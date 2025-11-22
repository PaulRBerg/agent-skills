"""
UserPromptSubmit event handler.
"""

from loguru import logger

from ai_notify.database import SessionTracker


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

    # Track prompt in database
    tracker = SessionTracker()
    tracker.track_prompt(session_id, prompt, cwd)

    logger.info(f"Tracked prompt for session {session_id}")
