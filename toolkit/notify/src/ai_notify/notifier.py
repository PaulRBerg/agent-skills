"""
Notification layer using terminal-notifier for macOS notifications.
"""

import os
import platform
import shlex
import shutil
import subprocess
from pathlib import Path
from typing import Optional

from loguru import logger

from ai_notify.config import Config, get_runtime_config

TASK_EXCERPT_LENGTH = 100
DETAIL_EXCERPT_LENGTH = 180


def _build_focus_command(app_bundle: str) -> Optional[str]:
    """
    Build a shell command that focuses the exact iTerm2 session that produced the
    notification, falling back to activating the app if the focus fails.

    Uses the `ITERM_SESSION_ID` environment variable (format `w0t0p0:UUID`) captured
    at notification time, and the `it2` CLI (iTerm2 Python API client) to focus that
    session by ID. Returns None when the session can't be identified or `it2` isn't
    available, so the caller can fall back to plain app activation.

    Args:
        app_bundle: Bundle ID to activate if focusing the specific session fails

    Returns:
        A shell command string suitable for terminal-notifier's -execute, or None
    """
    try:
        session_id_raw = os.environ.get("ITERM_SESSION_ID")
        if not session_id_raw:
            return None

        it2_path = shutil.which("it2")
        if not it2_path:
            logger.debug("it2 CLI not found; falling back to plain app activation")
            return None

        # ITERM_SESSION_ID looks like "w0t0p0:UUID"; it2 expects the bare UUID.
        session_id = session_id_raw.split(":", 1)[-1]

        focus_cmd = f"{shlex.quote(it2_path)} session focus {shlex.quote(session_id)}"
        activate_cmd = f"open -b {shlex.quote(app_bundle)}"
        return f"{focus_cmd} || {activate_cmd}"
    except Exception as e:
        logger.debug(f"Failed to build iTerm2 session focus command: {e}")
        return None


def _truncate_text(value: str, limit: int) -> str:
    """Normalize whitespace and bound notification text."""
    normalized = " ".join(value.split())
    if len(normalized) <= limit:
        return normalized
    return normalized[: limit - 3].rstrip() + "..."


def _format_context_message(task: str, detail_label: str, detail: str) -> str:
    """Build a compact task/detail notification body."""
    lines: list[str] = []
    task_excerpt = _truncate_text(task, TASK_EXCERPT_LENGTH)
    detail_excerpt = _truncate_text(detail, DETAIL_EXCERPT_LENGTH)

    if task_excerpt:
        lines.append(f"Task: {task_excerpt}")
    if detail_excerpt:
        lines.append(f"{detail_label}: {detail_excerpt}")
    return "\n".join(lines)


