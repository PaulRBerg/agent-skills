"""
Notification filtering logic.
"""

from loguru import logger
from ai_notify.config_loader import AINotifyConfig


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
