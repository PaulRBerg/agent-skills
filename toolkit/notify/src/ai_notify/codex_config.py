"""Helpers for updating Codex CLI notify settings."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import tomlkit

PROFILE_NAME_PATTERN = re.compile(r"^[A-Za-z0-9_-]+$")
_MISSING = object()


@dataclass(frozen=True)
class CodexNotifyUpdate:
    """Result of updating a Codex notify setting."""

    path: Path
    changed: bool
    profile: str | None
    conflict: bool = False
    previous_notify: Any = None


def validate_codex_profile_name(profile: str) -> str:
    """Validate a Codex profile name and return it unchanged."""
    if not PROFILE_NAME_PATTERN.fullmatch(profile):
        raise ValueError(
            "Codex profile names may contain only letters, numbers, hyphens, and underscores"
        )
    return profile


def resolve_codex_config_path(config_path: Path, profile: str | None = None) -> Path:
    """Resolve a base config path to the selected sibling profile file."""
    if profile is None:
        return config_path
    return config_path.with_name(f"{validate_codex_profile_name(profile)}.config.toml")


def set_codex_notify(
    config_path: Path,
    command: list[str],
    profile: str | None = None,
    force: bool = False,
) -> CodexNotifyUpdate:
    """Set Codex's root notify command, refusing a different command unless forced."""
    target_path = resolve_codex_config_path(config_path, profile)
    text = target_path.read_text() if target_path.exists() else ""
    document = _parse_toml(text, target_path)
    previous_notify = _root_notify(document)

    if previous_notify is not _MISSING:
        if previous_notify == command:
            return CodexNotifyUpdate(
                path=target_path,
                changed=False,
                profile=profile,
                previous_notify=previous_notify,
            )
        if not force:
            return CodexNotifyUpdate(
                path=target_path,
                changed=False,
                profile=profile,
                conflict=True,
                previous_notify=previous_notify,
            )

    document["notify"] = command
    updated_text = tomlkit.dumps(document)
    target_path.parent.mkdir(parents=True, exist_ok=True)
    target_path.write_text(updated_text)

    return CodexNotifyUpdate(
        path=target_path,
        changed=True,
        profile=profile,
        conflict=previous_notify is not _MISSING,
        previous_notify=None if previous_notify is _MISSING else previous_notify,
    )


def _parse_toml(text: str, path: Path) -> tomlkit.TOMLDocument:
    try:
        return tomlkit.parse(text) if text.strip() else tomlkit.document()
    except Exception as exc:  # noqa: BLE001
        raise ValueError(f"Failed to parse {path}: {exc}") from exc


def _root_notify(document: tomlkit.TOMLDocument) -> Any:
    if "notify" not in document:
        return _MISSING
    return document["notify"].unwrap()
