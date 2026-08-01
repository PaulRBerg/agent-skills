"""StopFailure event handler."""

from typing import Any

from loguru import logger

from ai_notify.config import get_runtime_config
from ai_notify.database import SessionTracker
from ai_notify.helpers.filters import should_send_failure_notification
from ai_notify.notifier import MacNotifier

MAX_FAILURE_MESSAGE_LENGTH = 320
DEFAULT_FAILURE_MESSAGE = "Claude Code API error"


def handle_stop_failure(data: dict[str, Any]) -> None:
    """Mark an API-failed turn as stopped and notify when completion alerts are enabled."""
    session_id = data.get("session_id", "")
    cwd = data.get("cwd", "")

    if not isinstance(session_id, str) or not session_id:
        raise ValueError("Missing session_id in input")

    tracker = SessionTracker()
    job_number = tracker.get_active_job_number(session_id)
    tracker.mark_stopped(session_id)

    runtime_config = get_runtime_config()
    if should_send_failure_notification(runtime_config):
        notifier = MacNotifier()
        project_name = notifier.get_project_name(cwd) or "Claude Code"
        notifier.notify_job_failed(
            project_name,
            _failure_message(data),
            job_number,
        )

    if job_number is None:
        logger.warning(f"No active job found for failed session {session_id}")
    else:
        logger.info(f"Job #{job_number} failed")


def _failure_message(data: dict[str, Any]) -> str:
    """Return the best documented StopFailure error text, normalized and bounded."""
    candidates = (
        data.get("last_assistant_message"),
        data.get("error_details"),
        data.get("error"),
    )
    message = next(
        (
            normalized
            for value in candidates
            if isinstance(value, str) and (normalized := " ".join(value.split()))
        ),
        DEFAULT_FAILURE_MESSAGE,
    )

    if len(message) <= MAX_FAILURE_MESSAGE_LENGTH:
        return message
    return message[: MAX_FAILURE_MESSAGE_LENGTH - 3].rstrip() + "..."
