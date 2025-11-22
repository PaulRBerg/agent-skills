"""
Stop event handler.
"""

from loguru import logger

from ai_notify.config import get_runtime_config
from ai_notify.database import SessionTracker
from ai_notify.helpers.cleanup import mark_cleanup_done, should_run_auto_cleanup
from ai_notify.notifier import MacNotifier
from ai_notify.utils import format_duration


def handle_stop(data: dict) -> None:
    """
    Handle Stop event.

    Marks session as stopped, sends notification if duration exceeds threshold,
    and triggers auto-cleanup if enabled.

    Args:
        data: Event data containing session_id and cwd

    Raises:
        ValueError: If session_id is missing from data
        Exception: For other failures during stop handling
    """
    # Extract required fields
    session_id = data.get("session_id", "")
    cwd = data.get("cwd", "")

    if not session_id:
        raise ValueError("Missing session_id in input")

    # Mark session as stopped
    tracker = SessionTracker()
    tracker.mark_stopped(session_id)

    # Get job info for notification
    job_number, duration_seconds = tracker.get_job_info(session_id)

    if job_number is not None and duration_seconds is not None:
        # Get runtime config for notification threshold
        runtime_config = get_runtime_config()
        threshold = runtime_config.notification.threshold_seconds

        # Smart filtering: only notify if duration meets threshold
        if duration_seconds >= threshold:
            # Send notification
            notifier = MacNotifier()
            project_name = notifier.get_project_name(cwd)
            duration_str = format_duration(duration_seconds)

            notifier.notify_job_done(project_name, job_number, duration_str)
            logger.info(f"Job #{job_number} completed in {duration_str}")
        else:
            # Skip notification for short-duration jobs
            duration_str = format_duration(duration_seconds)
            logger.debug(
                f"Skipping notification for job #{job_number} "
                f"(duration {duration_str} < threshold {threshold}s)"
            )
    else:
        logger.warning(f"No job info found for session {session_id}")

    # Auto-cleanup if enabled and enough time has passed
    runtime_config = get_runtime_config()
    if runtime_config.cleanup.auto_cleanup_enabled and should_run_auto_cleanup():
        logger.info("Running auto-cleanup...")
        stats = tracker.cleanup_old_data(
            retention_days=runtime_config.cleanup.retention_days,
            export_before=runtime_config.cleanup.export_before_cleanup,
        )
        mark_cleanup_done()
        logger.info(
            f"Auto-cleanup complete: {stats['rows_deleted']} rows deleted, "
            f"{stats['space_freed_kb']} KB freed"
        )
