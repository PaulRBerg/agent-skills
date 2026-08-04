#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Map agent skill installs, dependencies, and references.

Usage:
  uv run scripts/skill-map.py [--root PATH ...] [--skill NAME ...]
      [--format text|json|dot] [--include-catalog-sources]
      [--include-self] [--include-snippets] [--show-skipped]
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from collections import Counter, defaultdict
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Any, BinaryIO


DEFAULT_DIR_IGNORES = (
    ".git",
    "node_modules",
    "vendor",
    ".venv",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    "coverage",
)

AGENT_STATE_DIR_GLOBS = (
    "**/.claude/projects/**",
    "**/.claude/plans/**",
    "**/.claude/file-history/**",
    "**/.claude/tasks/**",
    "**/.claude/debug/**",
    "**/.claude/backups/**",
    "**/.claude/paste-cache/**",
    "**/.claude/image-cache/**",
    "**/.claude/session-env/**",
    "**/.claude/shell-snapshots/**",
    "**/.claude/todos/**",
    "**/.claude/statsig/**",
    "**/.claude/logs/**",
    "**/.codex/sessions/**",
    "**/.codex/archived_sessions/**",
    "**/.codex/threads/**",
    "**/.codex/backups/**",
    "**/.codex/log/**",
    "**/.codex/logs/**",
    "**/.codex/cache/**",
    "**/.codex/tmp/**",
    "**/.codex/.tmp/**",
    "**/.codex/shell_snapshots/**",
    "**/.codex/generated_images/**",
    "**/.codex/sqlite/**",
)

AGENT_STATE_FILE_GLOBS = (
    "**/.claude/history.jsonl",
    "**/.claude/stats-cache.json",
    "**/.claude/remote-settings.json",
    "**/.codex/history.jsonl",
    "**/.codex/session_index.jsonl",
    "**/.codex/*.sqlite",
    "**/.codex/*.sqlite-shm",
    "**/.codex/*.sqlite-wal",
    "**/.codex/*.bak",
)

CATALOG_SOURCE_PATHS = (
    "~/projects/agent-skills",
    "~/sablier/sablier-skills",
    "~/sablier/agent-skills",
)

MACOS_PROTECTED_HOME_PATHS = (
    "~/Library",
    "~/.Trash",
)

ALWAYS_IGNORED_HOME_PATHS = (
    "~/.agents",
    "~/.claude",
    "~/.codex",
    "~/.local/state/skills",
)

BROAD_SCAN_CACHE_PATHS = (
    "~/.cache",
    "~/.npm",
    "~/.rustup",
    "~/.cargo/git",
    "~/.cargo/registry",
    "~/.bun/install/cache",
    "~/.pnpm-store",
    "~/.local/share/uv",
    "~/.local/share/rustup",
    "~/.local/share/cargo/git",
    "~/.local/share/cargo/registry",
    "~/.local/share/bun/install/cache",
    "~/.local/share/pnpm/store",
    "~/go/pkg/mod",
)

SKILL_NAME_RE = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$")
FRONTMATTER_RE = re.compile(r"\A---\s*\n(.*?)\n---\s*(?:\n|\Z)", re.DOTALL)
NAME_FIELD_RE = re.compile(r"^name:\s*['\"]?([^'\"\n#]+?)['\"]?\s*(?:#.*)?$", re.MULTILINE)
UNRESOLVED_TOKEN_RE = re.compile(
    r"(?<![\w./-])\$(?P<dollar>[a-z0-9]+(?:-[a-z0-9]+)+)\b"
    r"|(?:^|[\s`'\"(])/(?P<slash>[a-z0-9]+(?:-[a-z0-9]+)+)\b"
)


@dataclass(frozen=True)
class Skill:
    name: str
    path: str
    realpath: str
    directory: str
    real_directory: str
    scope: str


def fail(message: str, code: int = 2) -> None:
    print(f"skill-map: {message}", file=sys.stderr)
    raise SystemExit(code)


def run_rg(args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)


