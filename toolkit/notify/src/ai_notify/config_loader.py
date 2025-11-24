"""
Configuration loader for ai-notify with YAML file support.
"""

import yaml
from enum import Enum
from pathlib import Path
from typing import Optional, Any, cast
from pydantic import BaseModel, Field, field_validator
from pydantic.fields import FieldInfo
from ruamel.yaml import YAML
from ruamel.yaml.comments import CommentedMap
from loguru import logger


# Default configuration paths
DEFAULT_CONFIG_DIR = Path.home() / ".config" / "ai-notify"
DEFAULT_CONFIG_PATH = DEFAULT_CONFIG_DIR / "config.yaml"
DEFAULT_EXPORT_DIR = DEFAULT_CONFIG_DIR / "exports"


class NotificationMode(str, Enum):
    """Notification modes for granular control."""

    ALL = "all"
    PERMISSION_ONLY = "permission_only"
    DISABLED = "disabled"


class NotificationConfig(BaseModel):
    """Notification-related configuration."""

    mode: NotificationMode = Field(
        default=NotificationMode.ALL,
        description="Notification mode: 'all' (default), 'permission_only', or 'disabled'",
    )
    threshold_seconds: int = Field(
        default=10,
        ge=0,
        description="Minimum job duration in seconds to trigger notification (0 = notify all)",
    )
    sound: str = Field(default="default", description="Notification sound to use")
    app_bundle: str = Field(
        default="dev.warp.Warp-Stable",
        description="Application bundle ID to focus on notification click",
    )
    exclude_patterns: list[str] = Field(
        default_factory=list,
        description="List of prompt prefixes to exclude from notifications (case-sensitive)",
    )


class DatabaseConfig(BaseModel):
    """Database-related configuration."""

    path: Path = Field(
        default=Path.home() / ".config" / "ai-notify" / "ai-notify.db",
        description="Path to SQLite database file",
    )


class CleanupConfig(BaseModel):
    """Data cleanup configuration."""

    retention_days: int = Field(
        default=30,
        ge=1,
        description="Number of days to retain session data (older data will be auto-cleaned)",
    )
    auto_cleanup_enabled: bool = Field(
        default=True, description="Enable automatic cleanup of old data"
    )
    export_before_cleanup: bool = Field(default=True, description="Export data before cleanup")


class LoggingConfig(BaseModel):
    """Logging configuration."""

    level: str = Field(
        default="INFO", description="Log level (DEBUG, INFO, WARNING, ERROR, CRITICAL)"
    )
    path: Path = Field(
        default=Path.home() / ".config" / "ai-notify" / "ai-notify.log",
        description="Path to log file",
    )

    @field_validator("level")
    @classmethod
    def validate_level(cls, v: str) -> str:
        """Validate log level is valid."""
        valid_levels = {"DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"}
        if v.upper() not in valid_levels:
            raise ValueError(f"Log level must be one of {valid_levels}")
        return v.upper()


class AINotifyConfig(BaseModel):
    """Complete ai-notify configuration."""

    notification: NotificationConfig = Field(default_factory=NotificationConfig)
    database: DatabaseConfig = Field(default_factory=DatabaseConfig)
    cleanup: CleanupConfig = Field(default_factory=CleanupConfig)
    logging: LoggingConfig = Field(default_factory=LoggingConfig)


def _get_field_description(model: type[BaseModel], field_name: str) -> Optional[str]:
    """
    Extract description from a Pydantic field.

    Args:
        model: Pydantic model class
        field_name: Name of the field

    Returns:
        Field description or None
    """
    field_info = model.model_fields.get(field_name)
    if field_info and isinstance(field_info, FieldInfo):
        return field_info.description
    return None


