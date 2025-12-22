"""
Event handler for Codex CLI notify payloads.
"""

from __future__ import annotations

import os
from typing import Any, Iterable, Optional

from loguru import logger

from ai_notify.config import get_runtime_config
from ai_notify.helpers.filters import should_send_codex_notification
from ai_notify.notifier import MacNotifier


CODEX_EVENT_TYPE = "agent-turn-complete"


def handle_codex_notify(payload: dict[str, Any]) -> None:
    """
    Handle Codex CLI notify payload.

    Args:
        payload: Parsed JSON payload from Codex CLI notify
    """
    event_type = _first_value(payload, ("type", "event"))
    if event_type and event_type != CODEX_EVENT_TYPE:
        logger.debug(f"Skipping Codex event type '{event_type}'")
        return

    runtime_config = get_runtime_config()
    prompt = _extract_prompt(payload)

    if not should_send_codex_notification(prompt, runtime_config):
        return

    cwd = payload.get("cwd") or os.getcwd()
    project_name = MacNotifier.get_project_name(str(cwd))

    message = _extract_assistant_message(payload) or prompt
    if message:
        message = _truncate_message(message, 320)

    notifier = MacNotifier()
    notifier.send_notification(
        title=project_name,
        subtitle="Codex turn complete",
        message=message,
    )


def _extract_prompt(payload: dict[str, Any]) -> str:
    messages = _first_value(
        payload,
        ("input-messages", "input_messages", "inputMessages"),
    )
    return _extract_last_user_message(messages)


def _extract_assistant_message(payload: dict[str, Any]) -> str:
    message = _first_value(
        payload,
        ("last-assistant-message", "last_assistant_message", "lastAssistantMessage"),
    )
    return _extract_message_text(message)


def _extract_last_user_message(messages: Any) -> str:
    if not messages:
        return ""

    if isinstance(messages, str):
        return messages.strip()

    if not isinstance(messages, list):
        return ""

    user_messages: list[str] = []
    all_messages: list[str] = []

    for item in messages:
        text = _extract_message_text(item)
        if not text:
            continue

        if isinstance(item, dict) and item.get("role") == "user":
            user_messages.append(text)
        else:
            all_messages.append(text)

    if user_messages:
        return user_messages[-1]
    if all_messages:
        return all_messages[-1]
    return ""


def _extract_message_text(message: Any) -> str:
    if not message:
        return ""

    if isinstance(message, str):
        return message.strip()

    if isinstance(message, dict):
        if "content" in message:
            return _extract_message_text(message["content"])
        if "text" in message:
            return _extract_message_text(message["text"])
        if "message" in message:
            return _extract_message_text(message["message"])
        return ""

    if isinstance(message, list):
        parts: list[str] = []
        for item in message:
            text = _extract_message_text(item)
            if text:
                parts.append(text)
        return " ".join(parts).strip()

    return ""


def _truncate_message(message: str, limit: int) -> str:
    normalized = " ".join(message.split())
    if len(normalized) <= limit:
        return normalized
    if limit <= 3:
        return normalized[:limit]
    return normalized[: limit - 3] + "..."


def _first_value(payload: dict[str, Any], keys: Iterable[str]) -> Optional[Any]:
    for key in keys:
        if key in payload:
            return payload[key]
    return None
