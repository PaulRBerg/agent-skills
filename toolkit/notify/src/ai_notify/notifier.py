"""
Notification layer using terminal-notifier for macOS notifications.
"""

import platform
import shutil
import subprocess
from pathlib import Path
from typing import Optional

from loguru import logger

from ai_notify.config import Config, get_runtime_config


class MacNotifier:
    """Sends desktop notifications using terminal-notifier (macOS only)."""

    def __init__(self, config: Optional[Config] = None):
        """
        Initialize notifier.

        Args:
            config: Configuration instance (creates default if None)
        """
        self.config = config or Config()
        self._available: Optional[bool] = None
        self._icon_path: Optional[Path] = None

    def _get_icon_path(self) -> Optional[Path]:
        """
        Get path to bundled Claude icon.

        Returns:
            Path to icon file, or None if not found
        """
        if self._icon_path is not None:
            return self._icon_path

        icon_path = Path(__file__).parent / "assets" / "claude-icon.png"
        if icon_path.exists():
            self._icon_path = icon_path
            return icon_path

        logger.warning(f"Claude icon not found at {icon_path}")
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
            # Build notification message
            full_message = f"{subtitle}\n{message}" if message else subtitle

            # Get runtime config for notification settings
            runtime_config = get_runtime_config()

            # Build terminal-notifier command
            cmd = [
                "terminal-notifier",
                "-title",
                title,
                "-message",
                full_message,
                "-activate",
                runtime_config.notification.app_bundle,
                "-sound",
                runtime_config.notification.sound,
            ]

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

    def notify_job_done(
        self,
        project_name: str,
        job_number: int,
        duration_str: str,
    ) -> bool:
        """
        Send job completion notification.

        Args:
            project_name: Project name (from cwd)
            job_number: Job sequence number
            duration_str: Human-readable duration (e.g., "53s", "6m53s")

        Returns:
            True if notification was sent successfully
        """
        subtitle = f"Prompt #{job_number} completed in {duration_str}"
        return self.send_notification(
            title=project_name,
            subtitle=subtitle,
        )

    def notify_permission_request(
        self,
        project_name: str,
        message: str = "Claude is waiting for permission",
    ) -> bool:
        """
        Send permission request notification.

        Args:
            project_name: Project name (from cwd)
            message: Permission request message

        Returns:
            True if notification was sent successfully
        """
        return self.send_notification(
            title=project_name,
            subtitle="Approval needed",
            message=message,
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
