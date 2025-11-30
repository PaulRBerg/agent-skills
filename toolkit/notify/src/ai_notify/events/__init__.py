"""
Event handlers for Claude Code hooks.
"""

from ai_notify.events.ask_user_question import handle_ask_user_question
from ai_notify.events.notification import handle_notification
from ai_notify.events.permission_request import handle_permission
from ai_notify.events.stop import handle_stop
from ai_notify.events.user_prompt_submit import handle_user_prompt

__all__ = [
    "handle_ask_user_question",
    "handle_notification",
    "handle_permission",
    "handle_stop",
    "handle_user_prompt",
]
