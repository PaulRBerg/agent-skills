#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Report source files over 1000 LOC for large-file refactor planning.

Usage:
  uv run scripts/large-file-refactor.py [path] [--include-generated]
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


THRESHOLD = 1000
TEST_THRESHOLD = 2000
PLAN_LIMIT = 3

TEST_DIR_NAMES = {
    "test",
    "tests",
    "__tests__",
    "spec",
    "specs",
}

TEST_NAME_TOKENS = {"test", "tests", "spec", "specs"}

_TOKEN_RE = re.compile(r"[A-Z]?[a-z0-9]+|[A-Z]+(?![a-z])")

ALWAYS_IGNORED_DIRS = {
    ".git",
    ".hg",
    ".svn",
    ".jj",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".cache",
}

SOURCE_ONLY_IGNORED_DIRS = {
    ".gradle",
    ".next",
    ".nuxt",
    ".parcel-cache",
    ".serverless",
    ".tox",
    ".turbo",
    ".venv",
    "bower_components",
    "build",
    "coverage",
    "dist",
    "env",
    "external",
    "generated",
    "gen",
    "node_modules",
    "out",
    "target",
    "third_party",
    "tmp",
    "vendor",
    "vendors",
    "venv",
}

LOCK_FILES = {
    "bun.lock",
    "bun.lockb",
    "cargo.lock",
    "composer.lock",
    "go.sum",
    "package-lock.json",
    "pnpm-lock.yaml",
    "poetry.lock",
    "uv.lock",
    "yarn.lock",
}

GENERATED_MARKERS = (
    ".generated.",
    ".gen.",
    ".min.",
    ".pb.",
    ".schema.",
    "_generated.",
    "_pb2.",
    ".designer.",
)

LANGUAGE_BY_EXTENSION = {
    ".bash": "Shell",
    ".astro": "Astro",
    ".asm": "Assembly",
    ".c": "C",
    ".cjs": "JavaScript",
    ".cc": "C++",
    ".cpp": "C++",
    ".css": "CSS",
    ".gql": "GraphQL",
    ".graphql": "GraphQL",
    ".go": "Go",
    ".h": "C/C++ Header",
    ".hpp": "C++ Header",
    ".js": "JavaScript",
    ".jsx": "JavaScript JSX",
    ".less": "Less",
    ".mjs": "JavaScript",
    ".mts": "TypeScript",
    ".nu": "Nushell",
    ".py": "Python",
    ".pyi": "Python Interface",
    ".rs": "Rust",
    ".s": "Assembly",
    ".sass": "Sass",
    ".scss": "SCSS",
    ".sh": "Shell",
    ".sol": "Solidity",
    ".sql": "SQL",
    ".swift": "Swift",
    ".tsx": "TypeScript TSX",
    ".ts": "TypeScript",
    ".vue": "Vue",
    ".zsh": "Zsh",
}

LANGUAGE_BY_FILENAME = {
    "justfile": "Just",
    "makefile": "Makefile",
}

TOKEI_LANGUAGE_ALLOWLIST = {
    "Astro",
    "Assembly",
    "BASH",
    "C",
    "C Header",
    "C++",
    "CSS",
    "Go",
    "GraphQL",
    "JSX",
    "JavaScript",
    "Just",
    "Less",
    "Makefile",
    "Nushell",
    "Python",
    "Rust",
    "SCSS",
    "SQL",
    "Sass",
    "Shell",
    "Solidity",
    "Swift",
    "TSX",
    "TypeScript",
    "Vue",
}

HASH_COMMENT_LANGUAGES = {
    "GraphQL",
    "Just",
    "Makefile",
    "Nushell",
    "Python",
    "Python Interface",
    "Shell",
    "Zsh",
}

DASH_COMMENT_LANGUAGES = {"SQL"}
SLASH_COMMENT_LANGUAGES = {
    "C",
    "C++",
    "C++ Header",
    "C/C++ Header",
    "CSS",
    "Go",
    "JavaScript",
    "JavaScript JSX",
    "Kotlin",
    "Kotlin Script",
    "Less",
    "Rust",
    "SCSS",
    "Solidity",
    "Swift",
    "TypeScript",
    "TypeScript TSX",
}


@dataclass(frozen=True)
class FileStat:
    path: Path
    loc: int
    language: str


def fail(message: str, code: int = 2) -> None:
    print(f"large-file-refactor: {message}", file=sys.stderr)
    raise SystemExit(code)


