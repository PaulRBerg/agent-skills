"""StopFailure event handler."""

from typing import Any

from loguru import logger

from ai_notify.config import get_runtime_config
from ai_notify.database import SessionTracker
from ai_notify.helpers.filters import should_send_failure_notification
from ai_notify.notifier import MacNotifier
from ai_notify.utils import format_duration

DEFAULT_FAILURE_MESSAGE = "Claude Code API error"


def handle_stop_failure(data: dict[str, Any]) -> None:
    """Mark an API-failed turn as stopped and notify when completion alerts are enabled."""
    session_id = data.get("session_id", "")
    cwd = data.get("cwd", "")

    if not isinstance(session_id, str) or not session_id:
        raise ValueError("Missing session_id in input")

    tracker = SessionTracker()
    prompt = tracker.get_active_prompt(session_id)
    tracker.mark_stopped(session_id)

    job_number = None
    duration_seconds = None
    if prompt is not None:
        job_number, duration_seconds, _ = tracker.get_job_info(session_id)

    runtime_config = get_runtime_config()
    if should_send_failure_notification(runtime_config):
        notifier = MacNotifier()
        project_name = notifier.get_project_name(cwd) or "Claude Code"
        notifier.notify_job_failed(
            project_name,
            task=prompt or "",
            error=_failure_message(data),
            duration_str=(
                format_duration(duration_seconds) if duration_seconds is not None else None
            ),
        )

    if job_number is None:
        logger.warning(f"No active job found for failed session {session_id}")
    else:
        logger.info(f"Job #{job_number} failed")


def _failure_message(data: dict[str, Any]) -> str:
    """Return the best documented StopFailure error text, normalized."""
    candidates = (
        data.get("last_assistant_message"),
        data.get("error_details"),
        data.get("error"),
    )
    return next(
        (
            normalized
            for value in candidates
            if isinstance(value, str) and (normalized := " ".join(value.split()))
        ),
        DEFAULT_FAILURE_MESSAGE,
    )
