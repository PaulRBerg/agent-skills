"""
PermissionRequest event handler.
"""

from loguru import logger

from ai_notify.config import get_runtime_config
from ai_notify.database import SessionTracker
from ai_notify.helpers.filters import should_send_permission_notification
from ai_notify.notifier import MacNotifier


def handle_permission(data: dict) -> None:
    """
    Handle PermissionRequest event.

    Sends a notification for permission requests with details about the
    requested tool or command and the job number if available.

    Args:
        data: Event data containing session_id, cwd, and tool_input

    Raises:
        Exception: For failures during permission notification handling
    """
    # Extract required fields
    session_id = data.get("session_id", "")
    cwd = data.get("cwd", "")
    tool_input = data.get("tool_input", {})

    # Early exit if permission notifications disabled
    runtime_config = get_runtime_config()
    if not should_send_permission_notification(runtime_config):
        return

    # Look up job number for this session
    job_number = None
    if session_id:
        tracker = SessionTracker()
        job_number = tracker.get_active_job_number(session_id)

    # Get permission details
    if isinstance(tool_input, dict):
        # Extract tool name or command being requested
        tool_name = tool_input.get("name", "")
        command = tool_input.get("command", "")
        description = tool_input.get("description", "")

        # Build notification message
        if command:
            message = f"Command: {command}"
        elif tool_name:
            message = f"Tool: {tool_name}"
        elif description:
            message = description
        else:
            message = "Permission requested"
    else:
        message = "Permission requested"

    # Send notification
    notifier = MacNotifier()
    project_name = notifier.get_project_name(cwd)
    notifier.notify_permission_request(project_name, message, job_number)

    logger.info(f"Permission request notification sent: {message} (job #{job_number})")
