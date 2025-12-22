"""
Notification filtering logic.
"""

from loguru import logger
from ai_notify.config_loader import AINotifyConfig, NotificationMode


def should_send_notification(
    prompt: str,
    duration_seconds: int,
    config: AINotifyConfig,
) -> bool:
    """
    Determine whether to send a notification based on filters.

    Args:
        prompt: User prompt text
        duration_seconds: Job duration in seconds
        config: Runtime configuration

    Returns:
        True if notification should be sent, False otherwise
    """
    # Check duration threshold
    threshold = config.notification.threshold_seconds
    if duration_seconds < threshold:
        logger.debug(
            f"Skipping notification: duration {duration_seconds}s < threshold {threshold}s"
        )
        return False

    # Check exclude patterns (case-sensitive prefix matching)
    exclude_patterns = config.notification.exclude_patterns
    if exclude_patterns and prompt:
        for pattern in exclude_patterns:
            if prompt.startswith(pattern):
                logger.debug(
                    f"Skipping notification: prompt starts with excluded pattern '{pattern}'"
                )
                return False

    return True


def should_send_completion_notification(
    prompt: str,
    duration_seconds: int,
    config: AINotifyConfig,
) -> bool:
    """
    Determine whether to send a completion notification based on mode and filters.

    Args:
        prompt: User prompt text
        duration_seconds: Job duration in seconds
        config: Runtime configuration

    Returns:
        True if notification should be sent, False otherwise
    """
    # Check mode first (fast path)
    if config.notification.mode == NotificationMode.DISABLED:
        return False

    if config.notification.mode == NotificationMode.PERMISSION_ONLY:
        return False

    # Mode is ALL - apply existing filters
    return should_send_notification(prompt, duration_seconds, config)


def should_send_permission_notification(config: AINotifyConfig) -> bool:
    """
    Determine whether to send permission request notifications.

    Args:
        config: Runtime configuration

    Returns:
        True if permission notifications should be sent, False otherwise
    """
    return config.notification.mode != NotificationMode.DISABLED


def should_send_codex_notification(prompt: str, config: AINotifyConfig) -> bool:
    """
    Determine whether to send a Codex CLI notification.

    Codex notify payloads do not include job duration, so we only apply mode
    and exclude pattern filtering.

    Args:
        prompt: User prompt text
        config: Runtime configuration

    Returns:
        True if notification should be sent, False otherwise
    """
    if config.notification.mode == NotificationMode.DISABLED:
        return False

    if config.notification.mode == NotificationMode.PERMISSION_ONLY:
        return False

    exclude_patterns = config.notification.exclude_patterns
    if exclude_patterns and prompt:
        for pattern in exclude_patterns:
            if prompt.startswith(pattern):
                logger.debug(
                    f"Skipping Codex notification: prompt starts with excluded pattern '{pattern}'"
                )
                return False

    return True
