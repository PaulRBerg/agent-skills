"""
Event handlers for Claude Code hooks.
"""

from ai_notify.events.notification import handle_notification
from ai_notify.events.permission_request import handle_permission
from ai_notify.events.stop import handle_stop
from ai_notify.events.user_prompt_submit import handle_user_prompt

__all__ = [
    "handle_user_prompt",
    "handle_stop",
    "handle_notification",
    "handle_permission",
]
