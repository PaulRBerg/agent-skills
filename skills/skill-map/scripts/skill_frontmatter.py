"""Parse the YAML frontmatter fields consumed by skill-map."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml


SKILL_NAME_RE = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$")
EXTERNAL_DEPENDENCY_RE = re.compile(
    r"^(?P<owner>[A-Za-z0-9](?:[A-Za-z0-9_.-]*[A-Za-z0-9])?)/"
    r"(?P<repository>[A-Za-z0-9](?:[A-Za-z0-9_.-]*[A-Za-z0-9])?)#"
    r"(?P<skill>[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?)$"
)
FRONTMATTER_RE = re.compile(r"\A---\s*\n(?P<body>.*?)\n---\s*(?:\n|\Z)", re.DOTALL)
ALL_CLIENTS = ("claude-code", "codex")


class FrontmatterError(ValueError):
    """A SKILL.md frontmatter field cannot be mapped safely."""


@dataclass(frozen=True)
class Dependency:
    identifier: str
    skill_name: str
    line: int


@dataclass(frozen=True)
class SkillFrontmatter:
    name: str | None
    clients: tuple[str, ...]
    dependencies: tuple[Dependency, ...]


def dependency_skill_name(identifier: str) -> str:
    return identifier.rsplit("#", 1)[-1]


def declared_dependency_edges(
    skills: list[Any], selected: set[str], include_self: bool
) -> list[dict[str, Any]]:
    edges: list[dict[str, Any]] = []
    for skill in skills:
        path = Path(skill.path)
        try:
            dependencies = parse_skill_frontmatter(path).dependencies
        except FrontmatterError as error:
            raise FrontmatterError(f"cannot parse {path}: {error}") from error
        for dependency in dependencies:
            if selected and skill.name not in selected and dependency.skill_name not in selected:
                continue
            if skill.name == dependency.skill_name and not include_self:
                continue
            edges.append(
                {
                    "type": "dependency",
                    "source": skill.name,
                    "target": dependency.identifier,
                    "path": skill.path,
                    "line": dependency.line,
                    "declared": True,
                }
            )
    return edges


def _read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError as error:
        raise FrontmatterError(f"cannot read {path}: {error}") from error


def _dependency_lines(frontmatter: str, dependencies: list[str], start_line: int) -> list[int]:
    lines = frontmatter.splitlines()
    field_index = next(
        (index for index, line in enumerate(lines) if re.match(r"^skill-dependencies:\s*(?:#.*)?$", line)),
        None,
    )
    if field_index is None:
        return [start_line] * len(dependencies)

    result: list[int] = []
    cursor = field_index + 1
    for identifier in dependencies:
        while cursor < len(lines):
            match = re.match(r"^\s+-\s+(.+?)\s*(?:#.*)?$", lines[cursor])
            line_number = start_line + cursor
            cursor += 1
            if match and identifier in match.group(1):
                result.append(line_number)
                break
        else:
            result.append(start_line + field_index)
    return result


def _parse_dependencies(value: Any, frontmatter: str, start_line: int) -> tuple[Dependency, ...]:
    if not isinstance(value, list) or not value:
        raise FrontmatterError("skill-dependencies must be a non-empty array when present")
    if any(not isinstance(item, str) for item in value):
        raise FrontmatterError("skill-dependencies entries must be strings")

    dependencies = list(value)
    lines = _dependency_lines(frontmatter, dependencies, start_line)
    parsed: list[Dependency] = []
    for identifier, line in zip(dependencies, lines, strict=True):
        external = EXTERNAL_DEPENDENCY_RE.fullmatch(identifier)
        if "#" in identifier or "/" in identifier:
            if external is None:
                raise FrontmatterError(
                    f"invalid skill-dependencies entry {identifier!r}; expected ORG/REPO#SKILL"
                )
            skill_name = external.group("skill")
        elif SKILL_NAME_RE.fullmatch(identifier):
            skill_name = identifier
        else:
            raise FrontmatterError(f"invalid skill-dependencies entry {identifier!r}")
        parsed.append(Dependency(identifier, skill_name, line))
    return tuple(parsed)


def parse_skill_frontmatter(path: Path) -> SkillFrontmatter:
    text = _read(path)
    match = FRONTMATTER_RE.match(text)
    if not match:
        return SkillFrontmatter(None, ALL_CLIENTS, ())

    body = match.group("body")
    try:
        document = yaml.safe_load(body) or {}
    except yaml.YAMLError as error:
        raise FrontmatterError(f"invalid YAML frontmatter: {error}") from error
    if not isinstance(document, dict):
        raise FrontmatterError("frontmatter must be a YAML mapping")

    name = document.get("name")
    parsed_name = name if isinstance(name, str) and SKILL_NAME_RE.fullmatch(name) else None
    metadata = document.get("metadata")
    install_targets = metadata.get("install-targets") if isinstance(metadata, dict) else None
    clients = (
        tuple(client for client in ALL_CLIENTS if client in install_targets.split())
        if isinstance(install_targets, str)
        else ALL_CLIENTS
    )
    dependencies = (
        _parse_dependencies(document["skill-dependencies"], body, 2)
        if "skill-dependencies" in document
        else ()
    )
    return SkillFrontmatter(parsed_name, clients or ALL_CLIENTS, dependencies)