def expand_argv(argv: list[str]) -> list[str]:
    if len(argv) != 1:
        return argv
    raw = argv[0]
    if Path(os.path.expanduser(raw)).exists():
        return argv
    if raw.startswith("-") or " --" in raw:
        return shlex.split(raw)
    return argv


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", nargs="?", default=".", help="file or directory to scan")
    parser.add_argument(
        "--include-generated",
        action="store_true",
        help="include generated, vendored, dependency, and build-output paths",
    )
    return parser.parse_args(expand_argv(argv))


def resolve_scan_path(raw_path: str) -> Path:
    path = Path(os.path.expanduser(raw_path)).resolve()
    if not path.exists():
        fail(f"path does not exist: {path}")
    if not path.is_dir() and not path.is_file():
        fail(f"path is not a file or directory: {path}")
    return path


def git_root(path: Path) -> Path | None:
    cwd = path if path.is_dir() else path.parent
    result = subprocess.run(
        ["git", "-C", str(cwd), "rev-parse", "--show-toplevel"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode != 0:
        return None
    return Path(result.stdout.strip()).resolve()


def display_base(path: Path) -> Path:
    root = git_root(path)
    if root is not None:
        return root
    return path if path.is_dir() else path.parent


def normalize_part(part: str) -> str:
    return part.lower()


def ignored_dir_names(include_generated: bool) -> set[str]:
    names = set(ALWAYS_IGNORED_DIRS)
    if not include_generated:
        names.update(SOURCE_ONLY_IGNORED_DIRS)
    return names


def is_ignored_path(path: Path, include_generated: bool) -> bool:
    parts = {normalize_part(part) for part in path.parts}
    if parts & ignored_dir_names(include_generated):
        return True
    name = path.name.lower()
    if not include_generated:
        if name in LOCK_FILES:
            return True
        if any(marker in name for marker in GENERATED_MARKERS):
            return True
    return False


def fallback_language(path: Path) -> str | None:
    filename_language = LANGUAGE_BY_FILENAME.get(path.name.lower())
    if filename_language is not None:
        return filename_language
    return LANGUAGE_BY_EXTENSION.get(path.suffix.lower())


def is_refactorable_language(language: str) -> bool:
    return language in TOKEI_LANGUAGE_ALLOWLIST


def tokenize_name(name: str) -> list[str]:
    parts = re.split(r"[^A-Za-z0-9]+", name)
    tokens: list[str] = []
    for part in parts:
        if part:
            tokens.extend(_TOKEN_RE.findall(part))
    return [token.lower() for token in tokens]


def is_test_path(path: Path) -> bool:
    for part in path.parts[:-1]:
        if normalize_part(part) in TEST_DIR_NAMES:
            return True
    return bool(TEST_NAME_TOKENS & set(tokenize_name(path.name)))


def threshold_for(path: Path) -> int:
    return TEST_THRESHOLD if is_test_path(path) else THRESHOLD


def run_tokei(path: Path, include_generated: bool) -> list[FileStat] | None:
    if shutil.which("tokei") is None:
        return None
    args = ["tokei", "--output", "json"]
    if include_generated:
        args.append("--no-ignore")
    for name in sorted(ignored_dir_names(include_generated)):
        args.extend(["--exclude", name])
    args.append(str(path))
    result = subprocess.run(
        args,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        return None
    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        return None

    stats: list[FileStat] = []
    for language, payload in data.items():
        if language == "Total" or not isinstance(payload, dict):
            continue
        if not is_refactorable_language(language):
            continue
        files = payload.get("files") or payload.get("reports")
        if isinstance(files, dict):
            for raw_file, file_payload in files.items():
                append_tokei_file(stats, path, Path(raw_file), file_payload, language, include_generated)
        elif isinstance(files, list):
            for file_payload in files:
                if not isinstance(file_payload, dict):
                    continue
                raw_file = file_payload.get("name") or file_payload.get("path")
                if isinstance(raw_file, str):
                    append_tokei_file(stats, path, Path(raw_file), file_payload, language, include_generated)
    return stats


def append_tokei_file(
    stats: list[FileStat],
    scan_path: Path,
    raw_file: Path,
    payload: Any,
    language: str,
    include_generated: bool,
) -> None:
    if not isinstance(payload, dict):
        return
    code = payload.get("code")
    if not isinstance(code, int):
        stats_payload = payload.get("stats")
        if isinstance(stats_payload, dict):
            code = stats_payload.get("code")
    if not isinstance(code, int):
        return
    scan_base = scan_path if scan_path.is_dir() else scan_path.parent
    path = raw_file if raw_file.is_absolute() else scan_base / raw_file
    path = path.resolve()
    if is_ignored_path(path, include_generated):
        return
    stats.append(FileStat(path=path, loc=code, language=language))


def rg_files(root: Path, include_generated: bool) -> list[Path] | None:
    if shutil.which("rg") is None:
        return None
    args = ["rg", "--files", "--hidden"]
    if include_generated:
        args.append("--no-ignore")
    for name in sorted(ignored_dir_names(include_generated)):
        args.extend(["-g", f"!**/{name}/**"])
    result = subprocess.run(
        args,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode not in (0, 1):
        return None
    return [(root / line).resolve() for line in result.stdout.splitlines() if line]


def walk_files(root: Path, include_generated: bool) -> list[Path]:
    files: list[Path] = []
    ignored_names = ignored_dir_names(include_generated)
    for current, dirs, filenames in os.walk(root):
        current_path = Path(current)
        dirs[:] = [name for name in dirs if normalize_part(name) not in ignored_names]
        for filename in filenames:
            files.append((current_path / filename).resolve())
    return files


def candidate_files(path: Path, include_generated: bool) -> list[Path]:
    if path.is_file():
        return [path]
    return rg_files(path, include_generated) or walk_files(path, include_generated)


def fallback_stats(path: Path, include_generated: bool) -> list[FileStat]:
    stats: list[FileStat] = []
    for file_path in candidate_files(path, include_generated):
        if is_ignored_path(file_path, include_generated):
            continue
        language = fallback_language(file_path)
        if language is None:
            continue
        loc = count_source_lines(file_path, language)
        stats.append(FileStat(path=file_path, loc=loc, language=language))
    return stats


def count_source_lines(path: Path, language: str) -> int:
    try:
        lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
    except OSError:
        return 0

    count = 0
    in_block_comment = False
    supports_slash_comments = language in SLASH_COMMENT_LANGUAGES

    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        if supports_slash_comments:
            stripped, in_block_comment = strip_slash_block_comment(stripped, in_block_comment)
            if not stripped:
                continue
            if stripped.startswith("//"):
                continue
        if language in HASH_COMMENT_LANGUAGES and stripped.startswith("#"):
            continue
        if language in DASH_COMMENT_LANGUAGES and stripped.startswith("--"):
            continue
        count += 1
    return count


def strip_slash_block_comment(line: str, in_block_comment: bool) -> tuple[str, bool]:
    remainder = line
    output = ""
    while remainder:
        if in_block_comment:
            end = remainder.find("*/")
            if end == -1:
                return output.strip(), True
            remainder = remainder[end + 2 :]
            in_block_comment = False
            continue
        start = remainder.find("/*")
        if start == -1:
            output += remainder
            break
        output += remainder[:start]
        remainder = remainder[start + 2 :]
        in_block_comment = True
    return output.strip(), in_block_comment


def large_files(stats: list[FileStat]) -> list[FileStat]:
    by_path: dict[Path, FileStat] = {}
    for stat in stats:
        if stat.loc <= threshold_for(stat.path):
            continue
        current = by_path.get(stat.path)
        if current is None or stat.loc > current.loc:
            by_path[stat.path] = stat
    return sorted(by_path.values(), key=lambda item: (-item.loc, item.path.as_posix()))


def display_path(path: Path, base: Path) -> str:
    try:
        return path.relative_to(base).as_posix()
    except ValueError:
        return path.as_posix()


def md_escape(text: str) -> str:
    return text.replace("|", "\\|")


def print_report(scan_path: Path, files: list[FileStat], metric: str, include_generated: bool) -> None:
    base = display_base(scan_path)
    print("# Large File Report")
    print()
    print(f"- Scan root: `{display_path(scan_path, base)}`")
    print(f"- Threshold: files over {THRESHOLD} LOC (test files over {TEST_THRESHOLD} LOC)")
    print(f"- Metric: {metric}")
    print(f"- Noise policy: {'include generated/vendor/build/dependency paths' if include_generated else 'source-only default'}")
    print()

    if not files:
        print("No files over the threshold found.")
        return

    print("| Rank | LOC | Language | Path |")
    print("| ---: | ---: | --- | --- |")
    for index, stat in enumerate(files, start=1):
        print(
            f"| {index} | {stat.loc} | {md_escape(stat.language)} | "
            f"`{md_escape(display_path(stat.path, base))}` |"
        )

    print()
    print("## Largest Candidates to Inspect")
    print()
    for index, stat in enumerate(files[:PLAN_LIMIT], start=1):
        print(f"{index}. `{display_path(stat.path, base)}` ({stat.loc} LOC)")


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    scan_path = resolve_scan_path(args.path)

    stats = run_tokei(scan_path, args.include_generated)
    if stats is None:
        stats = fallback_stats(scan_path, args.include_generated)
        metric = "portable source-line estimate; install tokei for language-aware counts"
    else:
        metric = "language-aware LOC via tokei"

    print_report(scan_path, large_files(stats), metric, args.include_generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
