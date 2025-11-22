"""
PermissionRequest event handler.
"""

from loguru import logger

from ai_notify.notifier import MacNotifier


def handle_permission(data: dict) -> None:
    """
    Handle PermissionRequest event.

    Sends a notification for permission requests with details about the
    requested tool or command.

    Args:
        data: Event data containing cwd and tool_input

    Raises:
        Exception: For failures during permission notification handling
    """
    # Extract required fields
    cwd = data.get("cwd", "")
    tool_input = data.get("tool_input", {})

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
    notifier.notify_permission_request(project_name, message)

    logger.info(f"Permission request notification sent: {message}")
