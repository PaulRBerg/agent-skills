"""
Notification event handler.
"""

from loguru import logger

from ai_notify.database import SessionTracker


def handle_notification(data: dict) -> None:
    """
    Handle Notification event.

    Suppresses "waiting for input" notifications and tracks waiting state.
    Other notifications are logged but not sent to the user.

    Args:
        data: Event data containing session_id and message

    Raises:
        ValueError: If session_id is missing from data
        Exception: For other failures during notification handling
    """
    # Extract required fields
    session_id = data.get("session_id", "")
    message = data.get("message", "")

    if not session_id:
        raise ValueError("Missing session_id in input")

    # Check if this is a "waiting for input" notification
    waiting_keywords = ["waiting for input", "waiting for user", "approval needed"]
    is_waiting = any(keyword in message.lower() for keyword in waiting_keywords)

    if is_waiting:
        # Track waiting state but don't send notification
        # The Stop handler will send the job completion notification
        tracker = SessionTracker()
        tracker.mark_waiting(session_id)
        logger.debug(f"Suppressed waiting notification for session {session_id}")
    else:
        # For other notifications, just log them
        logger.info(f"Notification: {message}")