def _create_commented_map(data: dict[str, Any], model: type[BaseModel]) -> CommentedMap:
    """
    Create a CommentedMap with inline comments from Pydantic field descriptions.

    Args:
        data: Dictionary data to convert
        model: Pydantic model class to extract descriptions from

    Returns:
        CommentedMap with inline comments
    """
    cm = CommentedMap()

    for key, value in data.items():
        # Convert nested BaseModel instances or dicts
        if isinstance(value, dict):
            # Get the nested model type if available
            field_info = model.model_fields.get(key)
            if field_info and hasattr(field_info.annotation, "model_fields"):
                cm[key] = _create_commented_map(value, cast(type[BaseModel], field_info.annotation))
            else:
                cm[key] = CommentedMap(value)
        elif isinstance(value, list):
            cm[key] = value
        else:
            cm[key] = value

        # Add inline comment from field description
        description = _get_field_description(model, key)
        if description:
            cm.yaml_add_eol_comment(description, key)

    return cm


class ConfigLoader:
    """Loads and manages ai-notify configuration from YAML file."""

    def __init__(self, config_path: Optional[Path] = None):
        """
        Initialize config loader.

        Args:
            config_path: Path to YAML config file (default: ~/.config/ai-notify/config.yaml)
        """
        self.config_path = config_path or DEFAULT_CONFIG_PATH
        self._config: Optional[AINotifyConfig] = None

    def load(self) -> AINotifyConfig:
        """
        Load configuration from YAML file with fallback to defaults.

        Returns:
            AINotifyConfig instance
        """
        if self._config is not None:
            return self._config

        if self.config_path.exists():
            try:
                with open(self.config_path, "r") as f:
                    yaml_data = yaml.safe_load(f) or {}
                self._config = AINotifyConfig(**yaml_data)
                logger.debug(f"Loaded configuration from {self.config_path}")
            except Exception as e:
                logger.warning(f"Failed to load config from {self.config_path}: {e}")
                logger.info("Using default configuration")
                self._config = AINotifyConfig()
        else:
            logger.debug(f"No config file found at {self.config_path}, using defaults")
            self._config = AINotifyConfig()

        return self._config

    def save(self, config: Optional[AINotifyConfig] = None) -> None:
        """
        Save configuration to YAML file with helpful inline comments.

        Args:
            config: Configuration to save (default: current loaded config)
        """
        if config is None:
            if self._config is None:
                raise ValueError("No configuration loaded or provided to save")
            config = self._config

        # Ensure config directory exists
        self.config_path.parent.mkdir(parents=True, exist_ok=True)

        # Convert Pydantic model to dict for YAML serialization
        config_dict = config.model_dump(mode="python")

        # Convert Path objects and Enums to strings
        def path_to_str(obj):
            if isinstance(obj, Path):
                return str(obj)
            elif isinstance(obj, Enum):
                return obj.value
            elif isinstance(obj, dict):
                return {k: path_to_str(v) for k, v in obj.items()}
            elif isinstance(obj, list):
                return [path_to_str(v) for v in obj]
            return obj

        config_dict = path_to_str(config_dict)
        assert isinstance(config_dict, dict)  # config.model_dump() always returns dict

        # Create CommentedMap with inline comments from Pydantic field descriptions
        commented_config = _create_commented_map(config_dict, AINotifyConfig)

        # Write YAML with comments using ruamel.yaml
        yaml_writer = YAML()
        yaml_writer.default_flow_style = False
        yaml_writer.preserve_quotes = True
        yaml_writer.width = 4096  # Prevent line wrapping

        with open(self.config_path, "w") as f:
            yaml_writer.dump(commented_config, f)

        logger.info(f"Configuration saved to {self.config_path}")

    def reset_to_defaults(self) -> AINotifyConfig:
        """
        Reset configuration to defaults and save.

        Returns:
            Default AINotifyConfig instance
        """
        self._config = AINotifyConfig()
        self.save(self._config)
        logger.info("Configuration reset to defaults")
        return self._config


def get_config(config_path: Optional[Path] = None) -> AINotifyConfig:
    """
    Convenience function to load configuration.

    Args:
        config_path: Optional custom config path

    Returns:
        AINotifyConfig instance
    """
    loader = ConfigLoader(config_path)
    return loader.load()