def rg_base_args(root: Path, include_catalog_sources: bool) -> list[str]:
    if shutil.which("rg") is None:
        fail("ripgrep (rg) is required")
    args = ["rg", "--hidden", "--no-messages"]
    for name in DEFAULT_DIR_IGNORES:
        args.extend(["-g", f"!**/{name}/**"])
    for pattern in AGENT_STATE_DIR_GLOBS + AGENT_STATE_FILE_GLOBS:
        args.extend(["-g", f"!{pattern}"])
    for protected_root in macos_protected_home_roots():
        if protected_root == root:
            continue
        if protected_root.is_relative_to(root):
            relative = protected_root.relative_to(root).as_posix()
            args.extend(["-g", f"!{relative}/**"])
    for ignored_root in always_ignored_home_roots():
        if ignored_root == root:
            continue
        if ignored_root.is_relative_to(root):
            relative = ignored_root.relative_to(root).as_posix()
            args.extend(["-g", f"!{relative}/**"])
    for cache_root in broad_scan_cache_roots():
        if cache_root == root:
            continue
        if cache_root.is_relative_to(root):
            relative = cache_root.relative_to(root).as_posix()
            args.extend(["-g", f"!{relative}/**"])
    if not include_catalog_sources:
        for catalog_root in catalog_source_roots():
            if root.is_relative_to(catalog_root):
                continue
            if catalog_root.is_relative_to(root):
                relative = catalog_root.relative_to(root).as_posix()
                args.extend(["-g", f"!{relative}/**"])
    return args


def normalize_roots(raw_roots: list[str]) -> list[Path]:
    roots = [Path(os.path.expanduser(root)).resolve() for root in raw_roots]
    existing = []
    for root in roots:
        if not root.exists():
            fail(f"root does not exist: {root}")
        if not root.is_dir():
            fail(f"root is not a directory: {root}")
        existing.append(root)
    return existing


def catalog_source_roots() -> list[Path]:
    return [Path(os.path.expanduser(path)).resolve() for path in CATALOG_SOURCE_PATHS]


def macos_protected_home_roots() -> list[Path]:
    return [Path(os.path.expanduser(path)).resolve() for path in MACOS_PROTECTED_HOME_PATHS]


def always_ignored_home_roots() -> list[Path]:
    return [Path(os.path.expanduser(path)).resolve() for path in ALWAYS_IGNORED_HOME_PATHS]


def broad_scan_cache_roots() -> list[Path]:
    return [Path(os.path.expanduser(path)).resolve() for path in BROAD_SCAN_CACHE_PATHS]


def catalog_sources_enabled_for_path(path: Path, roots: list[Path], include_catalog_sources: bool) -> bool:
    if include_catalog_sources:
        return True
    for catalog_root in catalog_source_roots():
        if not path.is_relative_to(catalog_root):
            continue
        return any(root.is_relative_to(catalog_root) for root in roots)
    return True


def path_is_scannable(path: Path, roots: list[Path], include_catalog_sources: bool) -> bool:
    return catalog_sources_enabled_for_path(path.resolve(), roots, include_catalog_sources)


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def parse_skill_name(path: Path) -> str:
    text = read_text(path)
    match = FRONTMATTER_RE.match(text)
    if match:
        name_match = NAME_FIELD_RE.search(match.group(1))
        if name_match:
            value = name_match.group(1).strip()
            if SKILL_NAME_RE.match(value):
                return value
    return path.parent.name


def classify_scope(path: Path) -> str:
    parts = path.parts
    joined = "/".join(parts)
    if "/.agents/skills/" in joined:
        return "agents"
    if "/.claude/skills/" in joined:
        return "claude"
    if "/.codex/skills/" in joined:
        return "codex"
    if "/skills/" in joined:
        return "catalog"
    return "unknown"


def discover_skills(roots: list[Path], include_catalog_sources: bool) -> list[Skill]:
    skills: list[Skill] = []
    seen_paths: set[str] = set()
    for root in roots:
        cmd = rg_base_args(root, include_catalog_sources) + ["--files", "-g", "SKILL.md", "."]
        result = run_rg(cmd, cwd=root)
        if result.returncode not in (0, 1):
            fail(f"rg skill discovery failed for {root}:\n{result.stderr.strip()}")

        for line in result.stdout.splitlines():
            path = (root / line).resolve()
            if not path_is_scannable(path, roots, include_catalog_sources):
                continue
            if str(path) in seen_paths or path.name != "SKILL.md":
                continue
            seen_paths.add(str(path))
            name = parse_skill_name(path)
            if not SKILL_NAME_RE.match(name):
                continue
            skills.append(
                Skill(
                    name=name,
                    path=str(path),
                    realpath=str(path.resolve()),
                    directory=str(path.parent),
                    real_directory=str(path.parent.resolve()),
                    scope=classify_scope(path),
                )
            )
    return sorted(skills, key=lambda skill: (skill.name, skill.path))


def skill_for_file(path: str, skills: list[Skill]) -> Skill | None:
    real = str(Path(path).resolve())
    candidates = [skill for skill in skills if real == skill.realpath or real.startswith(skill.real_directory + os.sep)]
    if not candidates:
        return None
    return max(candidates, key=lambda skill: len(skill.real_directory))


