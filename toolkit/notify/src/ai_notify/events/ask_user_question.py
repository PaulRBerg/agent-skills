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

    # Look up the current task for context.
    task = ""
    if session_id:
        tracker = SessionTracker()
        task = tracker.get_active_prompt(session_id) or ""

    question = _question_text(tool_input)

    # Send notification
    notifier = MacNotifier()
    project_name = notifier.get_project_name(cwd)
    notifier.notify_question(
        project_name,
        task=task,
        question=question,
    )

    logger.info(f"Question notification sent: {question}")


def _question_text(tool_input: object) -> str:
    """Return the first valid question and note any remaining questions."""
    if not isinstance(tool_input, dict):
        return "Claude is asking a question"

    raw_questions = tool_input.get("questions")
    if not isinstance(raw_questions, list):
        return "Claude is asking a question"

    questions: list[str] = []
    for item in raw_questions:
        if not isinstance(item, dict):
            continue
        question = item.get("question")
        if isinstance(question, str) and question.strip():
            questions.append(question.strip())
    if not questions:
        return "Claude is asking a question"

    suffix = f" (+{len(questions) - 1} more)" if len(questions) > 1 else ""
    return questions[0] + suffix
