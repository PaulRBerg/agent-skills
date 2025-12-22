"""
Helpers for updating Codex CLI config.toml notify settings.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class CodexNotifyUpdate:
    path: Path
    changed: bool
    profile: str | None


def set_codex_notify(
    config_path: Path,
    command: list[str],
    profile: str | None = None,
) -> CodexNotifyUpdate:
    """
    Set Codex CLI notify command in config.toml.

    Args:
        config_path: Path to Codex config.toml
        command: Notify command array (e.g. ["ai-notify", "codex"])
        profile: Optional profile name (e.g. "quiet")

    Returns:
        CodexNotifyUpdate indicating whether changes were made.
    """
    text = ""
    if config_path.exists():
        text = config_path.read_text()

    updated_text, changed = _update_notify_in_toml(text, command, profile)

    if changed:
        config_path.parent.mkdir(parents=True, exist_ok=True)
        config_path.write_text(updated_text)

    return CodexNotifyUpdate(path=config_path, changed=changed, profile=profile)


def _update_notify_in_toml(
    text: str,
    command: list[str],
    profile: str | None,
) -> tuple[str, bool]:
    if not text.strip():
        return _format_notify_block("", command), True

    lines = text.splitlines(keepends=True)
    target_section = f"profiles.{profile}" if profile else None

    current_section: str | None = None
    found_section = False
    section_header_index: int | None = None
    first_table_index: int | None = None

    i = 0
    while i < len(lines):
        stripped = lines[i].strip()
        if _is_table_header(stripped):
            section_name = stripped[1:-1].strip()
            current_section = section_name
            if first_table_index is None:
                first_table_index = i
            if target_section and section_name == target_section:
                found_section = True
                section_header_index = i
            i += 1
            continue

        if _is_notify_line(stripped) and _in_target_section(current_section, target_section):
            block_start = i
            block_end = _find_block_end(lines, i)
            indent = _line_indent(lines[block_start])
            new_block = _format_notify_block(indent, command)

            if lines[block_start:block_end] == [new_block]:
                return text, False

            lines[block_start:block_end] = [new_block]
            return "".join(lines), True

        i += 1

    new_block = _format_notify_block(
        _section_indent(lines, section_header_index) if target_section else "",
        command,
    )

    if target_section:
        if found_section and section_header_index is not None:
            insert_at = _find_insert_after_header(lines, section_header_index)
            lines.insert(insert_at, new_block)
            return "".join(lines), True

        return _append_section(lines, target_section, new_block)

    insert_at = first_table_index if first_table_index is not None else len(lines)
    lines.insert(insert_at, new_block)
    return "".join(lines), True


def _format_notify_block(indent: str, command: Iterable[str]) -> str:
    items = ", ".join(_toml_string(item) for item in command)
    return f"{indent}notify = [{items}]\n"


def _toml_string(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def _is_notify_line(stripped: str) -> bool:
    return stripped.startswith("notify") and "=" in stripped


def _is_table_header(stripped: str) -> bool:
    return stripped.startswith("[") and stripped.endswith("]") and not stripped.startswith("[[")


def _in_target_section(current: str | None, target: str | None) -> bool:
    if target is None:
        return current is None
    return current == target


def _find_block_end(lines: list[str], start: int) -> int:
    stripped = lines[start].strip()
    if "[" in stripped and "]" not in stripped:
        i = start + 1
        while i < len(lines):
            if "]" in lines[i]:
                return i + 1
            i += 1
        return len(lines)
    return start + 1


def _line_indent(line: str) -> str:
    return line[: len(line) - len(line.lstrip())]


def _section_indent(lines: list[str], header_index: int | None) -> str:
    if header_index is None:
        return "  "
    i = header_index + 1
    while i < len(lines):
        stripped = lines[i].strip()
        if stripped.startswith("["):
            break
        if stripped and not stripped.startswith("#"):
            return _line_indent(lines[i])
        i += 1
    return "  "


def _find_insert_after_header(lines: list[str], header_index: int) -> int:
    i = header_index + 1
    while i < len(lines):
        stripped = lines[i].strip()
        if stripped.startswith("["):
            return i
        if stripped and not stripped.startswith("#"):
            return i
        i += 1
    return i


def _append_section(lines: list[str], section_name: str, new_block: str) -> tuple[str, bool]:
    if lines and not lines[-1].endswith("\n"):
        lines[-1] = f"{lines[-1]}\n"

    if lines and lines[-1].strip():
        lines.append("\n")

    lines.append(f"[{section_name}]\n")
    lines.append(new_block)
    return "".join(lines), True