class MacNotifier:
    """Sends desktop notifications using terminal-notifier (macOS only)."""

    def __init__(self, config: Optional[Config] = None, icon_name: str = "claude"):
        """
        Initialize notifier.

        Args:
            config: Configuration instance (creates default if None)
            icon_name: Bundled notification icon name
        """
        self.config = config or Config()
        self._icon_name = icon_name
        self._available: Optional[bool] = None
        self._icon_path: Optional[Path] = None

    def _get_icon_path(self) -> Optional[Path]:
        """
        Get path to the configured bundled notification icon.

        Returns:
            Path to icon file, or None if not found
        """
        if self._icon_path is not None:
            return self._icon_path

        icon_path = Path(__file__).parent / "assets" / f"{self._icon_name}-icon.png"
        if icon_path.exists():
            self._icon_path = icon_path
            return icon_path

        logger.warning(f"Notification icon not found at {icon_path}")
        return None

    def check_available(self) -> bool:
        """
        Check if notifications are available on this platform.

        Returns:
            True if terminal-notifier is installed and platform is macOS
        """
        if self._available is not None:
            return self._available

        # Check platform
        if platform.system() != "Darwin":
            logger.warning("Notifications require macOS")
            self._available = False
            return False

        # Check for terminal-notifier binary
        if shutil.which("terminal-notifier") is None:
            logger.warning(
                "terminal-notifier not found. Install with: brew install terminal-notifier"
            )
            self._available = False
            return False

        self._available = True
        return True

    def send_notification(
        self,
        title: str,
        subtitle: str,
        message: str = "",
    ) -> bool:
        """
        Send a desktop notification using terminal-notifier.

        Args:
            title: Notification title (e.g., project name)
            subtitle: Notification subtitle (e.g., event details)
            message: Optional notification message body

        Returns:
            True if notification was sent successfully
        """
        if not self.check_available():
            logger.debug("Skipping notification (not available on this platform)")
            return False

        try:
            # Get runtime config for notification settings
            runtime_config = get_runtime_config()

            # Build terminal-notifier command
            cmd = [
                "terminal-notifier",
                "-title",
                title,
            ]
            if message:
                cmd.extend(["-subtitle", subtitle, "-message", message])
            else:
                # terminal-notifier requires a message; retain the existing
                # two-argument behavior without duplicating the subtitle.
                cmd.extend(["-message", subtitle])
            cmd.append("-ignoreDnD")

            focus_command = _build_focus_command(runtime_config.notification.app_bundle)
            if focus_command:
                cmd.extend(["-execute", focus_command])
            else:
                cmd.extend(["-activate", runtime_config.notification.app_bundle])

            cmd.extend(["-sound", runtime_config.notification.sound])

            # Add icon as content image if available (contentImage instead of appIcon due to macOS API restrictions)
            icon_path = self._get_icon_path()
            if icon_path:
                cmd.extend(["-contentImage", str(icon_path)])

            # Execute command
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False,
            )

            if result.returncode != 0:
                logger.error(f"terminal-notifier failed: {result.stderr or result.stdout}")
                return False

            logger.info(f"Sent notification: {title} - {subtitle}")
            return True

        except Exception as e:
            logger.error(f"Failed to send notification: {e}")
            self._available = False
            return False

    def notify_completion(
        self,
        project_name: str,
        *,
        agent: str,
        task: str,
        result: str,
        duration_str: Optional[str] = None,
    ) -> bool:
        """Send a contextual turn-completion notification."""
        subtitle = f"{agent} completed"
        if duration_str:
            subtitle += f" in {duration_str}"

        message = _format_context_message(task, "Result", result)
        if not message:
            message = "Result: Turn completed."

        return self.send_notification(
            title=project_name or agent,
            subtitle=subtitle,
            message=message,
        )

    def notify_permission_request(
        self,
        project_name: str,
        *,
        task: str,
        request: str,
    ) -> bool:
        """Send a contextual permission-request notification."""
        return self.send_notification(
            title=project_name or "Claude Code",
            subtitle="Claude needs approval",
            message=_format_context_message(
                task,
                "Request",
                request or "Permission requested",
            ),
        )

    def notify_job_failed(
        self,
        project_name: str,
        *,
        task: str,
        error: str,
        duration_str: Optional[str] = None,
    ) -> bool:
        """Send a contextual failed-turn notification."""
        subtitle = "Claude failed"
        if duration_str:
            subtitle += f" after {duration_str}"

        return self.send_notification(
            title=project_name or "Claude Code",
            subtitle=subtitle,
            message=_format_context_message(task, "Error", error),
        )

    def notify_question(
        self,
        project_name: str,
        *,
        task: str,
        question: str,
    ) -> bool:
        """Send a contextual question notification."""
        return self.send_notification(
            title=project_name or "Claude Code",
            subtitle="Claude needs input",
            message=_format_context_message(
                task,
                "Question",
                question or "Claude is asking a question",
            ),
        )

    @staticmethod
    def get_project_name(cwd: str) -> str:
        """
        Extract project name from current working directory.

        Args:
            cwd: Current working directory path

        Returns:
            Project name (basename of directory)
        """
        return Path(cwd).name
