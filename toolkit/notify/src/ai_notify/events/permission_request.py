"""
PermissionRequest event handler.
"""

from typing import Any

from loguru import logger

from ai_notify.config import get_runtime_config
from ai_notify.database import SessionTracker
from ai_notify.helpers.filters import should_send_permission_notification
from ai_notify.notifier import MacNotifier

PERMISSION_DETAIL_KEYS = (
    "command",
    "file_path",
    "notebook_path",
    "path",
    "url",
    "query",
    "description",
)


def handle_permission(data: dict) -> None:
    """
    Handle PermissionRequest event.

    Sends a notification for permission requests with details about the
    current task and requested tool action.

    Args:
        data: Event data containing session_id, cwd, tool_name, and tool_input

    Raises:
        Exception: For failures during permission notification handling
    """
    # Extract required fields
    session_id = data.get("session_id", "")
    cwd = data.get("cwd", "")
    tool_name = data.get("tool_name", "")  # top-level in current Claude Code payloads
    tool_input = data.get("tool_input", {})

    # Early exit if permission notifications disabled
    runtime_config = get_runtime_config()
    if not should_send_permission_notification(runtime_config):
        return

    # Look up the current task for context.
    task = ""
    if session_id:
        tracker = SessionTracker()
        task = tracker.get_active_prompt(session_id) or ""

    request = _permission_request(tool_name, tool_input)

    # Send notification
    notifier = MacNotifier()
    project_name = notifier.get_project_name(cwd)
    notifier.notify_permission_request(
        project_name,
        task=task,
        request=request,
    )

    logger.info(f"Permission request notification sent: {request}")


def _permission_request(tool_name: Any, tool_input: Any) -> str:
    """Return the most actionable permission context for later truncation."""
    normalized_tool = tool_name.strip() if isinstance(tool_name, str) else ""
    detail = ""

    if isinstance(tool_input, dict):
        nested_name = tool_input.get("name")
        if not normalized_tool and isinstance(nested_name, str):
            normalized_tool = nested_name.strip()

        for key in PERMISSION_DETAIL_KEYS:
            value = tool_input.get(key)
            if isinstance(value, str) and value.strip():
                detail = value.strip()
                break

    if normalized_tool and detail:
        return f"{normalized_tool} — {detail}"
    if normalized_tool:
        return normalized_tool
    return detail or "Permission requested"