def build_known_pattern(names: list[str]) -> str | None:
    if not names:
        return None
    alt = "|".join(re.escape(name) for name in sorted(names, key=len, reverse=True))
    return (
        rf"(?<![\w.-])[$/](?P<direct>{alt})\b"
        rf"|(?:^|[`\s'\"(])(?P<prose>{alt})\s+skill\b"
        rf"|(?P<path>(?:\.agents|\.claude|\.codex)?/skills/(?P<path_name>{alt})\b)"
        rf"|(?P<skill_path>skills/(?P<catalog_name>{alt})\b)"
        rf"|(?P<sibling>\.\./(?P<sibling_name>{alt})/SKILL\.md)"
    )


def build_unresolved_pattern(names: set[str]) -> str | None:
    kebab_names = sorted((name for name in names if "-" in name), key=len, reverse=True)
    if not kebab_names:
        return None
    alt = "|".join(re.escape(name) for name in kebab_names)
    return (
        rf"(?<![\w./-])\$(?P<dollar>{alt})\b"
        rf"|(?:^|[\s`'\"(])/(?P<slash>{alt})\b"
    )


def parse_rg_matches(stream: BinaryIO) -> Iterator[tuple[str, int, str]]:
    """Parse `rg --null --line-number --only-matching` output without buffering it all."""
    buffer = bytearray()
    path: bytes | None = None

    while chunk := stream.read(64 * 1024):
        buffer.extend(chunk)
        while True:
            if path is None:
                separator = buffer.find(b"\0")
                if separator < 0:
                    break
                path = bytes(buffer[:separator])
                del buffer[: separator + 1]

            separator = buffer.find(b"\n")
            if separator < 0:
                break
            payload = bytes(buffer[:separator])
            del buffer[: separator + 1]

            line_number, found, match = payload.partition(b":")
            if not found or not line_number.isdigit():
                fail("malformed ripgrep reference output")
            yield os.fsdecode(path), int(line_number), match.decode("utf-8", errors="replace")
            path = None

    if path is not None or buffer:
        fail("truncated ripgrep reference output")


