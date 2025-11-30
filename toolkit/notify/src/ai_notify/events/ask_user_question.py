"""
AskUserQuestion event handler.
"""

from loguru import logger

from ai_notify.config import get_runtime_config
from ai_notify.database import SessionTracker
from ai_notify.helpers.filters import should_send_permission_notification
from ai_notify.notifier import MacNotifier


def handle_ask_user_question(data: dict) -> None:
    """
    Handle PreToolUse/AskUserQuestion event.

    Sends a notification when Claude asks the user a question.
    Uses same filtering as permission requests (fires unless mode is disabled).

    Args:
        data: Event data containing session_id, cwd, and tool_input

    Raises:
        Exception: For failures during question notification handling
    """
    session_id = data.get("session_id", "")
    cwd = data.get("cwd", "")
    tool_input = data.get("tool_input", {})

    # Early exit if notifications disabled
    runtime_config = get_runtime_config()
    if not should_send_permission_notification(runtime_config):
        return

    # Look up job number for this session
    job_number = None
    if session_id:
        tracker = SessionTracker()
        job_number = tracker.get_active_job_number(session_id)

    # Extract question text from tool_input.questions[0].question
    message = "Claude is asking a question"
    if isinstance(tool_input, dict):
        questions = tool_input.get("questions", [])
        if questions and isinstance(questions[0], dict):
            question_text = questions[0].get("question", message)
            # Truncate long questions at ~80 chars
            if len(question_text) > 80:
                message = question_text[:77] + "..."
            else:
                message = question_text

    # Send notification
    notifier = MacNotifier()
    project_name = notifier.get_project_name(cwd)
    notifier.notify_question(project_name, message, job_number)

    logger.info(f"Question notification sent: {message} (job #{job_number})")