def search_pattern(roots: list[Path], pattern: str, include_catalog_sources: bool) -> Iterator[dict[str, Any]]:
    for root in roots:
        cmd = rg_base_args(root, include_catalog_sources) + [
            "--line-number",
            "--with-filename",
            "--null",
            "--only-matching",
            "--pcre2",
            "-e",
            pattern,
            ".",
        ]
        process = subprocess.Popen(cmd, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        assert process.stdout is not None
        assert process.stderr is not None
        try:
            for relative_path, line_number, match in parse_rg_matches(process.stdout):
                path = (root / relative_path).resolve()
                if not path_is_scannable(path, roots, include_catalog_sources):
                    continue
                yield {
                    "path": str(path),
                    "line_number": line_number,
                    "match": match,
                }
            stderr = process.stderr.read().decode("utf-8", errors="replace")
            returncode = process.wait()
        except BaseException:
            process.terminate()
            process.wait()
            raise
        finally:
            process.stdout.close()
            process.stderr.close()

        if returncode not in (0, 1):
            fail(f"rg reference search failed for {root}:\n{stderr.strip()}")


def matched_names(text: str, pattern: re.Pattern[str]) -> list[str]:
    names = set()
    for match in pattern.finditer(text):
        name = next(
            (
                match.group(group)
                for group in ("direct", "prose", "path_name", "catalog_name", "sibling_name")
                if match.group(group)
            ),
            None,
        )
        if name:
            names.add(name)
    return sorted(names, key=len, reverse=True)


def collect_edges(
    roots: list[Path],
    skills: list[Skill],
    selected: set[str],
    include_self: bool,
    include_snippets: bool,
    include_catalog_sources: bool,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    known_names = {skill.name for skill in skills}
    edges: list[dict[str, Any]] = []
    unresolved: list[dict[str, Any]] = []

    def scan_known_references(search_roots: list[Path], pattern: str) -> None:
        compiled_pattern = re.compile(pattern)
        for data in search_pattern(search_roots, pattern, include_catalog_sources):
            path = data["path"]
            line_number = data["line_number"]
            match_text = data["match"]
            targets = matched_names(match_text, compiled_pattern)
            if not targets:
                continue
            for target in targets:
                source_skill = skill_for_file(path, skills)
                source = source_skill.name if source_skill else None
                if selected and target not in selected and source not in selected:
                    continue
                if source == target and not include_self:
                    continue
                entry = {
                    "type": "dependency" if source else "external-reference",
                    "source": source,
                    "target": target,
                    "path": path,
                    "line": line_number,
                }
                if include_snippets:
                    entry["snippet"] = match_text
                edges.append(entry)

    all_known_pattern = build_known_pattern(list(known_names))
    if not selected:
        if all_known_pattern:
            scan_known_references(roots, all_known_pattern)
    else:
        inbound_pattern = build_known_pattern(list(selected & known_names))
        if inbound_pattern:
            scan_known_references(roots, inbound_pattern)

        outbound_roots = sorted(
            {Path(skill.real_directory) for skill in skills if skill.name in selected},
            key=str,
        )
        if outbound_roots and all_known_pattern:
            scan_known_references(outbound_roots, all_known_pattern)

    unresolved_pattern = (
        UNRESOLVED_TOKEN_RE.pattern if not selected else build_unresolved_pattern(selected - known_names)
    )
    if unresolved_pattern:
        for data in search_pattern(roots, unresolved_pattern, include_catalog_sources):
            path = data["path"]
            match_text = data["match"]
            line_number = data["line_number"]
            for match in UNRESOLVED_TOKEN_RE.finditer(match_text):
                name = match.group("dollar") or match.group("slash")
                if name in known_names:
                    continue
                if selected and name not in selected:
                    continue
                source_skill = skill_for_file(path, skills)
                entry = {
                    "type": "unresolved-like-reference",
                    "source": source_skill.name if source_skill else None,
                    "target": name,
                    "path": path,
                    "line": line_number,
                }
                if include_snippets:
                    entry["snippet"] = match_text
                unresolved.append(entry)

    unique_edges = unique_by(edges, ("type", "source", "target", "path", "line"))
    unique_unresolved = unique_by(unresolved, ("type", "source", "target", "path", "line"))
    return unique_edges, unique_unresolved


def unique_by(items: list[dict[str, Any]], keys: tuple[str, ...]) -> list[dict[str, Any]]:
    seen: set[tuple[Any, ...]] = set()
    unique = []
    for item in items:
        marker = tuple(item.get(key) for key in keys)
        if marker in seen:
            continue
        seen.add(marker)
        unique.append(item)
    return sorted(unique, key=lambda item: (item.get("target") or "", item.get("source") or "", item["path"], item["line"]))


def duplicate_installs(skills: list[Skill], selected: set[str]) -> list[dict[str, Any]]:
    by_name: dict[str, list[Skill]] = defaultdict(list)
    for skill in skills:
        by_name[skill.name].append(skill)
    duplicates = []
    for name, entries in sorted(by_name.items()):
        realpaths = sorted({entry.realpath for entry in entries})
        if len(realpaths) <= 1:
            continue
        if selected and name not in selected:
            continue
        duplicates.append(
            {
                "type": "duplicate-install",
                "name": name,
                "paths": [entry.path for entry in entries],
            }
        )
    return duplicates


def filter_skills(skills: list[Skill], selected: set[str]) -> list[Skill]:
    if not selected:
        return skills
    return [skill for skill in skills if skill.name in selected]


def skipped_summary() -> dict[str, list[str]]:
    return {
        "directories": [f"**/{name}/**" for name in DEFAULT_DIR_IGNORES] + list(AGENT_STATE_DIR_GLOBS),
        "files": list(AGENT_STATE_FILE_GLOBS),
        "macos_protected_home_paths": list(MACOS_PROTECTED_HOME_PATHS),
        "always_ignored_home_paths": list(ALWAYS_IGNORED_HOME_PATHS),
        "broad_scan_cache_paths": list(BROAD_SCAN_CACHE_PATHS),
        "catalog_sources": list(CATALOG_SOURCE_PATHS),
    }


def as_json(
    roots: list[Path],
    skills: list[Skill],
    edges: list[dict[str, Any]],
    unresolved: list[dict[str, Any]],
    duplicates: list[dict[str, Any]],
    show_skipped: bool,
) -> None:
    payload: dict[str, Any] = {
        "roots": [str(root) for root in roots],
        "skills": [skill.__dict__ for skill in skills],
        "edges": edges,
        "duplicates": duplicates,
        "unresolved": unresolved,
        "counts": {
            "skills": len(skills),
            "edges": len(edges),
            "duplicates": len(duplicates),
            "unresolved": len(unresolved),
        },
    }
    if show_skipped:
        payload["skipped"] = skipped_summary()
    print(json.dumps(payload, indent=2, sort_keys=True))


def as_dot(edges: list[dict[str, Any]], duplicates: list[dict[str, Any]]) -> None:
    print("digraph skill_map {")
    print('  rankdir="LR";')
    seen_pairs: set[tuple[str, str]] = set()
    for edge in edges:
        if edge["type"] != "dependency" or not edge.get("source"):
            continue
        pair = (edge["source"], edge["target"])
        if pair in seen_pairs:
            continue
        seen_pairs.add(pair)
        print(f'  "{edge["source"]}" -> "{edge["target"]}";')
    for duplicate in duplicates:
        print(f'  "{duplicate["name"]}" [shape=box, style="dashed"];')
    print("}")


def as_text(
    roots: list[Path],
    skills: list[Skill],
    edges: list[dict[str, Any]],
    unresolved: list[dict[str, Any]],
    duplicates: list[dict[str, Any]],
    show_skipped: bool,
) -> None:
    print("Skill Map")
    print(f"Roots: {', '.join(str(root) for root in roots)}")
    print(f"Skills: {len(skills)}")

    if duplicates:
        print("\nDuplicate installs:")
        for duplicate in duplicates:
            print(f"- {duplicate['name']}")
            for path in duplicate["paths"]:
                print(f"  {path}")

    dependency_edges = [edge for edge in edges if edge["type"] == "dependency"]
    external_edges = [edge for edge in edges if edge["type"] == "external-reference"]

    if dependency_edges:
        print("\nDependencies:")
        for edge in dependency_edges:
            print(f"- {edge['source']} -> {edge['target']} ({edge['path']}:{edge['line']})")
            if "snippet" in edge:
                print(f"  {edge['snippet']}")

    if external_edges:
        print("\nExternal references:")
        for edge in external_edges:
            print(f"- {edge['target']} ({edge['path']}:{edge['line']})")
            if "snippet" in edge:
                print(f"  {edge['snippet']}")

    if unresolved:
        print("\nUnresolved skill-like references:")
        for edge in unresolved:
            source = f"{edge['source']} -> " if edge.get("source") else ""
            print(f"- {source}{edge['target']} ({edge['path']}:{edge['line']})")
            if "snippet" in edge:
                print(f"  {edge['snippet']}")

    if not duplicates and not edges and not unresolved:
        print("\nNo cross-references found.")

    if show_skipped:
        print("\nIgnored globs:")
        for pattern in skipped_summary()["directories"] + skipped_summary()["files"]:
            print(f"- {pattern}")
        print("\nIgnored catalog source roots during broad scans:")
        for pattern in skipped_summary()["catalog_sources"]:
            print(f"- {pattern}")
        print("\nIgnored macOS protected home paths:")
        for pattern in skipped_summary()["macos_protected_home_paths"]:
            print(f"- {pattern}")
        print("\nAlways-ignored agent home paths during broad scans:")
        for pattern in skipped_summary()["always_ignored_home_paths"]:
            print(f"- {pattern}")
        print("\nIgnored dependency and package cache paths during broad scans:")
        for pattern in skipped_summary()["broad_scan_cache_paths"]:
            print(f"- {pattern}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", action="append", default=[], help="root to scan; repeatable (default: ~)")
    parser.add_argument("--skill", action="append", default=[], help="skill name filter; repeatable")
    parser.add_argument("--format", choices=("text", "json", "dot"), default="text")
    parser.add_argument(
        "--include-catalog-sources",
        action="store_true",
        help="include known local skill catalog source checkouts in broad scans",
    )
    parser.add_argument("--include-self", action="store_true", help="include references from a skill to itself")
    parser.add_argument("--include-snippets", action="store_true", help="include matching line text")
    parser.add_argument("--show-skipped", action="store_true", help="show ignored glob summary")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    roots = normalize_roots(args.root or [str(Path.home())])
    selected = set(args.skill)
    invalid = sorted(name for name in selected if not SKILL_NAME_RE.match(name))
    if invalid:
        fail(f"invalid skill name filter: {', '.join(invalid)}")

    discovered = discover_skills(roots, args.include_catalog_sources)
    known_counts = Counter(skill.name for skill in discovered)
    missing_filters = sorted(name for name in selected if known_counts[name] == 0)
    if missing_filters:
        print(f"skill-map: warning: no discovered skill named {', '.join(missing_filters)}", file=sys.stderr)

    edges, unresolved = collect_edges(
        roots,
        discovered,
        selected,
        args.include_self,
        args.include_snippets,
        args.include_catalog_sources,
    )
    duplicates = duplicate_installs(discovered, selected)
    displayed_skills = filter_skills(discovered, selected)

    if args.format == "json":
        as_json(roots, displayed_skills, edges, unresolved, duplicates, args.show_skipped)
    elif args.format == "dot":
        as_dot(edges, duplicates)
    else:
        as_text(roots, displayed_skills, edges, unresolved, duplicates, args.show_skipped)


if __name__ == "__main__":
    main()
